use std::borrow::Cow;
use std::fmt;

use thiserror::Error;

/// Position of a record in its input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    /// 1-based line number.
    Line(usize),
    /// 0-based byte offset.
    Offset(u64),
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Line(line) => write!(f, "line {line}"),
            Self::Offset(offset) => write!(f, "offset {offset}"),
        }
    }
}

/// What went wrong while reading.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ReadErrorKind {
    /// Input ended where a record was required.
    #[error("unexpected EOF, expected {expected}")]
    UnexpectedEof {
        /// Description of the record that was required.
        expected: Cow<'static, str>,
    },
    /// A record of the wrong type was found.
    #[error("unexpected record type {found}, expected {expected}")]
    UnexpectedRecord {
        /// Description of the record that was required.
        expected: Cow<'static, str>,
        /// Type of the record found.
        found: &'static str,
    },
    /// The content does not match any known record type.
    #[error("unknown record type {0}")]
    UnknownRecord(String),
    /// A line ended with an unexpected terminator.
    #[error("unexpected line terminator {found:?}, expected {expected:?}")]
    UnexpectedTerminator {
        /// The required terminator.
        expected: String,
        /// The terminator found.
        found: String,
    },
    /// The input ended in the middle of a fixed-size record.
    #[error("truncated record: expected {expected} bytes, got {actual}")]
    Truncated {
        /// Required size.
        expected: usize,
        /// Bytes available.
        actual: usize,
    },
    /// The record could not be decoded. Boxed to keep [`ReadError`] small.
    #[error(transparent)]
    Record(Box<crate::Error>),
    /// Reading the input failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// A format-specific problem.
    #[error("{0}")]
    Other(String),
}

impl From<crate::Error> for ReadErrorKind {
    fn from(err: crate::Error) -> Self {
        Self::Record(Box::new(err))
    }
}

/// An error reading a record, with its location.
///
/// Displays as `"{source} @ {location} - {reason}"`, like the Python
/// `MalformedError`.
#[derive(Debug, Error)]
#[error("{source_name} @ {location} - {kind}")]
pub struct ReadError {
    /// Name of the input, such as a file name, or `<memory>`.
    pub source_name: Cow<'static, str>,
    /// Where the offending record starts.
    pub location: Location,
    /// What went wrong.
    pub kind: ReadErrorKind,
}

impl ReadError {
    /// Creates an error.
    pub fn new(
        source_name: impl Into<Cow<'static, str>>,
        location: Location,
        kind: ReadErrorKind,
    ) -> Self {
        Self {
            source_name: source_name.into(),
            location,
            kind,
        }
    }
}
