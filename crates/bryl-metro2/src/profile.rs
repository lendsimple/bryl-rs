//! The 24-month payment history profile.

use std::fmt;
use std::str::FromStr;

use bryl::codec::{decode_alpha, encode_alpha};
use bryl::kind::AlphaField;
use bryl::{FieldErrorKind, FieldSpec, FieldValue, Sanitize};
use thiserror::Error;

use crate::codes::PaymentHistoryCode;

/// Months in a payment history profile.
pub const PROFILE_MONTHS: usize = 24;

/// Payment history for up to 24 months, most recent first. Each month is a
/// [`PaymentHistoryCode`] or a space (no history reported).
///
/// The constructor checks the characters; the rule that `B` (no prior
/// history) may only be followed by `B` or spaces is checked by
/// [`crate::validate_payment_history`] and [`crate::BaseSegment::validate`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct PaymentHistoryProfile(String);

/// A payment history profile that cannot be represented or breaks a rule.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum PaymentHistoryError {
    /// More than 24 months.
    #[error("payment history profile has {0} months, at most 24 allowed")]
    TooLong(usize),
    /// A character that is not a payment history code or a space.
    #[error("Invalid payment history code {code:?} at position {position}")]
    InvalidCode {
        /// The character.
        code: char,
        /// 0-based position.
        position: usize,
    },
    /// A code other than `B` after a `B`.
    #[error("Code 'B' cannot be embedded: non-B code {code:?} at position {position} follows 'B'")]
    EmbeddedB {
        /// The code after the `B`.
        code: char,
        /// 0-based position.
        position: usize,
    },
}

impl PaymentHistoryProfile {
    /// Checks the length and characters of `profile`.
    ///
    /// # Errors
    ///
    /// Returns an error for more than 24 characters or an unknown code.
    pub fn new(profile: &str) -> Result<Self, PaymentHistoryError> {
        let months = profile.chars().count();
        if months > PROFILE_MONTHS {
            return Err(PaymentHistoryError::TooLong(months));
        }
        if let Some((position, code)) = profile.chars().enumerate().find(|&(_, c)| {
            c != ' ' && PaymentHistoryCode::from_code(c.encode_utf8(&mut [0; 4])).is_none()
        }) {
            return Err(PaymentHistoryError::InvalidCode { code, position });
        }
        Ok(Self(profile.trim_end().to_owned()))
    }

    /// The profile text, without trailing spaces.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Each reported month: `None` for a space.
    pub fn months(&self) -> impl Iterator<Item = Option<PaymentHistoryCode>> + '_ {
        self.0
            .chars()
            .map(|c| PaymentHistoryCode::from_code(c.encode_utf8(&mut [0; 4])))
    }
}

impl fmt::Display for PaymentHistoryProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for PaymentHistoryProfile {
    type Err = PaymentHistoryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl FieldValue for PaymentHistoryProfile {
    fn encode(
        &self,
        spec: &FieldSpec,
        _sanitize: Sanitize,
        out: &mut Vec<u8>,
    ) -> Result<(), FieldErrorKind> {
        encode_alpha(spec, &self.0, out)
    }

    fn decode(spec: &FieldSpec, raw: &[u8]) -> Result<Self, FieldErrorKind> {
        let text = decode_alpha(spec, raw)?;
        Self::new(&text).map_err(|err| FieldErrorKind::Invalid(err.to_string()))
    }
}

impl AlphaField for PaymentHistoryProfile {}
