use super::ReadErrorKind;
use crate::Record;

/// Decodes a raw record into one of several record types.
///
/// Replaces the Python `as_record_type` callback. Format crates implement it
/// for an enum of their record types, usually by looking at a record type
/// code. Every [`Record`] implements it for itself.
pub trait Dispatch: Sized {
    /// Decodes `raw`, choosing the record type from its content.
    ///
    /// # Errors
    ///
    /// Returns [`ReadErrorKind::UnknownRecord`] when no record type matches,
    /// or [`ReadErrorKind::Record`] when the matching record is invalid.
    fn dispatch(raw: &[u8]) -> Result<Self, ReadErrorKind>;

    /// Name of this record's type, for error messages.
    fn type_name(&self) -> &'static str;
}

impl<R: Record> Dispatch for R {
    fn dispatch(raw: &[u8]) -> Result<Self, ReadErrorKind> {
        Ok(R::decode(raw)?)
    }

    fn type_name(&self) -> &'static str {
        R::NAME
    }
}
