use super::alpha::{decode_text, encode_text};
use super::numeric::{decode_number, encode_number};
use super::{FieldValue, kind_mismatch};
use crate::field::{Const, Constant, FieldKind, FieldSpec};
use crate::{FieldErrorKind, Sanitize};

impl FieldValue for Const {
    fn encode(
        &self,
        spec: &FieldSpec,
        _sanitize: Sanitize,
        out: &mut Vec<u8>,
    ) -> Result<(), FieldErrorKind> {
        let constant = spec.constant.ok_or(FieldErrorKind::MissingConstant)?;
        encode_constant(spec, constant, out)
    }

    fn decode(spec: &FieldSpec, raw: &[u8]) -> Result<Self, FieldErrorKind> {
        let constant = spec.constant.ok_or(FieldErrorKind::MissingConstant)?;
        match (constant, spec.kind) {
            (Constant::Str(expected), FieldKind::Alpha) => {
                let found = decode_text(spec, raw)?;
                if found != expected {
                    return Err(FieldErrorKind::ConstantMismatch {
                        expected: expected.to_owned(),
                        found,
                    });
                }
            }
            (Constant::Num(expected), FieldKind::Numeric { .. }) => {
                let found = decode_number(spec, raw)?;
                if found != expected {
                    return Err(FieldErrorKind::ConstantMismatch {
                        expected: expected.to_string(),
                        found: found.to_string(),
                    });
                }
            }
            _ => return Err(kind_mismatch::<Self>(spec)),
        }
        Ok(Const)
    }
}

/// Writes a constant; constants are never sanitized.
pub(crate) fn encode_constant(
    spec: &FieldSpec,
    constant: Constant,
    out: &mut Vec<u8>,
) -> Result<(), FieldErrorKind> {
    match (constant, spec.kind) {
        (Constant::Str(value), FieldKind::Alpha) => encode_text(spec, value, out),
        (Constant::Num(value), FieldKind::Numeric { .. }) => encode_number(spec, value, out),
        _ => Err(kind_mismatch::<Const>(spec)),
    }
}
