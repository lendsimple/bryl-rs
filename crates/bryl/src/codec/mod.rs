//! Encoding and decoding of individual field values.

mod alpha;
mod constant;
mod datetime;
mod numeric;
mod option;

pub use alpha::{decode_alpha, encode_alpha};
pub(crate) use constant::encode_constant;
pub use numeric::{decode_numeric, encode_numeric};

use crate::field::{Align, FieldSpec};
use crate::record::{EncodeCx, Record};
use crate::{Error, FieldErrorKind, Sanitize};

/// A Rust type that can be stored in a fixed-width field.
///
/// Implementations must write exactly `spec.length` bytes on success.
pub trait FieldValue: Sized {
    /// Appends the encoded value to `out`.
    ///
    /// # Errors
    ///
    /// Returns why the value cannot be stored in the field.
    fn encode(
        &self,
        spec: &FieldSpec,
        sanitize: Sanitize,
        out: &mut Vec<u8>,
    ) -> Result<(), FieldErrorKind>;

    /// Decodes a value from `raw`, which is exactly `spec.length` bytes.
    ///
    /// # Errors
    ///
    /// Returns why `raw` is not a valid value for the field.
    fn decode(spec: &FieldSpec, raw: &[u8]) -> Result<Self, FieldErrorKind>;
}

/// Encodes one field of record `R`, attaching record and field context to errors.
///
/// Used by [`Record::encode_into`] implementations.
///
/// # Errors
///
/// Returns [`Error::Field`] if the value cannot be encoded.
pub fn encode_field<R: Record, T: FieldValue>(
    spec: &FieldSpec,
    value: &T,
    cx: &EncodeCx,
    out: &mut Vec<u8>,
) -> Result<(), Error> {
    let sanitize = Sanitize::resolve(spec.sanitize, cx.sanitize, R::SANITIZE);
    let start = out.len();
    let result = value.encode(spec, sanitize, out);
    if result.is_err() {
        out.truncate(start);
    }
    debug_assert!(
        result.is_err() || out.len() - start == spec.length,
        "{}.{} encoded {} bytes, expected {}",
        R::NAME,
        spec.name,
        out.len() - start,
        spec.length,
    );
    result.map_err(|kind| field_error::<R>(spec, kind))
}

/// Decodes one field of record `R` from the whole record `raw`.
///
/// Used by [`Record::decode_fields`] implementations.
///
/// # Errors
///
/// Returns [`Error::Length`] if `raw` does not cover the field, or
/// [`Error::Field`] if its content is invalid.
pub fn decode_field<R: Record, T: FieldValue>(spec: &FieldSpec, raw: &[u8]) -> Result<T, Error> {
    let bytes = raw.get(spec.range()).ok_or(Error::Length {
        record: R::NAME,
        expected: R::LENGTH,
        actual: raw.len(),
    })?;
    T::decode(spec, bytes).map_err(|kind| field_error::<R>(spec, kind))
}

/// Encodes a record embedded in record `R` at `offset` (`#[bryl(flatten)]`).
///
/// The embedded record uses its own sanitize defaults unless `cx` overrides
/// them. Errors are reported against `R`, with offsets relative to `R`.
///
/// # Errors
///
/// Returns the embedded record's first encoding error.
pub fn encode_flattened<R: Record, T: Record>(
    offset: usize,
    value: &T,
    cx: &EncodeCx,
    out: &mut Vec<u8>,
) -> Result<(), Error> {
    let start = out.len();
    value.encode_into(cx, out).map_err(|err| {
        out.truncate(start);
        reparent::<R>(offset, err, None)
    })
}

/// Decodes a record embedded in record `R` at `offset` (`#[bryl(flatten)]`).
///
/// # Errors
///
/// Returns [`Error::Length`] if `raw` does not cover the embedded record, or
/// its first invalid field, reported against `R`.
pub fn decode_flattened<R: Record, T: Record>(offset: usize, raw: &[u8]) -> Result<T, Error> {
    let bytes = raw.get(offset..offset + T::LENGTH).ok_or(Error::Length {
        record: R::NAME,
        expected: R::LENGTH,
        actual: raw.len(),
    })?;
    T::decode_fields(bytes).map_err(|err| reparent::<R>(offset, err, Some(raw.len())))
}

/// Re-attributes an embedded record's error to the outer record `R`.
fn reparent<R: Record>(offset: usize, err: Error, raw_len: Option<usize>) -> Error {
    match err {
        Error::Field {
            field,
            offset: inner,
            kind,
            ..
        } => Error::Field {
            record: R::NAME,
            field,
            offset: offset + inner,
            kind,
        },
        Error::Length { actual, .. } => Error::Length {
            record: R::NAME,
            expected: R::LENGTH,
            actual: raw_len.unwrap_or(actual),
        },
    }
}

fn field_error<R: Record>(spec: &FieldSpec, kind: FieldErrorKind) -> Error {
    Error::Field {
        record: R::NAME,
        field: spec.name,
        offset: spec.offset,
        kind,
    }
}

/// Writes `value` padded to `spec.length` according to `spec.align`.
/// `value` must already be known to fit.
fn pad_into(spec: &FieldSpec, value: &[u8], out: &mut Vec<u8>) {
    let fill = spec.length - value.len();
    match spec.align {
        Align::Left => {
            out.extend_from_slice(value);
            out.resize(out.len() + fill, spec.pad);
        }
        Align::Right => {
            out.resize(out.len() + fill, spec.pad);
            out.extend_from_slice(value);
        }
    }
}

/// Removes padding from the side opposite the alignment.
fn strip<'a>(spec: &FieldSpec, raw: &'a [u8]) -> &'a [u8] {
    match spec.align {
        Align::Left => {
            let end = raw
                .iter()
                .rposition(|&b| b != spec.pad)
                .map_or(0, |i| i + 1);
            &raw[..end]
        }
        Align::Right => {
            let start = raw.iter().position(|&b| b != spec.pad).unwrap_or(raw.len());
            &raw[start..]
        }
    }
}

/// Lossless view of raw bytes for error messages.
fn show(raw: &[u8]) -> String {
    raw.iter().map(|&b| char::from(b)).collect()
}

fn kind_mismatch<T>(spec: &FieldSpec) -> FieldErrorKind {
    FieldErrorKind::KindMismatch {
        type_name: std::any::type_name::<T>(),
        kind: spec.kind.name(),
    }
}
