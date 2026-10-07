//! The NACHA record types (94 characters each).

use bon::Builder;
use bryl::read::{Dispatch, ReadErrorKind};
use bryl::{Const, Record};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

use crate::codes::{ReturnReasonCode, ServiceClassCode, StandardEntryClass, TransactionCode};
use crate::types::{FileIdModifier, RoutingNumber};

/// Length of every NACHA record.
pub const RECORD_LENGTH: usize = 94;

/// File header record (`1`).
#[derive(Record, Builder, Debug, Clone, PartialEq, Eq)]
#[bryl(sanitize(upper), length = 94)]
#[builder(on(String, into))]
pub struct FileHeader {
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(1), constant = "1")]
    #[builder(skip)]
    pub record_type: Const,
    /// Fixed by the format; stores nothing.
    #[bryl(numeric(2), constant = 1)]
    #[builder(skip)]
    pub priority_code: Const,
    /// Routing number of the receiving point, written as ` TTTTAAAAC`.
    #[bryl(numeric(10), pad = ' ')]
    pub immediate_destination: RoutingNumber,
    /// Right-aligned and space-padded. NACHA's developer guide describes it
    /// as the originating bank's routing number "preceded by a blank"
    /// (` 123456789`); some banks ask for a 10-character company ID instead,
    /// which is written unchanged. Check your bank's file specification.
    #[bryl(alpha(10), align = "right")]
    pub immediate_origin: String,
    /// Date the file was created.
    #[bryl(date("YYMMDD"))]
    pub file_creation_date: NaiveDate,
    /// Time the file was created.
    #[bryl(time("hhmm"))]
    pub file_creation_time: NaiveTime,
    /// Distinguishes files created on the same day (`A`–`Z`, `0`–`9`).
    #[bryl(alpha(1))]
    #[builder(default)]
    pub file_id_modifier: FileIdModifier,
    /// Fixed by the format; stores nothing.
    #[bryl(numeric(3), constant = 94)]
    #[builder(skip)]
    pub record_size: Const,
    /// Fixed by the format; stores nothing.
    #[bryl(numeric(2), constant = 10)]
    #[builder(skip)]
    pub blocking_factor: Const,
    /// Fixed by the format; stores nothing.
    #[bryl(numeric(1), constant = 1)]
    #[builder(skip)]
    pub format_code: Const,
    /// Name of the receiving point.
    #[bryl(alpha(23))]
    pub immediate_destination_name: String,
    /// Name of the originator.
    #[bryl(alpha(23))]
    pub immediate_origin_name: String,
    /// Optional reference for the originator's use.
    #[bryl(alpha(8))]
    #[builder(default)]
    pub reference_code: String,
}

impl FileHeader {
    /// File creation date and time combined.
    pub fn file_creation(&self) -> NaiveDateTime {
        self.file_creation_date.and_time(self.file_creation_time)
    }
}

/// Company/batch header record (`5`).
#[derive(Record, Builder, Debug, Clone, PartialEq, Eq)]
#[bryl(sanitize(upper), length = 94)]
#[builder(on(String, into))]
pub struct BatchHeader {
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(1), constant = "5")]
    #[builder(skip)]
    pub record_type: Const,
    /// Which transactions the batch contains.
    #[bryl(numeric(3))]
    pub service_class_code: ServiceClassCode,
    /// Originating company name.
    #[bryl(alpha(16))]
    pub company_name: String,
    /// Company discretionary data.
    #[bryl(alpha(20))]
    #[builder(default)]
    pub company_discretionary_data: String,
    /// Originating company identification.
    #[bryl(alpha(10))]
    pub company_id: String,
    /// Standard entry class (SEC) code.
    #[bryl(alpha(3))]
    pub standard_entry_class: StandardEntryClass,
    /// Entry description shown to receivers, e.g. `PAYROLL`.
    #[bryl(alpha(10))]
    pub company_entry_description: String,
    /// Descriptive date shown to receivers.
    #[bryl(alpha(6))]
    #[builder(default)]
    pub company_descriptive_date: String,
    /// Date the originator intends the entries to settle.
    #[bryl(date("YYMMDD"))]
    pub effective_entry_date: NaiveDate,
    /// Julian settlement date, filled in by the ACH operator; blank when
    /// originating.
    #[bryl(alpha(3))]
    #[builder(default)]
    pub settlement_date: String,
    /// Fixed by the format; stores nothing.
    #[bryl(numeric(1), constant = 1)]
    #[builder(skip)]
    pub originator_status_code: Const,
    /// First eight digits of the originating DFI's routing number.
    #[bryl(numeric(8))]
    pub originating_dfi_id: u32,
    /// Batch number, ascending from 1 within the file.
    #[bryl(numeric(7))]
    pub batch_number: u32,
}

