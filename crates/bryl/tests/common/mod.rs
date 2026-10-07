//! Shared test fixtures.

#![allow(dead_code)]

use bryl::{Const, FieldSpec, FieldValue, Record, Sanitize, Token};
use chrono::NaiveDate;

/// A 20-character record: a 10-character alpha field, a 5-digit number and
/// 5 characters of reserved filler.
#[derive(Record, Debug, Clone, PartialEq)]
#[bryl(length = 20)]
pub struct SampleRecord {
    #[bryl(alpha(10))]
    pub alpha: String,
    #[bryl(numeric(5))]
    pub num: u32,
    #[bryl(alpha(5), reserved)]
    pub filler: Const,
}

impl SampleRecord {
    pub fn new(alpha: &str, num: u32) -> Self {
        Self {
            alpha: alpha.to_owned(),
            num,
            filler: Const,
        }
    }
}

pub const MMDDYYYY: &[Token] = &[Token::Month, Token::Day, Token::Year4];

/// An uppercasing record with a constant tag, an optional zero-filled date and
/// a field-level sanitize override, modeled on Metro 2 segments.
#[derive(Record, Debug, Clone, PartialEq)]
#[bryl(sanitize(upper), length = 22)]
pub struct TaggedRecord {
    #[bryl(alpha(2), constant = "K1")]
    pub tag: Const,
    #[bryl(alpha(6))]
    pub name: String,
    #[bryl(alpha(6), no_sanitize)]
    pub raw_name: String,
    #[bryl(date("MMDDYYYY"))]
    pub closed: Option<NaiveDate>,
}

/// Encodes a single field value without sanitizing.
pub fn pack<T: FieldValue>(spec: &FieldSpec, value: &T) -> Result<String, bryl::FieldErrorKind> {
    pack_with(spec, value, Sanitize::NONE)
}

/// Encodes a single field value with the given sanitize settings.
pub fn pack_with<T: FieldValue>(
    spec: &FieldSpec,
    value: &T,
    sanitize: Sanitize,
) -> Result<String, bryl::FieldErrorKind> {
    let mut out = Vec::new();
    value.encode(spec, sanitize, &mut out)?;
    assert_eq!(out.len(), spec.length, "encoded width");
    Ok(String::from_utf8(out).expect("encoded output is ASCII"))
}

/// Decodes a single field value from exactly `spec.length` bytes.
pub fn unpack<T: FieldValue>(spec: &FieldSpec, raw: &str) -> Result<T, bryl::FieldErrorKind> {
    T::decode(spec, raw.as_bytes())
}
