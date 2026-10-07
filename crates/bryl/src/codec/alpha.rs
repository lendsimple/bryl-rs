use super::{FieldValue, kind_mismatch, pad_into, show, strip};
use crate::field::{FieldKind, FieldSpec};
use crate::sanitize::is_printable;
use crate::{FieldErrorKind, Sanitize};

impl FieldValue for String {
    fn encode(
        &self,
        spec: &FieldSpec,
        sanitize: Sanitize,
        out: &mut Vec<u8>,
    ) -> Result<(), FieldErrorKind> {
        if spec.kind != FieldKind::Alpha {
            return Err(kind_mismatch::<Self>(spec));
        }
        let value = sanitize.apply(self, spec.length);
        encode_text(spec, &value, out)
    }

    fn decode(spec: &FieldSpec, raw: &[u8]) -> Result<Self, FieldErrorKind> {
        if spec.kind != FieldKind::Alpha {
            return Err(kind_mismatch::<Self>(spec));
        }
        decode_text(spec, raw)
    }
}

/// Validates and pads text that is already sanitized.
pub(super) fn encode_text(
    spec: &FieldSpec,
    value: &str,
    out: &mut Vec<u8>,
) -> Result<(), FieldErrorKind> {
    let actual = value.chars().count();
    if actual > spec.length {
        return Err(FieldErrorKind::TooLong {
            max: spec.length,
            actual,
        });
    }
    if let Some((index, ch)) = value.chars().enumerate().find(|&(_, c)| !is_printable(c)) {
        return Err(FieldErrorKind::InvalidChar { ch, index });
    }
    pad_into(spec, value.as_bytes(), out);
    Ok(())
}

/// Strips padding and checks the remaining text is printable ASCII.
pub(super) fn decode_text(spec: &FieldSpec, raw: &[u8]) -> Result<String, FieldErrorKind> {
    if let Some((index, &b)) = raw
        .iter()
        .enumerate()
        .find(|&(_, &b)| !is_printable(char::from(b)))
    {
        return Err(FieldErrorKind::InvalidChar {
            ch: char::from(b),
            index,
        });
    }
    Ok(show(strip(spec, raw)))
}
