//! Validated field types.

use std::fmt;
use std::str::FromStr;

use bryl::codec::{decode_alpha, decode_numeric, encode_alpha, encode_numeric};
use bryl::kind::{AlphaField, NumericField};
use bryl::{FieldErrorKind, FieldKind, FieldSpec, FieldValue, Sanitize};
use thiserror::Error;

/// A nine-digit ABA routing transit number with a valid check digit.
///
/// The first eight digits are the transit routing number (TRN) that NACHA
/// entry hashes add up; the ninth is the check digit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RoutingNumber(u32);

/// A routing number that is not nine digits or fails the ABA checksum.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RoutingNumberError {
    /// Not exactly nine ASCII digits.
    #[error("routing number {0:?} must be exactly 9 digits")]
    Format(String),
    /// The check digit does not match.
    #[error("routing number {0:09} has an invalid check digit")]
    Checksum(u32),
}

impl RoutingNumber {
    /// Validates a routing number given as an integer (leading zeros implied).
    ///
    /// # Errors
    ///
    /// Returns an error if `value` has more than nine digits or fails the ABA
    /// 3-7-1 checksum.
    pub fn new(value: u32) -> Result<Self, RoutingNumberError> {
        if value > 999_999_999 {
            return Err(RoutingNumberError::Format(value.to_string()));
        }
        if !aba_checksum_ok(value) {
            return Err(RoutingNumberError::Checksum(value));
        }
        Ok(Self(value))
    }

    /// Builds a routing number from a TRN and check digit, as stored in
    /// separate fields by some systems.
    ///
    /// # Errors
    ///
    /// Returns an error if the combination is not a valid routing number.
    pub fn from_parts(trn: u32, check_digit: u8) -> Result<Self, RoutingNumberError> {
        let value = trn
            .checked_mul(10)
            .and_then(|v| v.checked_add(u32::from(check_digit)))
            .filter(|_| check_digit <= 9)
            .ok_or_else(|| RoutingNumberError::Format(format!("{trn}{check_digit}")))?;
        Self::new(value)
    }

    /// The nine-digit value.
    pub const fn get(self) -> u32 {
        self.0
    }

    /// The first eight digits, used for entry hashes.
    pub const fn trn(self) -> u32 {
        self.0 / 10
    }

    /// The ninth digit.
    #[allow(clippy::cast_possible_truncation)] // always 0..=9
    pub const fn check_digit(self) -> u8 {
        (self.0 % 10) as u8
    }
}

/// ABA 3-7-1 checksum: 3(d1+d4+d7) + 7(d2+d5+d8) + (d3+d6+d9) ≡ 0 (mod 10).
fn aba_checksum_ok(value: u32) -> bool {
    const WEIGHTS: [u32; 9] = [3, 7, 1, 3, 7, 1, 3, 7, 1];
    let mut rest = value;
    let mut digits = [0u32; 9];
    for digit in digits.iter_mut().rev() {
        *digit = rest % 10;
        rest /= 10;
    }
    digits.iter().zip(WEIGHTS).map(|(d, w)| d * w).sum::<u32>() % 10 == 0
}

impl fmt::Display for RoutingNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:09}", self.0)
    }
}

impl FromStr for RoutingNumber {
    type Err = RoutingNumberError;

    /// Parses exactly nine ASCII digits, such as `"091000019"`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 9 || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(RoutingNumberError::Format(s.to_owned()));
        }
        Self::new(
            s.parse()
                .map_err(|_| RoutingNumberError::Format(s.to_owned()))?,
        )
    }
}

impl TryFrom<u32> for RoutingNumber {
    type Error = RoutingNumberError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl FieldValue for RoutingNumber {
    fn encode(
        &self,
        spec: &FieldSpec,
        _sanitize: Sanitize,
        out: &mut Vec<u8>,
    ) -> Result<(), FieldErrorKind> {
        // Always nine digits, even with a space-padded ten-character field.
        let mut digits = Vec::with_capacity(9);
        encode_numeric(
            &FieldSpec::numeric(spec.name, 0, 9),
            u64::from(self.0),
            &mut digits,
        )?;
        let text = String::from_utf8(digits).expect("digits are ASCII");
        encode_alpha(
            &FieldSpec {
                kind: FieldKind::Alpha,
                ..*spec
            },
            &text,
            out,
        )
    }

    fn decode(spec: &FieldSpec, raw: &[u8]) -> Result<Self, FieldErrorKind> {
        let value = decode_numeric(spec, raw)?;
        let value = u32::try_from(value).map_err(|_| {
            FieldErrorKind::Invalid(format!("routing number {value} has more than 9 digits"))
        })?;
        Self::new(value).map_err(|err| FieldErrorKind::Invalid(err.to_string()))
    }
}

impl NumericField for RoutingNumber {}

/// File ID modifier: `A`–`Z` or `0`–`9`, distinguishing files created on the
/// same day for the same destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileIdModifier(u8);

/// A file ID modifier that is not an uppercase letter or digit.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("file ID modifier {0:?} must be A-Z or 0-9")]
pub struct FileIdModifierError(pub char);

impl FileIdModifier {
    /// Validates a modifier.
    ///
    /// # Errors
    ///
    /// Returns an error unless `c` is `A`–`Z` or `0`–`9`.
    pub fn new(c: char) -> Result<Self, FileIdModifierError> {
        match u8::try_from(c) {
            Ok(b) if b.is_ascii_uppercase() || b.is_ascii_digit() => Ok(Self(b)),
            _ => Err(FileIdModifierError(c)),
        }
    }

    /// The modifier character.
    pub const fn get(self) -> char {
        self.0 as char
    }
}

/// `A`, the usual modifier for a day's first file.
impl Default for FileIdModifier {
    fn default() -> Self {
        Self(b'A')
    }
}

impl fmt::Display for FileIdModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get())
    }
}

impl TryFrom<char> for FileIdModifier {
    type Error = FileIdModifierError;

    fn try_from(c: char) -> Result<Self, Self::Error> {
        Self::new(c)
    }
}

impl FieldValue for FileIdModifier {
    fn encode(
        &self,
        spec: &FieldSpec,
        _sanitize: Sanitize,
        out: &mut Vec<u8>,
    ) -> Result<(), FieldErrorKind> {
        encode_alpha(spec, &self.get().to_string(), out)
    }

    fn decode(spec: &FieldSpec, raw: &[u8]) -> Result<Self, FieldErrorKind> {
        let text = decode_alpha(spec, raw)?;
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => Self::new(c).map_err(|err| FieldErrorKind::Invalid(err.to_string())),
            _ => Err(FieldErrorKind::Invalid(format!(
                "file ID modifier {text:?} must be one character"
            ))),
        }
    }
}

impl AlphaField for FileIdModifier {}
