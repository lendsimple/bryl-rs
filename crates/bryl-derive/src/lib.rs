//! Derive macros for [`bryl`](https://docs.rs/bryl). Use them through `bryl`,
//! which re-exports them; the generated code refers to `::bryl`.

mod attrs;
mod code;
mod record;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Derives `bryl::Record` for a struct with named fields.
///
/// Fields are laid out in declaration order. Each field takes one kind:
///
/// | Attribute | Field type |
/// |---|---|
/// | `alpha(N)` | `String`, a string `#[derive(Code)]` enum, `bryl::Const`, or `Option` of these |
/// | `numeric(N)` | `u8`–`u64`, an integer `#[derive(Code)]` enum, `bryl::Const`, or `Option` of these |
/// | `date("MMDDYYYY")` | `chrono::NaiveDate` or `Option<NaiveDate>` |
/// | `time("hhmm")` | `chrono::NaiveTime` or `Option<NaiveTime>` |
/// | `datetime("MMDDYYYYhhmmss")` | `chrono::NaiveDateTime` or `Option<NaiveDateTime>` |
/// | `flatten` | another `Record`, spliced in at this position |
///
/// and optionally `pad = ' '`, `align = "left" | "right"`, `min = N`, `max = N`,
/// `constant = "K1" | 426`, `reserved`, `sanitize(upper, filter, truncate)` or
/// `no_sanitize`. Constant and reserved fields have type `bryl::Const`.
///
/// Struct options: `#[bryl(sanitize(upper), length = N)]`. `length` is checked
/// at compile time against the sum of the field lengths.
#[proc_macro_derive(Record, attributes(bryl))]
pub fn derive_record(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    record::expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derives a code table for a fieldless enum whose variants carry
/// `#[code("05")]` (stored in `alpha` fields) or `#[code(220)]` (stored in
/// `numeric` fields).
///
/// Generates `ALL`, `as_code`, `from_code`, `Display`, `FromStr`, `TryFrom`,
/// and `bryl::FieldValue`.
#[proc_macro_derive(Code, attributes(code))]
pub fn derive_code(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    code::expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
