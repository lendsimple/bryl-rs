//! Writing Metro 2 files.

use std::io::Write;

use bryl::{EncodeCx, Record, Sanitize};

use crate::data_record::DataRecord;
use crate::error::Error;
use crate::records::{HeaderRecord, TrailerRecord};

/// Writes Metro 2 files (character format) to `W`.
///
/// ```
/// use chrono::NaiveDate;
/// use metro2::{
///     AccountStatus, AccountType, BaseSegment, DataRecord, EcoaCode, HeaderRecord, PortfolioType,
///     Writer,
/// };
///
/// let day = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
/// let header = HeaderRecord::builder()
///     .activity_date(day)
///     .date_created(day)
///     .reporter_name("ACME LENDING")
///     .reporter_address("123 MAIN ST ANYTOWN US 12345")
///     .reporter_telephone_number(5_551_234_567)
///     .build();
/// let base = BaseSegment::builder()
///     .identification_number("FURNISHER123")
///     .consumer_account_number("ACCT-001")
///     .portfolio_type(PortfolioType::Installment)
///     .account_type(AccountType::Unsecured)
///     .date_opened(NaiveDate::from_ymd_opt(2019, 6, 15).unwrap())
///     .account_status(AccountStatus::Current)
///     .date_of_account_information(day)
///     .surname("SMITH")
///     .first_name("JOHN")
///     .ecoa_code(EcoaCode::Individual)
///     .first_line_of_address("123 MAIN ST")
///     .city("ANYTOWN")
///     .state("CA")
///     .zip_code("90210")
///     .build();
///
/// let mut writer = Writer::new(Vec::new());
/// let mut file = writer.begin_file(&header)?;
/// file.write(&DataRecord::new(base))?;
/// let trailer = file.finish()?;
///
/// assert_eq!(trailer.total_base_records, 1);
/// assert_eq!(writer.into_inner().len(), 426 * 3);
/// # Ok::<(), metro2::Error>(())
/// ```
#[derive(Debug)]
pub struct Writer<W> {
    out: W,
    newline: bool,
    validate: bool,
    sanitize: Option<Sanitize>,
}

impl<W: Write> Writer<W> {
    /// Writes records back to back (the record descriptor word gives each
    /// record's length), validating base segments.
    pub fn new(out: W) -> Self {
        Self {
            out,
            newline: false,
            validate: true,
            sanitize: None,
        }
    }

    /// Ends every record with `\n` (off by default).
    #[must_use]
    pub fn newline(mut self, newline: bool) -> Self {
        self.newline = newline;
        self
    }

    /// Whether to check each base segment with [`crate::BaseSegment::validate`]
    /// before writing it (on by default).
    #[must_use]
    pub fn validate(mut self, validate: bool) -> Self {
        self.validate = validate;
        self
    }

    /// Overrides the records' default sanitizing (uppercase).
    #[must_use]
    pub fn sanitize(mut self, sanitize: Sanitize) -> Self {
        self.sanitize = Some(sanitize);
        self
    }

    /// The output.
    pub fn get_ref(&self) -> &W {
        &self.out
    }

    /// Returns the output.
    pub fn into_inner(self) -> W {
        self.out
    }

    /// Writes the header and starts a file.
    ///
    /// # Errors
    ///
    /// Returns an error if the header cannot be encoded or written.
    pub fn begin_file(&mut self, header: &HeaderRecord) -> Result<FileWriter<'_, W>, Error> {
        let mut file = FileWriter {
            writer: self,
            trailer: TrailerRecord::default(),
        };
        let mut buf = Vec::new();
        header.encode_into(&file.cx(), &mut buf)?;
        file.emit(buf)?;
        Ok(file)
    }
}

/// An open file. Call [`FileWriter::finish`] to write the trailer; dropping
/// it instead leaves the output without one.
#[must_use = "call `finish` to write the trailer record"]
#[derive(Debug)]
pub struct FileWriter<'w, W: Write> {
    writer: &'w mut Writer<W>,
    trailer: TrailerRecord,
}

impl<W: Write> FileWriter<'_, W> {
    /// Validates (unless disabled), writes and counts a data record.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Invalid`] if the base segment breaks a rule, or an
    /// encoding or I/O error. Nothing is written for a rejected record.
    pub fn write(&mut self, record: &DataRecord) -> Result<(), Error> {
        if self.writer.validate {
            record
                .base
                .validate()
                .map_err(|violations| Error::Invalid {
                    account: record.base.consumer_account_number.clone(),
                    violations,
                })?;
        }
        let mut buf = Vec::with_capacity(record.len() + 1);
        record.encode_into(&self.cx(), &mut buf)?;
        self.emit(buf)?;
        self.trailer.accumulate(record);
        Ok(())
    }

    /// The totals so far (the derived totals are filled in by `finish`).
    pub fn trailer(&self) -> &TrailerRecord {
        &self.trailer
    }

    /// Writes the trailer and returns it.
    ///
    /// # Errors
    ///
    /// Returns an error if the trailer cannot be encoded or written.
    pub fn finish(mut self) -> Result<TrailerRecord, Error> {
        self.trailer.finalize();
        let mut buf = Vec::new();
        self.trailer.encode_into(&self.cx(), &mut buf)?;
        self.emit(buf)?;
        self.writer.out.flush()?;
        Ok(self.trailer)
    }

    fn cx(&self) -> EncodeCx {
        EncodeCx {
            sanitize: self.writer.sanitize,
        }
    }

    fn emit(&mut self, mut buf: Vec<u8>) -> Result<(), Error> {
        if self.writer.newline {
            buf.push(b'\n');
        }
        self.writer.out.write_all(&buf)?;
        Ok(())
    }
}
