//! Metro 2 field rules, ported from `metro2.py`'s validation helpers.

use chrono::NaiveDate;
use thiserror::Error;

use crate::codes::{AccountStatus, PaymentRating};
use crate::data_record::DataRecord;
use crate::profile::PaymentHistoryError;
use crate::records::{BaseSegment, J1Segment, J2Segment, L1Segment};

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
    /// A field contains a character its kind of data does not allow.
    #[error("{field} contains {ch:?} at position {position}; {allowed}")]
    Characters {
        /// Field name, e.g. `surname` or `j1[0].surname`.
        field: String,
        /// The offending character.
        ch: char,
        /// 0-based position in the value.
        position: usize,
        /// The characters the field allows.
        allowed: CharClass,
    },
    /// The account status is no longer used for reporting: CDIA retired 05
    /// in April 2022 (<https://www.cdiaonline.org/retirementaccountstatus05/>).
    /// Report the status the account had when it was transferred, special
    /// comment AT (transferred within the company) or O (to another company),
    /// and zero balance, amount past due and scheduled payment.
    #[error(
        "Account status '{status}' was retired in April 2022; report the status at the time of \
         transfer with special comment AT or O instead"
    )]
    RetiredStatus {
        /// Account status.
        status: AccountStatus,
    },
}

/// The characters a Metro 2 text field allows, per the field descriptions in
/// CDIA's Credit Reporting Resource Guide (2020 edition). The guide also says
/// alpha fields should be uppercase, which the writer does by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharClass {
    /// Consumer name fields: letters, spaces and hyphens ("other than the
    /// hyphen, do not report special characters").
    Name,
    /// Address lines and city: letters, digits, spaces, slashes, dashes and
    /// periods.
    Address,
    /// Consumer account number and identification number: letters and
    /// digits only ("no embedded blanks or special characters").
    Identifier,
}

impl CharClass {
    /// True if `c` is allowed.
    pub const fn allows(self, c: char) -> bool {
        match self {
            Self::Name => c.is_ascii_alphabetic() || c == ' ' || c == '-',
            Self::Address => c.is_ascii_alphanumeric() || matches!(c, ' ' | '/' | '-' | '.'),
            Self::Identifier => c.is_ascii_alphanumeric(),
        }
    }

    /// Checks that every character of `value` is allowed.
    ///
    /// # Errors
    ///
    /// Returns [`Violation::Characters`] for the first character that is not
    /// allowed, naming `field`.
    pub fn check(self, field: impl Into<String>, value: &str) -> Result<(), Violation> {
        match value.chars().enumerate().find(|&(_, c)| !self.allows(c)) {
            None => Ok(()),
            Some((position, ch)) => Err(Violation::Characters {
                field: field.into(),
                ch,
                position,
                allowed: self,
            }),
        }
    }
}

impl std::fmt::Display for CharClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Name => "only letters, spaces and hyphens are allowed",
            Self::Address => {
                "only letters, digits, spaces, slashes, dashes and periods are allowed"
            }
            Self::Identifier => "only letters and digits are allowed",
        })
    }
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
    /// Metro 2 documentation. A rating per delinquency status (e.g. `0` for
    /// 11) is not allowed.
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
    /// Checks the cross-field rules (payment rating, amount past due,
    /// payment history profile), the retired status 05, and the characters
    /// allowed in names, addresses and account and identification numbers.
    /// Returns every violation.
    ///
    /// # Errors
    ///
    /// Returns the violations, if any.
    pub fn validate(&self) -> Result<(), Vec<Violation>> {
        let mut violations: Vec<Violation> = [
            validate_payment_rating(self.account_status, self.payment_rating).err(),
            validate_amount_past_due(self.account_status, self.amount_past_due).err(),
            validate_payment_history(self.payment_history_profile.as_str())
                .err()
                .map(Violation::PaymentHistory),
            (self.account_status == AccountStatus::Transferred).then_some(
                Violation::RetiredStatus {
                    status: self.account_status,
                },
            ),
        ]
        .into_iter()
        .flatten()
        .collect();
        violations.extend(check_characters(
            "",
            &[
                (
                    "identification_number",
                    &self.identification_number,
                    CharClass::Identifier,
                ),
                (
                    "consumer_account_number",
                    &self.consumer_account_number,
                    CharClass::Identifier,
                ),
                ("surname", &self.surname, CharClass::Name),
                ("first_name", &self.first_name, CharClass::Name),
                ("middle_name", &self.middle_name, CharClass::Name),
                (
                    "first_line_of_address",
                    &self.first_line_of_address,
                    CharClass::Address,
                ),
                (
                    "second_line_of_address",
                    &self.second_line_of_address,
                    CharClass::Address,
                ),
                ("city", &self.city, CharClass::Address),
            ],
        ));
        into_result(violations)
    }
}