/// Entry detail record (`6`).
#[derive(Record, Builder, Debug, Clone, PartialEq, Eq)]
#[bryl(sanitize(upper), length = 94)]
#[builder(on(String, into))]
pub struct EntryDetail {
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(1), constant = "6")]
    #[builder(skip)]
    pub record_type: Const,
    /// Account type and debit/credit/prenote/return.
    #[bryl(numeric(2))]
    pub transaction_code: TransactionCode,
    /// Receiving DFI routing number: the TRN (8 digits) and check digit.
    #[bryl(numeric(9))]
    pub receiving_dfi: RoutingNumber,
    /// Left-aligned and space-padded, like every NACHA alphanumeric field;
    /// bank specifications (Chase, Regions, First Citizens) agree.
    #[bryl(alpha(17))]
    pub receiving_dfi_account_number: String,
    /// Amount in cents.
    #[bryl(numeric(10))]
    pub amount: u64,
    /// Receiver's identification number.
    #[bryl(alpha(15))]
    pub individual_id: String,
    /// Receiver's name.
    #[bryl(alpha(22))]
    pub individual_name: String,
    /// ODFI discretionary data.
    #[bryl(alpha(2))]
    #[builder(default)]
    pub discretionary_data: String,
    /// `1` if addenda records follow, otherwise `0`.
    #[bryl(numeric(1), max = 1)]
    #[builder(default)]
    pub addenda_record_indicator: u8,
    /// ODFI ID (8 digits) followed by a 7-digit sequence number. Trace
    /// numbers must be ascending (not necessarily consecutive) within a batch
    /// and unique within the file (NACHA's developer guide).
    #[bryl(numeric(15))]
    pub trace_number: u64,
}

impl EntryDetail {
    /// A copy with the account number replaced by `X`s, for logging.
    #[must_use]
    pub fn mask(&self) -> Self {
        Self {
            receiving_dfi_account_number: "X".repeat(17),
            ..self.clone()
        }
    }

    /// Last seven digits of the trace number, which addenda refer to.
    pub const fn sequence_number(&self) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // < 10^7
        let sequence = (self.trace_number % 10_000_000) as u32;
        sequence
    }
}

/// Entry detail addenda record (`7`, addenda type `05`).
#[derive(Record, Builder, Debug, Clone, PartialEq, Eq)]
#[bryl(sanitize(upper), length = 94)]
#[builder(on(String, into))]
pub struct Addendum {
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(1), constant = "7")]
    #[builder(skip)]
    pub record_type: Const,
    /// Fixed by the format; stores nothing.
    #[bryl(numeric(2), constant = 5)]
    #[builder(skip)]
    pub addenda_type_code: Const,
    /// Free-form payment information.
    #[bryl(alpha(80))]
    #[builder(default)]
    pub payment_related_information: String,
    /// Position of this addendum within its entry, starting at 1 (`0001` in
    /// bank specifications).
    #[bryl(numeric(4))]
    pub addenda_sequence_number: u16,
    /// Last seven digits of the entry's trace number.
    #[bryl(numeric(7))]
    pub entry_detail_sequence_number: u32,
}

/// Return addenda record (`7`, addenda type `99`). A return entry carries
/// exactly one, in place of type-05 addenda.
///
/// The layout is the standard return layout. Dishonored and contested
/// dishonored returns rearrange the addenda information and are not
/// supported (see [`ReturnReasonCode`]).
#[derive(Record, Builder, Debug, Clone, PartialEq, Eq)]
#[bryl(sanitize(upper), length = 94)]
#[builder(on(String, into))]
pub struct ReturnAddendum {
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(1), constant = "7")]
    #[builder(skip)]
    pub record_type: Const,
    /// Fixed by the format; stores nothing.
    #[bryl(numeric(2), constant = 99)]
    #[builder(skip)]
    pub addenda_type_code: Const,
    /// Why the entry was returned.
    #[bryl(alpha(3))]
    pub return_reason_code: ReturnReasonCode,
    /// Trace number of the entry being returned.
    #[bryl(numeric(15))]
    pub original_entry_trace_number: u64,
    /// Only for R14 and R15; blank otherwise. Two-digit years decode as
    /// 20YY.
    #[bryl(date("YYMMDD"), pad = ' ')]
    pub date_of_death: Option<NaiveDate>,
    /// First eight digits of the original entry's receiving DFI routing
    /// number.
    #[bryl(numeric(8))]
    pub original_receiving_dfi_id: u32,
    /// Free-form information from the returning DFI.
    #[bryl(alpha(44))]
    #[builder(default)]
    pub addenda_information: String,
    /// Trace number of the return entry itself, the same as its entry
    /// detail's.
    #[bryl(numeric(15))]
    pub trace_number: u64,
}

