//! Writing NACHA files.

use std::io::Write;

use bon::Builder;
use bryl::{EncodeCx, Record, Sanitize};
use chrono::{NaiveDate, NaiveDateTime, Utc};

use crate::codes::{ServiceClassCode, StandardEntryClass, TransactionCode};
use crate::entry::Entry;
use crate::error::Error;
use crate::records::{
    Addendum, BatchControl, BatchHeader, EntryDetail, FILLER, FileControl, FileHeader,
};
use crate::totals::Totals;
use crate::types::{FileIdModifier, RoutingNumber};
use crate::validate::{batch_issues, entry_issues};

const MAX_SEQUENCE: u32 = 9_999_999;

/// Writes NACHA files to `W`.
///
/// ```
/// use chrono::NaiveDate;
/// use nacha::{
///     BatchParams, EntryParams, FileParams, RoutingNumber, ServiceClassCode,
///     StandardEntryClass, TransactionCode, Writer,
/// };
///
/// let created_at = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap().and_hms_opt(14, 30, 0).unwrap();
/// let mut writer = Writer::new(Vec::new());
/// let mut file = writer.begin_file(
///     FileParams::builder()
///         .immediate_destination("091000019".parse()?)
///         .immediate_destination_name("DEST BANK")
///         .immediate_origin("1234567890")
///         .immediate_origin_name("ACME CORP")
///         .created_at(created_at)
///         .build(),
/// )?;
/// let mut batch = file.begin_batch(
///     BatchParams::builder()
///         .service_class_code(ServiceClassCode::CreditsOnly)
///         .company_name("ACME CORP")
///         .company_id("1234567890")
///         .standard_entry_class(StandardEntryClass::Ppd)
///         .company_entry_description("PAYROLL")
///         .originating_dfi_id(12_345_678)
///         .build(),
/// )?;
/// batch.entry(
///     EntryParams::builder()
///         .transaction_code(TransactionCode::CheckingCredit)
///         .receiving_dfi("091000019".parse()?)
///         .account_number("123456789")
///         .amount(10_000)
///         .individual_id("EMP001")
///         .individual_name("JANE SMITH")
///         .build(),
/// )?;
/// batch.finish()?;
/// let control = file.finish()?;
///
/// assert_eq!(control.total_credit_amount, 10_000);
/// let output = String::from_utf8(writer.into_inner()).unwrap();
/// assert_eq!(output.lines().count(), 10); // 5 records + 5 filler lines
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Nesting is checked by the compiler. An entry needs a batch:
///
/// ```compile_fail,E0599
/// # use nacha::{EntryParams, FileParams, Writer};
/// # fn f(writer: &mut Writer<Vec<u8>>, file: FileParams, entry: EntryParams) {
/// let mut file = writer.begin_file(file).unwrap();
/// file.entry(entry); // no method `entry` on `FileWriter`
/// # }
/// ```
///
/// and only one file or batch can be open at a time:
///
/// ```compile_fail,E0499
/// # use nacha::{FileParams, Writer};
/// # fn f(writer: &mut Writer<Vec<u8>>, a: FileParams, b: FileParams) {
/// let first = writer.begin_file(a).unwrap();
/// let second = writer.begin_file(b).unwrap(); // `writer` is already borrowed
/// first.finish().unwrap();
/// # }
/// ```
///
/// ```compile_fail,E0499
/// # use nacha::{BatchParams, FileParams, Writer};
/// # fn f(writer: &mut Writer<Vec<u8>>, file: FileParams, a: BatchParams, b: BatchParams) {
/// let mut file = writer.begin_file(file).unwrap();
/// let first = file.begin_batch(a).unwrap();
/// let second = file.begin_batch(b).unwrap(); // `file` is already borrowed
/// first.finish().unwrap();
/// # }
/// ```
#[derive(Debug)]
pub struct Writer<W> {
    out: W,
    pad_blocks: bool,
    line_ending: LineEnding,
    sanitize: Option<Sanitize>,
}

/// How each record line ends. NACHA does not specify it; some banks require
/// CRLF.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum LineEnding {
    /// `\n` (the default).
    #[default]
    Lf,
    /// `\r\n`.
    CrLf,
}

