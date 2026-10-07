//! [NACHA](https://www.nacha.org/) ACH files: fixed-width, 94-character
//! records laid out as
//!
//! ```text
//! FileHeader            1
//!   BatchHeader         5
//!     EntryDetail       6
//!       Addendum        7  (zero or more)
//!     ...
//!   BatchControl        8
//!   ...
//! FileControl           9
//! 9999...9999              (filler to a multiple of 10 lines)
//! ```
//!
//! Write files with [`Writer`], whose nested guards make it impossible to
//! write an entry outside a batch or a batch outside a file, and which
//! computes every control total. Read them with [`Reader`] record by record,
//! or with [`File::read`] all at once, and check them with
//! [`File::validate`].
//!
//! This is a port of `lms-python`'s `common/nacha.py`. Intentional
//! differences are listed in the repository's `DEVIATIONS.md`.

mod codes;
mod entry;
mod error;
mod file;
mod reader;
mod records;
mod totals;
mod types;
mod validate;
mod writer;

pub use codes::{AccountKind, REVERSAL, ServiceClassCode, StandardEntryClass, TransactionCode};
pub use entry::Entry;
pub use error::Error;
pub use file::{Batch, File};
pub use reader::{Entries, Reader};
pub use records::{
    Addendum, BatchControl, BatchHeader, EntryDetail, FILLER, FileControl, FileHeader, NachaRecord,
    RECORD_LENGTH,
};
pub use totals::{HASH_MODULUS, Totals};
pub use types::{FileIdModifier, FileIdModifierError, RoutingNumber, RoutingNumberError};
pub use validate::{Issue, IssueKind};
pub use writer::{
    BatchParams, BatchWriter, EntryParams, FileParams, FileWriter, LineEnding, Writer,
};
