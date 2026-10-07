use thiserror::Error;

use crate::validate::IssueKind;

/// An error writing a NACHA file.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Writing the output failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// A record could not be encoded.
    #[error(transparent)]
    Record(#[from] bryl::Error),
    /// An entry breaks a NACHA rule; nothing was written for it.
    #[error("invalid entry: {0}")]
    InvalidEntry(IssueKind),
    /// The file has used every 7-digit trace sequence or batch number.
    #[error("{0} exceeds 7 digits")]
    SequenceOverflow(&'static str),
}
