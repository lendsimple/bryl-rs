//! Reading Metro 2 files.

// The selection closures return `Err(record)` to hand a non-matching record
// back to the reader. That is not an error path: the record is moved once and
// re-queued, so the size of the `Err` variant does not matter.
#![allow(clippy::result_large_err)]

use std::borrow::Cow;
use std::io::{BufRead, ErrorKind, Read};

use bryl::Record;
use bryl::read::{Dispatch, LineSource, Location, Raw, ReadError, ReadErrorKind, Source};

use crate::data_record::{DataRecord, Segment};
use crate::error::Error;
use crate::file::File;
use crate::records::{HeaderRecord, TrailerRecord};

/// A header, data record or trailer.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)] // records are read one at a time
pub enum Metro2Record {
    /// The header record.
    Header(HeaderRecord),
    /// A base segment with its appended segments.
    Data(DataRecord),
    /// The trailer record.
    Trailer(TrailerRecord),
}

impl Metro2Record {
    /// The record as individual segments, in file order.
    pub fn into_segments(self) -> Vec<Segment> {
        match self {
            Self::Header(header) => vec![Segment::Header(header)],
            Self::Data(data) => data.into_segments(),
            Self::Trailer(trailer) => vec![Segment::Trailer(trailer)],
        }
    }
}

impl Dispatch for Metro2Record {
    /// Identifies the record by its content: `HEADER` or `TRAILER` after the
    /// record descriptor word, or a processing indicator of `1` for a base
    /// segment.
    fn dispatch(raw: &[u8]) -> Result<Self, ReadErrorKind> {
        if raw.get(4..10) == Some(b"HEADER") {
            return Ok(Self::Header(HeaderRecord::decode(&normalize_rdw::<
                HeaderRecord,
            >(raw))?));
        }
        if raw.get(4..11) == Some(b"TRAILER") {
            return Ok(Self::Trailer(TrailerRecord::decode(&normalize_rdw::<
                TrailerRecord,
            >(raw))?));
        }
        if raw.get(4) == Some(&b'1') {
            return DataRecord::decode(raw)
                .map(Self::Data)
                .map_err(|err| match err {
                    Error::Record(err) => err.into(),
                    other => ReadErrorKind::Other(other.to_string()),
                });
        }
        let found = raw.get(4..raw.len().min(11)).unwrap_or_default();
        Err(ReadErrorKind::UnknownRecord(format!(
            "{:?}",
            String::from_utf8_lossy(found)
        )))
    }

    fn type_name(&self) -> &'static str {
        match self {
            Self::Header(_) => HeaderRecord::NAME,
            Self::Data(_) => "DataRecord",
            Self::Trailer(_) => TrailerRecord::NAME,
        }
    }
}

/// Fixed-length files pad header and trailer records and give them a larger
/// record descriptor word (e.g. `0470`). The content is always the first 426
/// characters, so decode those with the standard word.
fn normalize_rdw<R: Record>(raw: &[u8]) -> Vec<u8> {
    let mut normalized = format!("{:04}", R::LENGTH).into_bytes();
    normalized.extend_from_slice(raw.get(4..R::LENGTH.min(raw.len())).unwrap_or_default());
    normalized
}

/// Records framed by their 4-digit record descriptor word.
#[derive(Debug)]
struct RdwSource<R> {
    reader: R,
    offset: u64,
    next_offset: u64,
}

impl<R: Read> RdwSource<R> {
    fn read_up_to(&mut self, buf: &mut [u8]) -> Result<usize, ReadErrorKind> {
        let mut filled = 0;
        while filled < buf.len() {
            match self.reader.read(&mut buf[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(err) if err.kind() == ErrorKind::Interrupted => {}
                Err(err) => return Err(err.into()),
            }
        }
        Ok(filled)
    }
}

impl<R: Read> Source for RdwSource<R> {
    fn next_raw(&mut self) -> Result<Option<Raw>, ReadErrorKind> {
        let mut rdw = [0; 4];
        let filled = self.read_up_to(&mut rdw)?;
        if filled == 0 {
            return Ok(None);
        }
        self.offset = self.next_offset;
        if filled < rdw.len() {
            return Err(ReadErrorKind::Truncated {
                expected: rdw.len(),
                actual: filled,
            });
        }
        let length: usize = std::str::from_utf8(&rdw)
            .ok()
            .filter(|s| s.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| {
                ReadErrorKind::Other(format!("invalid RDW: {:?}", String::from_utf8_lossy(&rdw)))
            })?;
        if length < rdw.len() {
            return Err(ReadErrorKind::Other(format!("invalid RDW: {length}")));
        }
        let mut bytes = vec![0; length];
        bytes[..4].copy_from_slice(&rdw);
        let filled = 4 + self.read_up_to(&mut bytes[4..])?;
        self.next_offset += filled as u64;
        if filled < length {
            return Err(ReadErrorKind::Truncated {
                expected: length,
                actual: filled,
            });
        }
        Ok(Some(Raw {
            bytes,
            terminator: Vec::new(),
            location: Location::Offset(self.offset),
        }))
    }

