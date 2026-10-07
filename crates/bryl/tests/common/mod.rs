//! Shared test fixtures. Stage 2 replaces these hand-written `Record` impls
//! with `#[derive(Record)]`.

#![allow(dead_code)]

use bryl::{Const, EncodeCx, Error, FieldSpec, FieldValue, Record, Sanitize, Token, codec};
use chrono::NaiveDate;

/// Port of `SampleRecord` from `test_bryl.py`:
/// `alpha = Alphanumeric(10)`, `num = Numeric(5)`, `filler = Alphanumeric(5).reserved()`.
#[derive(Debug, Clone, PartialEq)]
pub struct SampleRecord {
    pub alpha: String,
    pub num: u32,
    pub filler: Const,
}

impl Record for SampleRecord {
    const NAME: &'static str = "SampleRecord";
    const LENGTH: usize = 20;
    const FIELDS: &'static [FieldSpec] = &[
        FieldSpec::alpha("alpha", 0, 10),
        FieldSpec::numeric("num", 10, 5),
        FieldSpec::alpha("filler", 15, 5).reserved(),
    ];

    fn encode_into(&self, cx: &EncodeCx, out: &mut Vec<u8>) -> Result<(), Error> {
        codec::encode_field::<Self, _>(&Self::FIELDS[0], &self.alpha, cx, out)?;
        codec::encode_field::<Self, _>(&Self::FIELDS[1], &self.num, cx, out)?;
        codec::encode_field::<Self, _>(&Self::FIELDS[2], &self.filler, cx, out)
    }

    fn decode_fields(raw: &[u8]) -> Result<Self, Error> {
        Ok(Self {
            alpha: codec::decode_field::<Self, _>(&Self::FIELDS[0], raw)?,
            num: codec::decode_field::<Self, _>(&Self::FIELDS[1], raw)?,
            filler: codec::decode_field::<Self, _>(&Self::FIELDS[2], raw)?,
        })
    }
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
#[derive(Debug, Clone, PartialEq)]
pub struct TaggedRecord {
    pub tag: Const,
    pub name: String,
    pub raw_name: String,
    pub closed: Option<NaiveDate>,
}

impl Record for TaggedRecord {
    const NAME: &'static str = "TaggedRecord";
    const LENGTH: usize = 2 + 6 + 6 + 8;
    const FIELDS: &'static [FieldSpec] = &[
        FieldSpec::alpha("tag", 0, 2).with_constant_str("K1"),
        FieldSpec::alpha("name", 2, 6),
        FieldSpec::alpha("raw_name", 8, 6).with_sanitize(Sanitize::NONE),
        FieldSpec::date("closed", 14, MMDDYYYY),
    ];
    const SANITIZE: Sanitize = Sanitize::UPPER;

    fn encode_into(&self, cx: &EncodeCx, out: &mut Vec<u8>) -> Result<(), Error> {
        codec::encode_field::<Self, _>(&Self::FIELDS[0], &self.tag, cx, out)?;
        codec::encode_field::<Self, _>(&Self::FIELDS[1], &self.name, cx, out)?;
        codec::encode_field::<Self, _>(&Self::FIELDS[2], &self.raw_name, cx, out)?;
        codec::encode_field::<Self, _>(&Self::FIELDS[3], &self.closed, cx, out)
    }

    fn decode_fields(raw: &[u8]) -> Result<Self, Error> {
        Ok(Self {
            tag: codec::decode_field::<Self, _>(&Self::FIELDS[0], raw)?,
            name: codec::decode_field::<Self, _>(&Self::FIELDS[1], raw)?,
            raw_name: codec::decode_field::<Self, _>(&Self::FIELDS[2], raw)?,
            closed: codec::decode_field::<Self, _>(&Self::FIELDS[3], raw)?,
        })
    }
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