/// Company/batch control record (`8`).
#[derive(Record, Builder, Debug, Clone, PartialEq, Eq)]
#[bryl(sanitize(upper), length = 94)]
#[builder(on(String, into))]
pub struct BatchControl {
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(1), constant = "8")]
    #[builder(skip)]
    pub record_type: Const,
    /// Which transactions the batch contains.
    #[bryl(numeric(3))]
    pub service_class_code: ServiceClassCode,
    /// Entry detail plus addenda records covered.
    #[bryl(numeric(6))]
    pub entry_addenda_count: u32,
    /// Sum of receiving DFI TRNs, modulo 10^10.
    #[bryl(numeric(10))]
    pub entry_hash: u64,
    /// Total debits, in cents.
    #[bryl(numeric(12))]
    pub total_debit_amount: u64,
    /// Total credits, in cents.
    #[bryl(numeric(12))]
    pub total_credit_amount: u64,
    /// Originating company identification.
    #[bryl(alpha(10))]
    pub company_id: String,
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(19), reserved)]
    #[builder(skip)]
    pub message_authentication_code: Const,
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(6), reserved)]
    #[builder(skip)]
    pub reserved: Const,
    /// First eight digits of the originating DFI's routing number.
    #[bryl(numeric(8))]
    pub originating_dfi_id: u32,
    /// Batch number, ascending from 1 within the file.
    #[bryl(numeric(7))]
    pub batch_number: u32,
}

/// File control record (`9`).
#[derive(Record, Builder, Debug, Clone, PartialEq, Eq)]
#[bryl(sanitize(upper), length = 94)]
pub struct FileControl {
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(1), constant = "9")]
    #[builder(skip)]
    pub record_type: Const,
    /// Number of batches.
    #[bryl(numeric(6))]
    pub batch_count: u32,
    /// Number of 10-record blocks, including filler records. NACHA's
    /// developer guide requires files to be padded to whole blocks with
    /// lines of `9`s.
    #[bryl(numeric(6))]
    pub block_count: u32,
    /// Entry detail plus addenda records covered.
    #[bryl(numeric(8))]
    pub entry_addenda_count: u32,
    /// Sum of receiving DFI TRNs, modulo 10^10.
    #[bryl(numeric(10))]
    pub entry_hash: u64,
    /// Total debits, in cents.
    #[bryl(numeric(12))]
    pub total_debit_amount: u64,
    /// Total credits, in cents.
    #[bryl(numeric(12))]
    pub total_credit_amount: u64,
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(39), reserved)]
    #[builder(skip)]
    pub reserved: Const,
}

/// Any NACHA line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NachaRecord {
    /// `1`
    FileHeader(FileHeader),
    /// `5`
    BatchHeader(BatchHeader),
    /// `6`
    EntryDetail(EntryDetail),
    /// `7`, addenda type `05`
    Addendum(Addendum),
    /// `7`, addenda type `99`
    ReturnAddendum(ReturnAddendum),
    /// `8`
    BatchControl(BatchControl),
    /// `9`
    FileControl(FileControl),
    /// A line of 94 `9`s, padding the file to a multiple of 10 lines.
    Filler,
}

/// A filler line: 94 `9`s.
pub const FILLER: [u8; RECORD_LENGTH] = [b'9'; RECORD_LENGTH];

impl Dispatch for NachaRecord {
    /// Picks the record type from the first character. Lines must be exactly
    /// 94 characters (excluding the line terminator).
    fn dispatch(raw: &[u8]) -> Result<Self, ReadErrorKind> {
        if raw == FILLER {
            return Ok(Self::Filler);
        }
        Ok(match raw.first() {
            Some(b'1') => Self::FileHeader(FileHeader::decode_exact(raw)?),
            Some(b'5') => Self::BatchHeader(BatchHeader::decode_exact(raw)?),
            Some(b'6') => Self::EntryDetail(EntryDetail::decode_exact(raw)?),
            Some(b'7') if raw.get(1..3) == Some(b"99") => {
                Self::ReturnAddendum(ReturnAddendum::decode_exact(raw)?)
            }
            Some(b'7') => Self::Addendum(Addendum::decode_exact(raw)?),
            Some(b'8') => Self::BatchControl(BatchControl::decode_exact(raw)?),
            Some(b'9') => Self::FileControl(FileControl::decode_exact(raw)?),
            Some(&b) => {
                return Err(ReadErrorKind::UnknownRecord(format!("{:?}", char::from(b))));
            }
            None => return Err(ReadErrorKind::UnknownRecord("(empty line)".into())),
        })
    }

    fn type_name(&self) -> &'static str {
        match self {
            Self::FileHeader(_) => FileHeader::NAME,
            Self::BatchHeader(_) => BatchHeader::NAME,
            Self::EntryDetail(_) => EntryDetail::NAME,
            Self::Addendum(_) => Addendum::NAME,
            Self::ReturnAddendum(_) => ReturnAddendum::NAME,
            Self::BatchControl(_) => BatchControl::NAME,
            Self::FileControl(_) => FileControl::NAME,
            Self::Filler => "Filler",
        }
    }
}
