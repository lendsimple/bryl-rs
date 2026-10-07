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
    /// The account status requires a payment rating and none is reported.
    #[error("Account status '{status}' requires a payment rating")]
    PaymentRatingRequired {
        /// Account status.
        status: AccountStatus,
    },
    /// The account status does not allow a payment rating.
    #[error("Payment rating must be blank for account status '{status}', got '{rating}'")]
    PaymentRatingNotAllowed {
        /// Account status.
        status: AccountStatus,
        /// Rating reported.
        rating: PaymentRating,
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
    /// True for the statuses that must report a payment rating: 05, 13, 65,
    /// 88, 89, 94 and 95. These close or transfer the account, and the rating
    /// records its condition just before that. Every other status must leave
    /// the payment rating blank.
    ///
    /// Not yet checked against the Credit Reporting Resource Guide (CRRG)
    /// itself. Three independent sources agree on this list: moov-io/metro2's
    /// validator, Upstart's `metro_2` Ruby gem
    /// (`account_status_needs_payment_rating?`) and The Mortgage Office's
    /// Metro 2 documentation. `metro2.py` instead required a rating per
    /// delinquency status (e.g. `0` for 11); see DEVIATIONS.md (M8).
    pub const fn requires_payment_rating(self) -> bool {
        matches!(
            self,
            Self::Transferred
                | Self::PaidOrClosed
                | Self::PaidForeclosureStarted
                | Self::GovernmentClaimFiled
                | Self::DeedReceived
                | Self::ForeclosureCompleted
                | Self::VoluntarySurrender
        )
    }
}

/// Checks the payment rating against the account status: required (any
/// rating) for the statuses in [`AccountStatus::requires_payment_rating`],
/// blank for all others.
///
/// # Errors
///
/// Returns [`Violation::PaymentRatingRequired`] or
/// [`Violation::PaymentRatingNotAllowed`].
pub fn validate_payment_rating(
    status: AccountStatus,
    rating: Option<PaymentRating>,
) -> Result<(), Violation> {
    match (status.requires_payment_rating(), rating) {
        (true, None) => Err(Violation::PaymentRatingRequired { status }),
        (false, Some(rating)) => Err(Violation::PaymentRatingNotAllowed { status, rating }),
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
