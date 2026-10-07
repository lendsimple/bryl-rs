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
//! use bryl::{Code, Const, Record};
//! use chrono::NaiveDate;
//!
//! #[derive(Code, Debug, Clone, Copy, PartialEq)]
//! enum Status {
//!     #[code("11")]
//!     Current,
//!     #[code("97")]
//!     ChargeOff,
//! }
//!
//! #[derive(Record, Debug, Clone, PartialEq)]
//! #[bryl(sanitize(upper), length = 33)]
//! struct Account {
//!     #[bryl(alpha(2), constant = "AC")]
//!     tag: Const,
//!     #[bryl(alpha(10))]
//!     surname: String,
//!     #[bryl(numeric(9))]
//!     balance: u64,
//!     #[bryl(alpha(2))]
//!     status: Status,
//!     #[bryl(date("MMDDYYYY"))]
//!     closed: Option<NaiveDate>,
//!     #[bryl(alpha(2), reserved)]
//!     filler: Const,
//! }
//!
//! let account = Account {
//!     tag: Const,
//!     surname: "smith".into(),
//!     balance: 1250,
//!     status: Status::Current,
//!     closed: None,
//!     filler: Const,
//! };
//! let encoded = account.encode()?;
//! assert_eq!(encoded, "ACSMITH     0000012501100000000  ");
//!
//! let decoded = Account::decode(encoded.as_bytes())?;
//! assert_eq!(decoded.surname, "SMITH");
//! assert_eq!(decoded.status, Status::Current);
//! assert_eq!(Account::field("balance").map(|f| f.offset), Some(12));
//! # Ok::<(), bryl::Error>(())
//! ```
//!
//! Records can also be implemented by hand with [`codec::encode_field`] and
//! [`codec::decode_field`]; see [`Record`].

pub mod codec;
mod error;
mod field;
pub mod kind;
mod record;
mod sanitize;

pub use bryl_pattern::{PatternError, PatternKind, Token, parse_pattern, pattern_width};
pub use codec::FieldValue;
pub use error::{Error, FieldErrorKind};
pub use field::{Align, Const, Constant, FieldKind, FieldSpec};
pub use record::{EncodeCx, Record};
pub use sanitize::Sanitize;

/// Derives a fixed-width code table for a fieldless enum.
pub use bryl_derive::Code;
/// Derives [`Record`] for a struct; see the crate docs for the attributes.
pub use bryl_derive::Record;

/// Error returned when parsing a string or integer that is not in a code table.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown {type_name} code {code:?}")]
pub struct UnknownCode {
    /// The code table type.
    pub type_name: &'static str,
    /// The unrecognized code.
    pub code: String,
}

/// Support code for the derive macros. Not public API.
#[doc(hidden)]
pub mod __private {
    use crate::kind::{AlphaField, DateField, DateTimeField, NumericField, TimeField};

    pub const fn assert_alpha<T: AlphaField>() {}
    pub const fn assert_numeric<T: NumericField>() {}
    pub const fn assert_date<T: DateField>() {}
    pub const fn assert_time<T: TimeField>() {}
    pub const fn assert_datetime<T: DateTimeField>() {}
    pub const fn assert_record<T: crate::Record>() {}
}
