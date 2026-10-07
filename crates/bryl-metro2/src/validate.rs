//! Metro 2 field rules, ported from `metro2.py`'s validation helpers.

use chrono::NaiveDate;
use thiserror::Error;

use crate::codes::{AccountStatus, PaymentRating};
use crate::profile::PaymentHistoryError;
use crate::records::BaseSegment;

/// A rule broken by a base segment.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum Violation {
    /// The payment rating does not match the account status.
    #[error("Account status '{status}' requires payment rating '{expected}', got '{found}'")]
    PaymentRating {
        /// Account status.
        status: AccountStatus,
        /// Rating the status requires.
        expected: PaymentRating,
        /// Rating reported (blank if none).
        found: String,
    },
    /// Amount past due must be 0 for current and paid accounts.
    #[error("Amount past due must be 0 for status '{status}', got {amount}")]
    AmountPastDue {
        /// Account status.
        status: AccountStatus,
        /// Amount reported.
        amount: u32,
    },
    /// The payment history profile breaks a rule.
    #[error("payment history profile: {0}")]
    PaymentHistory(PaymentHistoryError),
}

/// True if the SSN is reported: not all zeros (unknown) and not all nines.
pub const fn is_valid_ssn(ssn: u32) -> bool {
    0 < ssn && ssn < 999_999_999
}

/// True if a telephone number is reported (not zero).
pub const fn is_valid_phone(phone: u64) -> bool {
    phone > 0
}

/// True if a date of birth is reported (not zero-filled).
pub const fn is_valid_dob(dob: Option<NaiveDate>) -> bool {
    dob.is_some()
}

impl AccountStatus {
    /// The payment rating this status requires, if any.
    ///
    /// This is `metro2.py`'s `PAYMENT_RATING_FOR_STATUS` table. ⚠ It has not
    /// been checked against the Credit Reporting Resource Guide (CRRG); see
    /// DEVIATIONS.md (M8).
    pub const fn required_payment_rating(self) -> Option<PaymentRating> {
        match self {
            Self::Current | Self::PaidOrClosed => Some(PaymentRating::Current),
            Self::Dpd30 => Some(PaymentRating::Past30),
            Self::Dpd60 => Some(PaymentRating::Past60),
            Self::Dpd90 => Some(PaymentRating::Past90),
            Self::Dpd120 => Some(PaymentRating::Past120),
            Self::Dpd150 => Some(PaymentRating::Past150),
            Self::Dpd180 => Some(PaymentRating::Past180),
            Self::Collections => Some(PaymentRating::Collection),
            Self::ChargeOff => Some(PaymentRating::ChargeOff),
            _ => None,
        }
    }
}

/// Checks that the payment rating matches the account status (see
/// [`AccountStatus::required_payment_rating`]).
///
/// # Errors
///
/// Returns [`Violation::PaymentRating`] on a mismatch.
pub fn validate_payment_rating(
    status: AccountStatus,
    rating: Option<PaymentRating>,
) -> Result<(), Violation> {
    match status.required_payment_rating() {
        Some(expected) if rating != Some(expected) => Err(Violation::PaymentRating {
            status,
            expected,
            found: rating.map_or_else(|| " ".to_owned(), |r| r.to_string()),
        }),
        _ => Ok(()),
    }
}

/// Checks that the amount past due is 0 for status 11 (current) and 13
/// (paid or closed).
///
/// # Errors
///
/// Returns [`Violation::AmountPastDue`] otherwise.
pub fn validate_amount_past_due(status: AccountStatus, amount: u32) -> Result<(), Violation> {
    if matches!(status, AccountStatus::Current | AccountStatus::PaidOrClosed) && amount != 0 {
        return Err(Violation::AmountPastDue { status, amount });
    }
    Ok(())
}

/// Checks a payment history profile: every character is a payment history
/// code or a space, and once `B` (no prior history) appears only `B` and
/// spaces may follow.
///
/// # Errors
///
/// Returns the first problem found.
pub fn validate_payment_history(profile: &str) -> Result<(), PaymentHistoryError> {
    crate::profile::PaymentHistoryProfile::new(profile)?;
    let mut b_seen = false;
    for (position, code) in profile.chars().enumerate() {
        if code == ' ' {
            continue;
        }
        if b_seen && code != 'B' {
            return Err(PaymentHistoryError::EmbeddedB { code, position });
        }
        b_seen |= code == 'B';
    }
    Ok(())
}

impl BaseSegment {
    /// Checks the cross-field rules: payment rating, amount past due and
    /// payment history profile. Returns every violation.
    ///
    /// # Errors
    ///
    /// Returns the violations, if any.
    pub fn validate(&self) -> Result<(), Vec<Violation>> {
        let violations: Vec<Violation> = [
            validate_payment_rating(self.account_status, self.payment_rating).err(),
            validate_amount_past_due(self.account_status, self.amount_past_due).err(),
            validate_payment_history(self.payment_history_profile.as_str())
                .err()
                .map(Violation::PaymentHistory),
        ]
        .into_iter()
        .flatten()
        .collect();
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
}
