//! Reading Metro 2 files.

// The selection closures return `Err(record)` to hand a non-matching record
// back to the reader. That is not an error path: the record is moved once and
// re-queued, so the size of the `Err` variant does not matter.
#![allow(clippy::result_large_err)]

use std::borrow::Cow;
use std::collections::VecDeque;
use std::io::{BufRead, ErrorKind, Read};
use std::ops::Range;

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

/// Smallest record: header, base segment and trailer are 426 characters.
const MIN_RECORD_LENGTH: usize = 426;

/// True if `unit` is a single header or trailer record (not a block).
fn is_plain_header_or_trailer(unit: &[u8]) -> bool {
    unit.get(4..10) == Some(b"HEADER") || unit.get(4..11) == Some(b"TRAILER")
}

/// Splits a variable block (a 4-digit block descriptor word followed by
/// RDW-prefixed records and optional blank padding) into its records' byte
/// ranges. Returns `None` if `unit` is not a well-formed block: every record
/// must have a valid record descriptor word, start like a header, trailer or
/// base segment, and fit in the block.
fn split_block(unit: &[u8]) -> Option<Vec<Range<usize>>> {
    let mut records = Vec::new();
    let mut pos = 4;
    while pos < unit.len() {
        if unit[pos..].iter().all(|&b| b == b' ') {
            break;
        }
        let rdw = unit.get(pos..pos + 4)?;
        if !rdw.iter().all(u8::is_ascii_digit) {
            return None;
        }
        let length: usize = std::str::from_utf8(rdw).ok()?.parse().ok()?;
        let end = pos + length;
        let record = unit.get(pos..end)?;
        let starts_like_a_record =
            is_plain_header_or_trailer(record) || record.get(4) == Some(&b'1');
        if length < MIN_RECORD_LENGTH || !starts_like_a_record {
            return None;
        }
        records.push(pos..end);
        pos = end;
    }
    (!records.is_empty()).then_some(records)
}

/// Unpacks variable-blocked files: a block descriptor word (BDW) holding one
/// or more records.
///
/// A file is blocked if its first unit is a block whose first record is the
/// header. Unblocked files pass through unchanged. In a blocked file, a unit
/// that is not a well-formed block is passed through as a single record, so
/// files that block only some records (as moov-io/metro2's sample does) are
/// read too.
#[derive(Debug)]
struct Blocks<S> {
    inner: S,
    blocked: Option<bool>,
    pending: VecDeque<Raw>,
    /// Blocks read so far.
    count: usize,
}

impl<S: Source> Blocks<S> {
    fn new(inner: S) -> Self {
        Self {
            inner,
            blocked: None,
            pending: VecDeque::new(),
            count: 0,
        }
    }
}

impl<S: Source> Source for Blocks<S> {
    fn next_raw(&mut self) -> Result<Option<Raw>, ReadErrorKind> {
        if let Some(raw) = self.pending.pop_front() {
            return Ok(Some(raw));
        }
        let Some(unit) = self.inner.next_raw()? else {
            return Ok(None);
        };
        let blocked = *self.blocked.get_or_insert_with(|| {
            split_block(&unit.bytes).is_some_and(|records| {
                unit.bytes.get(records[0].start + 4..records[0].start + 10) == Some(b"HEADER")
            })
        });
        if !blocked {
            return Ok(Some(unit));
        }
        // In a blocked file every unit is a block; a record outside a block
        // counts as a block of one.
        self.count += 1;
        if is_plain_header_or_trailer(&unit.bytes) {
            return Ok(Some(unit));
        }
        let Some(records) = split_block(&unit.bytes) else {
            return Ok(Some(unit));
        };
        for range in records {
            let location = match unit.location {
                Location::Offset(offset) => Location::Offset(offset + range.start as u64),
                line @ Location::Line(_) => line,
            };
            self.pending.push_back(Raw {
                bytes: unit.bytes[range].to_vec(),
                terminator: Vec::new(),
                location,
            });
        }
        Ok(self.pending.pop_front())
    }

    fn location(&self) -> Location {
        self.inner.location()
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
/// Variable-blocked files, where block descriptor words group records into
/// blocks, are read in either mode; see [`Reader::blocks`].
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
    inner: bryl::read::Reader<Blocks<Framing<R>>, Metro2Record>,
}

impl<R: BufRead> Reader<R> {
    /// Reads RDW-framed records from `input`.
    pub fn new(input: R) -> Self {
        Self {
            inner: bryl::read::Reader::new(Blocks::new(Framing::Rdw(RdwSource {
                reader: input,
                offset: 0,
                next_offset: 0,
            }))),
        }
    }

    /// Reads one record per line instead (`true`) or RDW-framed records
    /// (`false`). Call before reading.
    #[must_use]
    pub fn newline(self, newline: bool) -> Self {
        let name = self.inner.name().to_owned();
        let input = match self.inner.into_inner().inner {
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
            inner: bryl::read::Reader::new(Blocks::new(framing)).with_name(name),
        }
    }

    /// Blocks read so far in a variable-blocked file, counting a record
    /// outside a block as a block of one; 0 for a file that is not blocked.
    pub fn blocks(&self) -> usize {
        self.inner.source().count
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
            blocks: self.blocks(),
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
