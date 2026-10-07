//! Reading NACHA files.

// The selection closures return `Err(record)` to hand a non-matching record
// back to the reader. That is not an error path: the record is moved once and
// re-queued, so the size of the `Err` variant does not matter.
#![allow(clippy::result_large_err)]

use std::borrow::Cow;
use std::io::BufRead;

use bryl::read::{LineSource, ReadError, ReadErrorKind};

use crate::entry::Entry;
use crate::file::{Batch, File};
use crate::records::{BatchControl, BatchHeader, FileControl, FileHeader, NachaRecord};

/// Reads NACHA records, either one at a time or in file order.
///
/// Structured reading mirrors the file layout:
///
/// ```
/// # fn read(input: &[u8]) -> Result<(), bryl::read::ReadError> {
/// let mut reader = nacha::Reader::new(input).with_name("ach.txt");
/// let header = reader.file_header()?;
/// while let Some(batch) = reader.next_batch()? {
///     for entry in reader.entries() {
///         let entry = entry?;
///         println!("{} {}", entry.detail.individual_name, entry.detail.amount);
///     }
///     reader.batch_control()?;
/// }
/// reader.file_control()?;
/// reader.finish()?;
/// # Ok(())
/// # }
/// ```
///
/// Iterating yields every record, including [`NachaRecord::Filler`] lines.
/// Lines end with `\n` or `\r\n` and must be exactly 94 characters.
#[derive(Debug)]
pub struct Reader<R> {
    inner: bryl::read::Reader<LineSource<R>, NachaRecord>,
}

impl<R: BufRead> Reader<R> {
    /// Reads from `input`.
    pub fn new(input: R) -> Self {
        Self {
            inner: bryl::read::Reader::new(LineSource::new(input)),
        }
    }

    /// Sets the input name used in errors.
    #[must_use]
    pub fn with_name(self, name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            inner: self.inner.with_name(name),
        }
    }

    /// Reads the file header.
    ///
    /// # Errors
    ///
    /// Returns an error unless the next record is a file header.
    pub fn file_header(&mut self) -> Result<FileHeader, ReadError> {
        self.inner.expect("FileHeader", |record| match record {
            NachaRecord::FileHeader(header) => Ok(header),
            other => Err(other),
        })
    }

    /// Reads the next batch header, or returns `None` if the next record is
    /// not one (normally the file control).
    ///
    /// # Errors
    ///
    /// Returns an error if the next record cannot be read.
    pub fn next_batch(&mut self) -> Result<Option<BatchHeader>, ReadError> {
        self.inner.next_if(|record| match record {
            NachaRecord::BatchHeader(header) => Ok(header),
            other => Err(other),
        })
    }

    /// Reads the next entry and its addenda, or returns `None` if the next
    /// record is not an entry detail (normally the batch control).
    ///
    /// # Errors
    ///
    /// Returns an error if a record cannot be read.
    pub fn next_entry(&mut self) -> Result<Option<Entry>, ReadError> {
        let Some(detail) = self.inner.next_if(|record| match record {
            NachaRecord::EntryDetail(detail) => Ok(detail),
            other => Err(other),
        })?
        else {
            return Ok(None);
        };
        let mut addenda = Vec::new();
        while let Some(addendum) = self.inner.next_if(|record| match record {
            NachaRecord::Addendum(addendum) => Ok(addendum),
            other => Err(other),
        })? {
            addenda.push(addendum);
        }
        Ok(Some(Entry { detail, addenda }))
    }

    /// Iterates over the current batch's entries.
    pub fn entries(&mut self) -> Entries<'_, R> {
        Entries { reader: self }
    }

    /// Reads the batch control.
    ///
    /// # Errors
    ///
    /// Returns an error unless the next record is a batch control.
    pub fn batch_control(&mut self) -> Result<BatchControl, ReadError> {
        self.inner.expect("BatchControl", |record| match record {
            NachaRecord::BatchControl(control) => Ok(control),
            other => Err(other),
        })
    }

    /// Reads the file control.
    ///
    /// # Errors
    ///
    /// Returns an error unless the next record is a file control.
    pub fn file_control(&mut self) -> Result<FileControl, ReadError> {
        self.inner.expect("FileControl", |record| match record {
            NachaRecord::FileControl(control) => Ok(control),
            other => Err(other),
        })
    }

    /// Consumes the filler lines after the file control and checks that
    /// nothing else follows. Returns the number of filler lines.
    ///
    /// # Errors
    ///
    /// Returns an error if any other record follows.
    pub fn finish(&mut self) -> Result<usize, ReadError> {
        let mut filler = 0;
        while let Some(record) = self.inner.next_record()? {
            if record != NachaRecord::Filler {
                return Err(self.inner.error(ReadErrorKind::UnexpectedRecord {
                    expected: Cow::Borrowed("end of file"),
                    found: bryl::read::Dispatch::type_name(&record),
                }));
            }
            filler += 1;
        }
        Ok(filler)
    }

    /// Reads a whole file; see [`File::read`].
    ///
    /// # Errors
    ///
    /// Returns the first record that is missing, out of place or invalid.
    pub fn read_file(&mut self) -> Result<File, ReadError> {
        let header = self.file_header()?;
        let mut batches = Vec::new();
        while let Some(batch_header) = self.next_batch()? {
            let entries = self.entries().collect::<Result<_, _>>()?;
            let control = self.batch_control()?;
            batches.push(Batch {
                header: batch_header,
                entries,
                control,
            });
        }
        let control = self.file_control()?;
        let filler_count = self.finish()?;
        Ok(File {
            header,
            batches,
            control,
            filler_count,
        })
    }
}

impl<R: BufRead> Iterator for Reader<R> {
    type Item = Result<NachaRecord, ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// Iterator over a batch's entries; see [`Reader::entries`].
#[derive(Debug)]
pub struct Entries<'r, R> {
    reader: &'r mut Reader<R>,
}

impl<R: BufRead> Iterator for Entries<'_, R> {
    type Item = Result<Entry, ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.reader.next_entry().transpose()
    }
}