    fn location(&self) -> Location {
        Location::Offset(self.offset)
    }
}

/// How records are separated.
#[derive(Debug)]
enum Framing<R> {
    Rdw(RdwSource<R>),
    Lines(LineSource<R>),
}

impl<R: BufRead> Source for Framing<R> {
    fn next_raw(&mut self) -> Result<Option<Raw>, ReadErrorKind> {
        match self {
            Self::Rdw(source) => source.next_raw(),
            Self::Lines(source) => source.next_raw(),
        }
    }

    fn location(&self) -> Location {
        match self {
            Self::Rdw(source) => source.location(),
            Self::Lines(source) => source.location(),
        }
    }
}

/// Reads Metro 2 files (character format).
///
/// By default records are framed by their record descriptor word (RDW) and
/// errors are located by byte offset; [`Reader::newline`] switches to one
/// record per line (blank lines are skipped) located by line number.
///
/// ```
/// # fn read(input: &[u8]) -> Result<(), bryl::read::ReadError> {
/// let mut reader = metro2::Reader::new(input);
/// let header = reader.header()?;
/// for record in reader.data_records() {
///     println!("{}", record?.base.consumer_account_number);
/// }
/// let trailer = reader.trailer()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct Reader<R> {
    inner: bryl::read::Reader<Framing<R>, Metro2Record>,
}

impl<R: BufRead> Reader<R> {
    /// Reads RDW-framed records from `input`.
    pub fn new(input: R) -> Self {
        Self {
            inner: bryl::read::Reader::new(Framing::Rdw(RdwSource {
                reader: input,
                offset: 0,
                next_offset: 0,
            })),
        }
    }

    /// Reads one record per line instead (`true`) or RDW-framed records
    /// (`false`). Call before reading.
    #[must_use]
    pub fn newline(self, newline: bool) -> Self {
        let name = self.inner.name().to_owned();
        let input = match self.inner.into_inner() {
            Framing::Rdw(source) => source.reader,
            Framing::Lines(source) => source.into_inner(),
        };
        let framing = if newline {
            Framing::Lines(LineSource::new(input).skip_blank(true))
        } else {
            Framing::Rdw(RdwSource {
                reader: input,
                offset: 0,
                next_offset: 0,
            })
        };
        Self {
            inner: bryl::read::Reader::new(framing).with_name(name),
        }
    }

    /// Sets the input name used in errors.
    #[must_use]
    pub fn with_name(self, name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            inner: self.inner.with_name(name),
        }
    }

    /// Reads the header record.
    ///
    /// # Errors
    ///
    /// Returns an error unless the next record is a header.
    pub fn header(&mut self) -> Result<HeaderRecord, ReadError> {
        self.inner.expect("header", |record| match record {
            Metro2Record::Header(header) => Ok(header),
            other => Err(other),
        })
    }

    /// Reads the next data record, or returns `None` at the trailer or end of
    /// input.
    ///
    /// # Errors
    ///
    /// Returns an error if the next record cannot be read.
    pub fn next_data_record(&mut self) -> Result<Option<DataRecord>, ReadError> {
        self.inner.next_if(|record| match record {
            Metro2Record::Data(data) => Ok(data),
            other => Err(other),
        })
    }

    /// Iterates over data records until the trailer.
    pub fn data_records(&mut self) -> DataRecords<'_, R> {
        DataRecords { reader: self }
    }

    /// Reads the trailer record.
    ///
    /// # Errors
    ///
    /// Returns an error unless the next record is a trailer.
    pub fn trailer(&mut self) -> Result<TrailerRecord, ReadError> {
        self.inner.expect("trailer", |record| match record {
            Metro2Record::Trailer(trailer) => Ok(trailer),
            other => Err(other),
        })
    }

    /// Checks that nothing follows the trailer.
    ///
    /// # Errors
    ///
    /// Returns an error if another record follows.
    pub fn finish(&mut self) -> Result<(), ReadError> {
        match self.inner.next_record()? {
            None => Ok(()),
            Some(record) => Err(self.inner.error(ReadErrorKind::UnexpectedRecord {
                expected: Cow::Borrowed("end of file"),
                found: record.type_name(),
            })),
        }
    }

    /// Reads a whole file; see [`File::read`].
    ///
    /// # Errors
    ///
    /// Returns the first record that is missing, out of place or invalid.
    pub fn read_file(&mut self) -> Result<File, ReadError> {
        let header = self.header()?;
        let data_records = self.data_records().collect::<Result<_, _>>()?;
        let trailer = self.trailer()?;
        self.finish()?;
        Ok(File {
            header,
            data_records,
            trailer,
        })
    }

    /// Iterates over every record as individual segments: header, base,
    /// J1…N1, …, trailer.
    pub fn segments(self) -> impl Iterator<Item = Result<Segment, ReadError>> {
        self.flat_map(|record| match record {
            Ok(record) => record.into_segments().into_iter().map(Ok).collect(),
            Err(err) => vec![Err(err)],
        })
    }
}

impl<R: BufRead> Iterator for Reader<R> {
    type Item = Result<Metro2Record, ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// Iterator over data records; see [`Reader::data_records`].
#[derive(Debug)]
pub struct DataRecords<'r, R> {
    reader: &'r mut Reader<R>,
}

impl<R: BufRead> Iterator for DataRecords<'_, R> {
    type Item = Result<DataRecord, ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.reader.next_data_record().transpose()
    }
}
