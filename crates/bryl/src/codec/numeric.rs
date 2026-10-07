use super::{FieldValue, kind_mismatch, pad_into, show, strip};
use crate::field::{FieldKind, FieldSpec};
use crate::{FieldErrorKind, Sanitize};

/// Validates `value` against the field and writes its padded digits.
pub(super) fn encode_number(
    spec: &FieldSpec,
    value: u64,
    out: &mut Vec<u8>,
) -> Result<(), FieldErrorKind> {
    let digits = value.to_string();
    if digits.len() > spec.length {
        return Err(FieldErrorKind::TooLong {
            max: spec.length,
            actual: digits.len(),
        });
    }
    check_bounds(spec, value)?;
    pad_into(spec, digits.as_bytes(), out);
    Ok(())
}

/// Strips padding and parses the remaining ASCII digits; blank is zero.
pub(super) fn decode_number(spec: &FieldSpec, raw: &[u8]) -> Result<u64, FieldErrorKind> {
    let digits = strip(spec, raw);
    if !digits.iter().all(u8::is_ascii_digit) {
        return Err(FieldErrorKind::NotNumeric { raw: show(digits) });
    }
    let value = if digits.is_empty() {
        0
    } else {
        show(digits)
            .parse::<u64>()
            .map_err(|_| FieldErrorKind::Overflow {
                raw: show(digits),
                target: "u64",
            })?
    };
    check_bounds(spec, value)?;
    Ok(value)
}

fn check_bounds(spec: &FieldSpec, value: u64) -> Result<(), FieldErrorKind> {
    let FieldKind::Numeric { min, max } = spec.kind else {
        return Ok(());
    };
    if let Some(min) = min.filter(|&min| value < min) {
        return Err(FieldErrorKind::BelowMin { min, value });
    }
    if let Some(max) = max.filter(|&max| value > max) {
        return Err(FieldErrorKind::AboveMax { max, value });
    }
    Ok(())
}

macro_rules! impl_unsigned {
    ($($ty:ty),*) => {$(
        impl FieldValue for $ty {
            fn encode(
                &self,
                spec: &FieldSpec,
                _sanitize: Sanitize,
                out: &mut Vec<u8>,
            ) -> Result<(), FieldErrorKind> {
                if !matches!(spec.kind, FieldKind::Numeric { .. }) {
                    return Err(kind_mismatch::<Self>(spec));
                }
                encode_number(spec, u64::from(*self), out)
            }

            fn decode(spec: &FieldSpec, raw: &[u8]) -> Result<Self, FieldErrorKind> {
                if !matches!(spec.kind, FieldKind::Numeric { .. }) {
                    return Err(kind_mismatch::<Self>(spec));
                }
                let value = decode_number(spec, raw)?;
                Self::try_from(value).map_err(|_| FieldErrorKind::Overflow {
                    raw: value.to_string(),
                    target: stringify!($ty),
                })
            }
        }
    )*};
}

impl_unsigned!(u8, u16, u32, u64);