impl J1Segment {
    /// Checks the characters in the name fields.
    ///
    /// # Errors
    ///
    /// Returns the violations, if any. Field names are prefixed with
    /// `prefix`, e.g. `j1[0].`.
    pub fn validate(&self, prefix: &str) -> Result<(), Vec<Violation>> {
        into_result(check_characters(
            prefix,
            &[
                ("surname", &self.surname, CharClass::Name),
                ("first_name", &self.first_name, CharClass::Name),
                ("middle_name", &self.middle_name, CharClass::Name),
            ],
        ))
    }
}

impl J2Segment {
    /// Checks the characters in the name and address fields.
    ///
    /// # Errors
    ///
    /// Returns the violations, if any. Field names are prefixed with
    /// `prefix`, e.g. `j2[0].`.
    pub fn validate(&self, prefix: &str) -> Result<(), Vec<Violation>> {
        into_result(check_characters(
            prefix,
            &[
                ("surname", &self.surname, CharClass::Name),
                ("first_name", &self.first_name, CharClass::Name),
                ("middle_name", &self.middle_name, CharClass::Name),
                (
                    "first_line_of_address",
                    &self.first_line_of_address,
                    CharClass::Address,
                ),
                (
                    "second_line_of_address",
                    &self.second_line_of_address,
                    CharClass::Address,
                ),
                ("city", &self.city, CharClass::Address),
            ],
        ))
    }
}

impl L1Segment {
    /// Checks the characters in the new account and identification numbers,
    /// which follow the rules for the fields they replace.
    ///
    /// # Errors
    ///
    /// Returns the violations, if any.
    pub fn validate(&self) -> Result<(), Vec<Violation>> {
        into_result(check_characters(
            "l1.",
            &[
                (
                    "new_consumer_account_number",
                    &self.new_consumer_account_number,
                    CharClass::Identifier,
                ),
                (
                    "new_identification_number",
                    &self.new_identification_number,
                    CharClass::Identifier,
                ),
            ],
        ))
    }
}

impl DataRecord {
    /// Validates the base segment and its J1, J2 and L1 segments. Returns
    /// every violation; segment fields are named like `j1[0].surname`.
    ///
    /// # Errors
    ///
    /// Returns the violations, if any.
    pub fn validate(&self) -> Result<(), Vec<Violation>> {
        let mut violations = self.base.validate().err().unwrap_or_default();
        for (index, j1) in self.j1.iter().enumerate() {
            violations.extend(
                j1.validate(&format!("j1[{index}]."))
                    .err()
                    .unwrap_or_default(),
            );
        }
        for (index, j2) in self.j2.iter().enumerate() {
            violations.extend(
                j2.validate(&format!("j2[{index}]."))
                    .err()
                    .unwrap_or_default(),
            );
        }
        if let Some(l1) = &self.l1 {
            violations.extend(l1.validate().err().unwrap_or_default());
        }
        into_result(violations)
    }
}

fn check_characters(prefix: &str, fields: &[(&str, &String, CharClass)]) -> Vec<Violation> {
    fields
        .iter()
        .filter_map(|(name, value, class)| class.check(format!("{prefix}{name}"), value).err())
        .collect()
}

fn into_result(violations: Vec<Violation>) -> Result<(), Vec<Violation>> {
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}
