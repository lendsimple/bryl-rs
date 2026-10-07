use super::FieldValue;
use crate::field::{FieldKind, FieldSpec};
use crate::{FieldErrorKind, Sanitize};

/// `None` is an empty field filled with the padding byte: spaces for
/// alphanumeric fields, and zeros for numeric, date and time fields unless
/// they set another pad (zeros give the Metro 2 "zero date" convention).
///
/// On decode, a field that is all spaces, all padding, or (for non-alphanumeric
/// fields) all zeros is `None`. For a zero-padded numeric field this means
/// `Some(0)` decodes as `None`.
impl<T: FieldValue> FieldValue for Option<T> {
    fn encode(
        &self,
        spec: &FieldSpec,
        sanitize: Sanitize,
        out: &mut Vec<u8>,
    ) -> Result<(), FieldErrorKind> {
        if let Some(value) = self {
            return value.encode(spec, sanitize, out);
        }
        out.resize(out.len() + spec.length, spec.pad);
        Ok(())
    }

    fn decode(spec: &FieldSpec, raw: &[u8]) -> Result<Self, FieldErrorKind> {
        let all = |byte: u8| raw.iter().all(|&b| b == byte);
        let zero_is_empty = spec.kind != FieldKind::Alpha;
        if all(b' ') || all(spec.pad) || (zero_is_empty && all(b'0')) {
            return Ok(None);
        }
        T::decode(spec, raw).map(Some)
    }
}