impl LineEnding {
    /// The bytes that end a line.
    pub const fn as_bytes(self) -> &'static [u8] {
        match self {
            Self::Lf => b"\n",
            Self::CrLf => b"\r\n",
        }
    }
}

/// File header values. The file ID modifier defaults to `A`.
#[derive(Builder, Debug, Clone, PartialEq, Eq)]
#[builder(on(String, into))]
pub struct FileParams {
    /// Routing number of the receiving point.
    pub immediate_destination: RoutingNumber,
    /// Name of the receiving point.
    pub immediate_destination_name: String,
    /// Origin identifier, usually a 10-digit company ID or a routing number.
    pub immediate_origin: String,
    /// Origin name.
    pub immediate_origin_name: String,
    /// File creation date and time; also the default effective entry date.
    pub created_at: NaiveDateTime,
    /// Distinguishes files created on the same day.
    #[builder(default)]
    pub file_id_modifier: FileIdModifier,
    /// Optional reference code.
    #[builder(default)]
    pub reference_code: String,
}

impl FileParams {
    /// The current UTC time, for [`FileParams::created_at`].
    pub fn now() -> NaiveDateTime {
        Utc::now().naive_utc()
    }
}

/// Batch header values.
#[derive(Builder, Debug, Clone, PartialEq, Eq)]
#[builder(on(String, into))]
pub struct BatchParams {
    /// Which transactions the batch may contain.
    pub service_class_code: ServiceClassCode,
    /// Company name.
    pub company_name: String,
    /// Company identification.
    pub company_id: String,
    /// SEC code.
    pub standard_entry_class: StandardEntryClass,
    /// Entry description shown to receivers, e.g. `PAYROLL`.
    pub company_entry_description: String,
    /// First eight digits of the originating DFI's routing number.
    pub originating_dfi_id: u32,
    /// Defaults to the file creation date.
    pub effective_entry_date: Option<NaiveDate>,
    /// Descriptive date shown to receivers.
    #[builder(default)]
    pub company_descriptive_date: String,
    /// Company discretionary data.
    #[builder(default)]
    pub company_discretionary_data: String,
}

/// Entry detail values. Each string in `addenda` becomes an addendum's
/// payment related information.
#[derive(Builder, Debug, Clone, PartialEq, Eq)]
#[builder(on(String, into))]
pub struct EntryParams {
    /// Transaction code.
    pub transaction_code: TransactionCode,
    /// Receiving DFI routing number.
    pub receiving_dfi: RoutingNumber,
    /// Receiving account number.
    pub account_number: String,
    /// Amount in cents.
    pub amount: u64,
    /// Individual identification number.
    pub individual_id: String,
    /// Individual name.
    pub individual_name: String,
    /// Defaults to the ODFI ID followed by a file-wide 7-digit sequence.
    pub trace_number: Option<u64>,
    /// Discretionary data.
    #[builder(default)]
    pub discretionary_data: String,
    /// Payment related information, one addendum each.
    #[builder(default)]
    pub addenda: Vec<String>,
}

impl<W: Write> Writer<W> {
    /// Writes to `out`, padding files to whole blocks.
    pub fn new(out: W) -> Self {
        Self {
            out,
            pad_blocks: true,
            line_ending: LineEnding::Lf,
            sanitize: None,
        }
    }

    /// How record lines end (`\n` by default).
    #[must_use]
    pub fn line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    /// Whether to pad files with filler lines to a multiple of 10 records
    /// (on by default).
    #[must_use]
    pub fn pad_blocks(mut self, pad: bool) -> Self {
        self.pad_blocks = pad;
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

    /// Writes the file header and starts a file.
    ///
    /// # Errors
    ///
    /// Returns an error if the header cannot be encoded or written.
    pub fn begin_file(&mut self, params: FileParams) -> Result<FileWriter<'_, W>, Error> {
        let header = FileHeader::builder()
            .immediate_destination(params.immediate_destination)
            .immediate_destination_name(params.immediate_destination_name)
            .immediate_origin(params.immediate_origin)
            .immediate_origin_name(params.immediate_origin_name)
            .file_creation_date(params.created_at.date())
            .file_creation_time(params.created_at.time())
            .file_id_modifier(params.file_id_modifier)
            .reference_code(params.reference_code)
            .build();
        let mut file = FileWriter {
            writer: self,
            created_at: params.created_at,
            lines: 0,
            batches_started: 0,
            batch_count: 0,
            next_trace_sequence: 1,
            totals: Totals::default(),
            header,
        };
        let mut buf = Vec::new();
        file.encode(&file.header.clone(), &mut buf)?;
        file.emit(&buf, 1)?;
        Ok(file)
    }
}

