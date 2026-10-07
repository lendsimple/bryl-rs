use std::borrow::Cow;

use super::{Dispatch, Location, ReadError, ReadErrorKind, Source};

/// Reads records of type `D` from a [`Source`], with one record of lookahead.
///
/// Iterating yields every record. For structured formats, [`Reader::expect`]
/// and [`Reader::next_if`] consume a record only if it is the expected type;
/// otherwise it stays queued for the next call.
///
/// A record that fails to decode is consumed along with its error, so
/// iteration can continue past it.
#[derive(Debug)]
pub struct Reader<S, D> {
    source: S,
    name: Cow<'static, str>,
    peeked: Option<(D, Location)>,
}

impl<S: Source, D: Dispatch> Reader<S, D> {
    /// Reads from `source`; errors name the input `<memory>`.
    pub fn new(source: S) -> Self {
        Self {
            source,
            name: Cow::Borrowed("<memory>"),
            peeked: None,
        }
    }

    /// Sets the input name used in errors, such as a file name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = name.into();
        self
    }

    /// The input name used in errors.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Location of the peeked record, or of the last record read.
    pub fn location(&self) -> Location {
        self.peeked
            .as_ref()
            .map_or_else(|| self.source.location(), |(_, location)| *location)
    }

    /// The wrapped source.
    pub fn source(&self) -> &S {
        &self.source
    }

    /// Returns the wrapped source.
    pub fn into_inner(self) -> S {
        self.source
    }

    /// Builds an error located at the current record.
    pub fn error(&self, kind: ReadErrorKind) -> ReadError {
        ReadError::new(self.name.clone(), self.location(), kind)
    }

    /// Returns the next record without consuming it.
    ///
    /// # Errors
    ///
    /// Returns the error for a record that cannot be read or decoded; that
    /// record is consumed.
    pub fn peek(&mut self) -> Result<Option<&D>, ReadError> {
        if self.peeked.is_none() {
            self.peeked = self.read()?;
        }
        Ok(self.peeked.as_ref().map(|(record, _)| record))
    }

    /// Consumes and returns the next record, or `None` at end of input.
    ///
    /// # Errors
    ///
    /// Returns the error for a record that cannot be read or decoded.
    pub fn next_record(&mut self) -> Result<Option<D>, ReadError> {
        match self.peeked.take() {
            Some((record, _)) => Ok(Some(record)),
            None => Ok(self.read()?.map(|(record, _)| record)),
        }
    }

    /// Consumes the next record if `select` accepts it.
    ///
    /// `select` returns `Ok(value)` to accept, or gives the record back with
    /// `Err(record)` to leave it queued. Returns `None` at end of input or
    /// when the record is not accepted.
    ///
    /// # Errors
    ///
    /// Returns the error for a record that cannot be read or decoded.
    pub fn next_if<T>(
        &mut self,
        select: impl FnOnce(D) -> Result<T, D>,
    ) -> Result<Option<T>, ReadError> {
        self.peek()?;
        let Some((record, location)) = self.peeked.take() else {
            return Ok(None);
        };
        match select(record) {
            Ok(value) => Ok(Some(value)),
            Err(record) => {
                self.peeked = Some((record, location));
                Ok(None)
            }
        }
    }

    /// Consumes the next record, which must be accepted by `select`.
    ///
    /// `expected` describes the required record for error messages.
    ///
    /// # Errors
    ///
    /// Returns [`ReadErrorKind::UnexpectedEof`] at end of input, or
    /// [`ReadErrorKind::UnexpectedRecord`] if `select` rejects the record,
    /// which stays queued.
    pub fn expect<T>(
        &mut self,
        expected: impl Into<Cow<'static, str>>,
        select: impl FnOnce(D) -> Result<T, D>,
    ) -> Result<T, ReadError> {
        let expected = expected.into();
        let Some(found) = self.peek()?.map(Dispatch::type_name) else {
            return Err(self.error(ReadErrorKind::UnexpectedEof { expected }));
        };
        match self.next_if(select)? {
            Some(value) => Ok(value),
            None => Err(self.error(ReadErrorKind::UnexpectedRecord { expected, found })),
        }
    }

    fn read(&mut self) -> Result<Option<(D, Location)>, ReadError> {
        let raw = match self.source.next_raw() {
            Ok(Some(raw)) => raw,
            Ok(None) => return Ok(None),
            Err(kind) => return Err(self.error(kind)),
        };
        match D::dispatch(&raw.bytes) {
            Ok(record) => Ok(Some((record, raw.location))),
            Err(kind) => Err(ReadError::new(self.name.clone(), raw.location, kind)),
        }
    }
}

impl<S: Source, D: Dispatch> Iterator for Reader<S, D> {
    type Item = Result<D, ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_record().transpose()
    }
}
