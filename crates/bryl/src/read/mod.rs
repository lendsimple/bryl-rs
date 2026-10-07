//! Reading records from files and other byte streams.
//!
//! A [`Source`] splits input into raw records: [`LineSource`] for
//! newline-terminated records, [`BlockSource`] for fixed-size blocks. A
//! [`Dispatch`] type decides which record each one is (every [`Record`] is
//! one), and [`Reader`] combines the two with a one-record lookahead for
//! structured parsing.
//!
//! ```
//! use bryl::read::{LineSource, Reader};
//! use bryl::Record;
//!
//! #[derive(Record, Debug, PartialEq)]
//! struct Line {
//!     #[bryl(alpha(5))]
//!     name: String,
//!     #[bryl(numeric(3))]
//!     count: u16,
//! }
//!
//! let input = "apple007\npear 012\n";
//! let mut reader: Reader<_, Line> = Reader::new(LineSource::new(input.as_bytes()));
//! let lines: Vec<Line> = reader.by_ref().collect::<Result<_, _>>()?;
//! assert_eq!(lines[1].count, 12);
//! # Ok::<(), bryl::read::ReadError>(())
//! ```
//!
//! [`Record`]: crate::Record

mod block;
mod dispatch;
mod error;
mod line;
mod reader;

pub use block::BlockSource;
pub use dispatch::Dispatch;
pub use error::{Location, ReadError, ReadErrorKind};
pub use line::LineSource;
pub use reader::Reader;

/// One undecoded record and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raw {
    /// Record content, without its line terminator.
    pub bytes: Vec<u8>,
    /// Line terminator that followed the record (`\n`, `\r\n`, or empty).
    pub terminator: Vec<u8>,
    /// Where the record starts.
    pub location: Location,
}

/// Splits a byte stream into raw records.
pub trait Source {
    /// Reads the next raw record, or `None` at end of input.
    ///
    /// # Errors
    ///
    /// Returns an I/O error or a framing problem (bad terminator, truncated
    /// block), located by [`Source::location`].
    fn next_raw(&mut self) -> Result<Option<Raw>, ReadErrorKind>;

    /// Where the most recently attempted record starts (or where the next one
    /// would start, before any reads).
    fn location(&self) -> Location;
}
