//! Declaratively defined, fixed-width records made of typed, fixed-width fields.
//!
//! A [`Record`] is a struct whose fields each occupy a fixed byte range of the
//! encoded record. Each field is described by a [`FieldSpec`] (name, offset,
//! length, padding, alignment and [`FieldKind`]) and its Rust type implements
//! [`FieldValue`], which knows how to encode and decode that kind.
//!
//! Encoded records are always printable ASCII, so byte offsets equal character
//! offsets.
//!
//! ```
//! use bryl::{Const, EncodeCx, Error, FieldSpec, Record, Sanitize, codec};
//!
//! #[derive(Debug, PartialEq)]
//! struct Greeting {
//!     tag: Const,
//!     name: String,
//!     count: u32,
//! }
//!
//! impl Record for Greeting {
//!     const NAME: &'static str = "Greeting";
//!     const LENGTH: usize = 16;
//!     const FIELDS: &'static [FieldSpec] = &[
//!         FieldSpec::alpha("tag", 0, 1).with_constant_str("G"),
//!         FieldSpec::alpha("name", 1, 10),
//!         FieldSpec::numeric("count", 11, 5),
//!     ];
//!     const SANITIZE: Sanitize = Sanitize::UPPER;
//!
//!     fn encode_into(&self, cx: &EncodeCx, out: &mut Vec<u8>) -> Result<(), Error> {
//!         codec::encode_field::<Self, _>(&Self::FIELDS[0], &self.tag, cx, out)?;
//!         codec::encode_field::<Self, _>(&Self::FIELDS[1], &self.name, cx, out)?;
//!         codec::encode_field::<Self, _>(&Self::FIELDS[2], &self.count, cx, out)
//!     }
//!
//!     fn decode_fields(raw: &[u8]) -> Result<Self, Error> {
//!         Ok(Self {
//!             tag: codec::decode_field::<Self, _>(&Self::FIELDS[0], raw)?,
//!             name: codec::decode_field::<Self, _>(&Self::FIELDS[1], raw)?,
//!             count: codec::decode_field::<Self, _>(&Self::FIELDS[2], raw)?,
//!         })
//!     }
//! }
//!
//! let greeting = Greeting { tag: Const, name: "hello".into(), count: 42 };
//! let encoded = greeting.encode()?;
//! assert_eq!(encoded, "GHELLO     00042");
//! assert_eq!(Greeting::decode(encoded.as_bytes())?.name, "HELLO");
//! # Ok::<(), bryl::Error>(())
//! ```

pub mod codec;
mod error;
mod field;
mod pattern;
mod record;
mod sanitize;

pub use codec::FieldValue;
pub use error::{Error, FieldErrorKind};
pub use field::{Align, Const, Constant, FieldKind, FieldSpec};
pub use pattern::{PatternError, PatternKind, Token, parse_pattern, pattern_width};
pub use record::{EncodeCx, Record};
pub use sanitize::Sanitize;
