use thiserror::Error;

use crate::validate::Violation;

/// An error writing or decoding Metro 2 data.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Writing the output failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// A record could not be encoded or decoded.
    #[error(transparent)]
    Record(#[from] bryl::Error),
    /// A data record is longer than a 4-digit record descriptor word allows.
    #[error("data record is {length} characters; the record descriptor word allows at most 9999")]
    RecordTooLong {
        /// Total length.
        length: usize,
    },
    /// A segment identifier was found without the whole segment.
    #[error("truncated {segment} segment")]
    TruncatedSegment {
        /// Segment identifier, e.g. `J1`.
        segment: &'static str,
    },
    /// A segment that may appear once appeared again.
    #[error("duplicate {segment} segment")]
    DuplicateSegment {
        /// Segment identifier, e.g. `K1`.
        segment: &'static str,
    },
    /// A data record breaks a Metro 2 rule; nothing was written for it.
    #[error("account {account}: {}", violations.iter().map(ToString::to_string).collect::<Vec<_>>().join("; "))]
    Invalid {
        /// Consumer account number.
        account: String,
        /// The rules broken.
        violations: Vec<Violation>,
    },
}
