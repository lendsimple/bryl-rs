//! Marker traits tying Rust types to the field kinds they can be stored in.
//!
//! `#[derive(Record)]` requires the field type to implement the marker for the
//! field's kind, so declaring `#[bryl(numeric(5))] name: String` fails to
//! compile instead of failing on the first encode. Implement the matching
//! marker for custom [`FieldValue`] types.

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

use crate::{Const, FieldValue};

/// Types that can be stored in `alpha(N)` fields.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be stored in an `alpha` field",
    label = "use `String`, a `#[derive(Code)]` enum with string codes, or an `Option` of one"
)]
pub trait AlphaField: FieldValue {}

/// Types that can be stored in `numeric(N)` fields.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be stored in a `numeric` field",
    label = "use `u8`, `u16`, `u32`, `u64`, a `#[derive(Code)]` enum with integer codes, or an `Option` of one"
)]
pub trait NumericField: FieldValue {}

/// Types that can be stored in `date("...")` fields.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be stored in a `date` field",
    label = "use `chrono::NaiveDate` or `Option<chrono::NaiveDate>`"
)]
pub trait DateField: FieldValue {}

/// Types that can be stored in `time("...")` fields.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be stored in a `time` field",
    label = "use `chrono::NaiveTime` or `Option<chrono::NaiveTime>`"
)]
pub trait TimeField: FieldValue {}

/// Types that can be stored in `datetime("...")` fields.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be stored in a `datetime` field",
    label = "use `chrono::NaiveDateTime` or `Option<chrono::NaiveDateTime>`"
)]
pub trait DateTimeField: FieldValue {}

impl AlphaField for String {}
impl AlphaField for Const {}
impl NumericField for Const {}
impl NumericField for u8 {}
impl NumericField for u16 {}
impl NumericField for u32 {}
impl NumericField for u64 {}
impl DateField for NaiveDate {}
impl TimeField for NaiveTime {}
impl DateTimeField for NaiveDateTime {}

impl<T: AlphaField> AlphaField for Option<T> {}
impl<T: NumericField> NumericField for Option<T> {}
impl<T: DateField> DateField for Option<T> {}
impl<T: TimeField> TimeField for Option<T> {}
impl<T: DateTimeField> DateTimeField for Option<T> {}