/// An open file. Call [`FileWriter::finish`] to write the file control record;
/// dropping it instead leaves the output without one.
#[must_use = "call `finish` to write the file control record"]
#[derive(Debug)]
pub struct FileWriter<'w, W: Write> {
    writer: &'w mut Writer<W>,
    created_at: NaiveDateTime,
    /// Records written so far, excluding filler.
    lines: u32,
    /// Batches begun, including any dropped without `finish`.
    batches_started: u32,
    /// Batches finished.
    batch_count: u32,
    /// Next trace sequence; unique across the file.
    next_trace_sequence: u32,
    totals: Totals,
    header: FileHeader,
}

impl<'w, W: Write> FileWriter<'w, W> {
    /// The file header written by [`Writer::begin_file`].
    pub fn header(&self) -> &FileHeader {
        &self.header
    }

    /// Writes a batch header and starts a batch. Batches are numbered from 1
    /// in the order they are begun.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidBatch`] if the header breaks a NACHA rule (e.g.
    /// an ENR batch not described as `AUTOENROLL`), or an encoding or I/O
    /// error.
    pub fn begin_batch(&mut self, params: BatchParams) -> Result<BatchWriter<'_, 'w, W>, Error> {
        let batch_number = self.batches_started + 1;
        if batch_number > MAX_SEQUENCE {
            return Err(Error::SequenceOverflow("batch number"));
        }
        let header = BatchHeader::builder()
            .service_class_code(params.service_class_code)
            .company_name(params.company_name)
            .company_discretionary_data(params.company_discretionary_data)
            .company_id(params.company_id)
            .standard_entry_class(params.standard_entry_class)
            .company_entry_description(params.company_entry_description)
            .company_descriptive_date(params.company_descriptive_date)
            .effective_entry_date(
                params
                    .effective_entry_date
                    .unwrap_or_else(|| self.created_at.date()),
            )
            .originating_dfi_id(params.originating_dfi_id)
            .batch_number(batch_number)
            .build();
        if let Some(issue) = batch_issues(&header).into_iter().next() {
            return Err(Error::InvalidBatch(issue));
        }
        let mut buf = Vec::new();
        self.encode(&header, &mut buf)?;
        self.emit(&buf, 1)?;
        self.batches_started = batch_number;
        Ok(BatchWriter {
            file: self,
            header,
            totals: Totals::default(),
        })
    }

    /// Writes the file control record (and filler lines) and returns it.
    ///
    /// # Errors
    ///
    /// Returns an error if the control record cannot be encoded or written.
    pub fn finish(mut self) -> Result<FileControl, Error> {
        let lines = self.lines + 1;
        let control = FileControl::builder()
            .batch_count(self.batch_count)
            .block_count(lines.div_ceil(10))
            .entry_addenda_count(self.totals.entry_addenda_count)
            .entry_hash(self.totals.entry_hash)
            .total_debit_amount(self.totals.total_debit_amount)
            .total_credit_amount(self.totals.total_credit_amount)
            .build();
        let mut buf = Vec::new();
        self.encode(&control, &mut buf)?;
        if self.writer.pad_blocks {
            for _ in 0..(10 - lines % 10) % 10 {
                buf.extend_from_slice(&FILLER);
                buf.extend_from_slice(self.writer.line_ending.as_bytes());
            }
        }
        self.emit(&buf, 1)?;
        self.writer.out.flush()?;
        Ok(control)
    }

    fn encode<R: Record>(&self, record: &R, buf: &mut Vec<u8>) -> Result<(), bryl::Error> {
        let cx = EncodeCx {
            sanitize: self.writer.sanitize,
        };
        record.encode_into(&cx, buf)?;
        buf.extend_from_slice(self.writer.line_ending.as_bytes());
        Ok(())
    }

    fn emit(&mut self, buf: &[u8], records: u32) -> Result<(), Error> {
        self.writer.out.write_all(buf)?;
        self.lines += records;
        Ok(())
    }
}

/// An open batch. Call [`BatchWriter::finish`] to write the batch control
/// record; dropping it instead leaves the batch without one.
#[must_use = "call `finish` to write the batch control record"]
#[derive(Debug)]
pub struct BatchWriter<'f, 'w, W: Write> {
    file: &'f mut FileWriter<'w, W>,
    header: BatchHeader,
    totals: Totals,
}

impl<W: Write> BatchWriter<'_, '_, W> {
    /// The batch header written by [`FileWriter::begin_batch`].
    pub fn header(&self) -> &BatchHeader {
        &self.header
    }

    /// Writes an entry and its addenda, and returns them.
    ///
    /// The entry is checked against the batch (service class, debit/credit
    /// rules of the SEC code, prenote amount, addenda limits) and fully
    /// encoded before anything is written, so a rejected entry leaves the
    /// output unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidEntry`] for a rule violation, or an encoding or
    /// I/O error.
    pub fn entry(&mut self, params: EntryParams) -> Result<Entry, Error> {
        let sequence = self.file.next_trace_sequence;
        if sequence > MAX_SEQUENCE {
            return Err(Error::SequenceOverflow("trace sequence"));
        }
        let trace_number = params.trace_number.unwrap_or_else(|| {
            u64::from(self.header.originating_dfi_id) * 10_000_000 + u64::from(sequence)
        });
        let detail = EntryDetail::builder()
            .transaction_code(params.transaction_code)
            .receiving_dfi(params.receiving_dfi)
            .receiving_dfi_account_number(params.account_number)
            .amount(params.amount)
            .individual_id(params.individual_id)
            .individual_name(params.individual_name)
            .discretionary_data(params.discretionary_data)
            .addenda_record_indicator(u8::from(!params.addenda.is_empty()))
            .trace_number(trace_number)
            .build();
        let addenda = params
            .addenda
            .into_iter()
            .zip(1u16..)
            .map(|(info, position)| {
                Addendum::builder()
                    .payment_related_information(info)
                    .addenda_sequence_number(position)
                    .entry_detail_sequence_number(detail.sequence_number())
                    .build()
            })
            .collect();
        let entry = Entry { detail, addenda };
        if let Some(issue) = entry_issues(&self.header, &entry).into_iter().next() {
            return Err(Error::InvalidEntry(issue));
        }

        let mut buf = Vec::new();
        self.file.encode(&entry.detail, &mut buf)?;
        for addendum in &entry.addenda {
            self.file.encode(addendum, &mut buf)?;
        }
        let records = u32::try_from(entry.addenda.len() + 1).unwrap_or(u32::MAX);
        self.file.emit(&buf, records)?;
        self.file.next_trace_sequence += 1;
        self.totals.add_entry(&entry);
        Ok(entry)
    }

    /// Writes the batch control record and returns it.
    ///
    /// # Errors
    ///
    /// Returns an error if the control record cannot be encoded or written.
    pub fn finish(self) -> Result<BatchControl, Error> {
        let control = BatchControl::builder()
            .service_class_code(self.header.service_class_code)
            .entry_addenda_count(self.totals.entry_addenda_count)
            .entry_hash(self.totals.entry_hash)
            .total_debit_amount(self.totals.total_debit_amount)
            .total_credit_amount(self.totals.total_credit_amount)
            .company_id(self.header.company_id.clone())
            .originating_dfi_id(self.header.originating_dfi_id)
            .batch_number(self.header.batch_number)
            .build();
        let mut buf = Vec::new();
        self.file.encode(&control, &mut buf)?;
        self.file.emit(&buf, 1)?;
        self.file.totals.add(&self.totals);
        self.file.batch_count += 1;
        Ok(control)
    }
}
