//! NACHA rules checked when writing entries and when validating files.

use std::fmt;

use bryl::read::Location;

use thiserror::Error;

use crate::codes::{
    REVERSAL, ReturnReasonCode, ServiceClassCode, StandardEntryClass, TransactionCode,
};
use crate::entry::{Entry, SpecialAddendum};
use crate::records::BatchHeader;

/// A rule a file or entry breaks.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum IssueKind {
    /// A control record total does not match the records it covers.
    #[error("{field} is {recorded}, computed {computed}")]
    ControlMismatch {
        /// Control field name.
        field: &'static str,
        /// Value computed from the records.
        computed: u64,
        /// Value in the control record.
        recorded: u64,
    },
    /// A batch control record disagrees with its batch header.
    #[error("batch control {field} does not match the batch header")]
    HeaderMismatch {
        /// Field name.
        field: &'static str,
    },
    /// The transaction is not allowed by the batch's service class.
    #[error("transaction code {transaction_code} is not allowed in a {service_class_code} batch")]
    ServiceClass {
        /// Batch service class.
        service_class_code: ServiceClassCode,
        /// Entry transaction code.
        transaction_code: TransactionCode,
    },
    /// The transaction's direction is not allowed by the batch's SEC code
    /// (e.g. a credit in a TEL batch, or a debit in a CIE batch). Reversal
    /// batches are exempt.
    #[error("transaction code {transaction_code} is not allowed in a {standard_entry_class} batch")]
    EntryClassDirection {
        /// Batch SEC code.
        standard_entry_class: StandardEntryClass,
        /// Entry transaction code.
        transaction_code: TransactionCode,
    },
    /// The batch's SEC code requires a specific company entry description.
    #[error(
        "{standard_entry_class} batches must use company entry description {expected}, got {found:?}"
    )]
    EntryDescription {
        /// Batch SEC code.
        standard_entry_class: StandardEntryClass,
        /// The required description.
        expected: &'static str,
        /// The description in the batch header.
        found: String,
    },
    /// Batch numbers are not ascending.
    #[error("batch number is not greater than the previous batch's ({previous})")]
    BatchNumberOrder {
        /// The previous batch's number.
        previous: u32,
    },
    /// A prenote has a non-zero amount.
    #[error("prenote amount must be 0, got {amount}")]
    PrenoteAmount {
        /// The amount.
        amount: u64,
    },
    /// A zero-dollar remittance entry has a non-zero amount.
    #[error("zero-dollar remittance amount must be 0, got {amount}")]
    ZeroDollarAmount {
        /// The amount.
        amount: u64,
    },
    /// The batch's SEC code does not allow the zero-dollar remittance code
    /// (see [`StandardEntryClass::allows_zero_dollar_remittance`]).
    #[error(
        "zero-dollar remittance code {transaction_code} is not allowed in a {standard_entry_class} batch"
    )]
    ZeroDollarEntryClass {
        /// Batch SEC code.
        standard_entry_class: StandardEntryClass,
        /// Entry transaction code.
        transaction_code: TransactionCode,
    },
    /// A zero-dollar remittance entry has no addendum to carry its
    /// remittance data.
    #[error("zero-dollar remittance entries need at least one addendum")]
    ZeroDollarAddenda,
    /// A loan debit (55) outside a reversal batch.
    #[error("loan debits (55) are allowed only in a reversal batch")]
    LoanDebitNotReversal,
    /// More addenda than the entry class allows.
    #[error("{standard_entry_class} entries allow at most {max} addenda, got {count}")]
    TooManyAddenda {
        /// Batch SEC code.
        standard_entry_class: StandardEntryClass,
        /// Maximum allowed.
        max: u16,
        /// Addenda present.
        count: usize,
    },
    /// Fewer addenda than the entry class requires.
    #[error("{standard_entry_class} entries need at least {min} addenda, got {count}")]
    TooFewAddenda {
        /// Batch SEC code.
        standard_entry_class: StandardEntryClass,
        /// Minimum required.
        min: u16,
        /// Addenda present.
        count: usize,
    },
    /// The addenda record indicator does not match the addenda present.
    #[error("addenda record indicator is {recorded} but the entry has {count} addenda")]
    AddendaIndicator {
        /// Indicator value.
        recorded: u8,
        /// Addenda present.
        count: usize,
    },
    /// Addenda are not numbered 1, 2, 3, ...
    #[error("addendum {position} has sequence number {recorded}")]
    AddendaSequence {
        /// 1-based position of the addendum.
        position: usize,
        /// Its sequence number.
        recorded: u16,
    },
    /// An addendum's entry detail sequence number does not match its entry.
    #[error("addendum refers to entry sequence {recorded}, expected {expected}")]
    AddendaEntrySequence {
        /// Value in the addendum.
        recorded: u32,
        /// Last seven digits of the entry's trace number.
        expected: u32,
    },
    /// A return entry (other than in a COR batch) has no return addendum.
    #[error("return entries need a return addendum")]
    MissingReturnAddendum,
    /// An entry in a COR batch has no notification of change addendum.
    #[error("notification of change entries need a type-98 addendum")]
    MissingNocAddendum,
    /// A notification of change addendum outside a COR batch.
    #[error("notification of change addenda are not allowed in a {standard_entry_class} batch")]
    UnexpectedNocAddendum {
        /// Batch SEC code.
        standard_entry_class: StandardEntryClass,
    },
    /// A notification of change has a non-zero amount.
    #[error("notification of change amount must be 0, got {amount}")]
    NocAmount {
        /// The amount.
        amount: u64,
    },
    /// A notification of change uses a transaction code that is not a
    /// return or notification of change code (x1, x6).
    #[error("transaction code {transaction_code} is not a notification of change code")]
    NocTransactionCode {
        /// Entry transaction code.
        transaction_code: TransactionCode,
    },
    /// A notification of change addendum's trace number differs from its
    /// entry's.
    #[error("notification of change trace number is {recorded:015}, expected {expected:015}")]
    NocTraceNumber {
        /// Value in the addendum.
        recorded: u64,
        /// The entry's trace number.
        expected: u64,
    },
    /// An entry that is not a return, or a notification of change entry, has
    /// a return addendum.
    #[error(
        "transaction code {transaction_code} is not a return but the entry has a return addendum"
    )]
    UnexpectedReturnAddendum {
        /// Entry transaction code.
        transaction_code: TransactionCode,
    },
    /// A return entry has type-05 addenda as well as its return addendum.
    #[error("return entries carry only their return addendum, got {count} other addenda")]
    ReturnEntryAddenda {
        /// Type-05 addenda present.
        count: usize,
    },
    /// A return addendum's trace number differs from its entry's.
    #[error("return addendum trace number is {recorded:015}, expected {expected:015}")]
    ReturnTraceNumber {
        /// Value in the return addendum.
        recorded: u64,
        /// The entry's trace number.
        expected: u64,
    },
    /// A dishonored or contested return's two-digit reason code (positions
    /// 57-58, or 77-78 for the dishonored return code) names no known code.
    #[error("{field} {code:02} is not a known return reason code")]
    UnknownReturnReasonCode {
        /// Field name.
        field: &'static str,
        /// The two digits.
        code: u8,
    },
    /// A date of death with a return reason other than R14 or R15.
    #[error("a date of death is only allowed with R14 and R15, got {return_reason_code}")]
    DateOfDeath {
        /// The return reason code.
        return_reason_code: ReturnReasonCode,
    },
    /// Lenient reading could not decode an entry; see
    /// [`UnreadEntry`](crate::UnreadEntry).
    #[error("entry could not be read: {location}: {error}")]
    UnreadEntry {
        /// Where the record that failed to decode starts.
        location: Location,
        /// Why it failed.
        error: bryl::Error,
    },
    /// Trace numbers within a batch are not ascending.
    #[error("trace number is not greater than the previous entry's")]
    TraceOrder,
    /// The trace number appears more than once in the file.
    #[error("trace number appears more than once in the file")]
    DuplicateTrace,
    /// The file is not padded to a multiple of 10 lines with filler records.
    #[error("{lines} records and {filler} filler lines are not a multiple of 10")]
    BlockPadding {
        /// Records, excluding filler.
        lines: usize,
        /// Filler lines.
        filler: usize,
    },
}

/// A rule broken by a file, located by batch and entry where possible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// Batch number, for batch and entry issues.
    pub batch_number: Option<u32>,
    /// Trace number, for entry issues.
    pub trace_number: Option<u64>,
    /// The rule broken.
    pub kind: IssueKind,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(batch) = self.batch_number {
            write!(f, "batch {batch}: ")?;
        }
        if let Some(trace) = self.trace_number {
            write!(f, "entry {trace:015}: ")?;
        }
        write!(f, "{}", self.kind)
    }
}

impl std::error::Error for Issue {}

/// Rules for a batch header on its own.
pub(crate) fn batch_issues(batch: &BatchHeader) -> Vec<IssueKind> {
    let sec = batch.standard_entry_class;
    match sec.required_entry_description() {
        Some(expected)
            if !batch
                .company_entry_description
                .trim()
                .eq_ignore_ascii_case(expected) =>
        {
            vec![IssueKind::EntryDescription {
                standard_entry_class: sec,
                expected,
                found: batch.company_entry_description.clone(),
            }]
        }
        _ => Vec::new(),
    }
}

/// Rules for a single entry within its batch.
pub(crate) fn entry_issues(batch: &BatchHeader, entry: &Entry) -> Vec<IssueKind> {
    let mut issues = Vec::new();
    let detail = &entry.detail;
    let code = detail.transaction_code;
    if !batch.service_class_code.allows(code) {
        issues.push(IssueKind::ServiceClass {
            service_class_code: batch.service_class_code,
            transaction_code: code,
        });
    }
    let sec = batch.standard_entry_class;
    let reversal = batch
        .company_entry_description
        .trim()
        .eq_ignore_ascii_case(REVERSAL);
    let wrong_direction =
        (sec.debits_only() && code.is_credit()) || (sec.credits_only() && code.is_debit());
    if wrong_direction && !reversal {
        issues.push(IssueKind::EntryClassDirection {
            standard_entry_class: sec,
            transaction_code: code,
        });
    }
    if code.is_prenote() && detail.amount != 0 {
        issues.push(IssueKind::PrenoteAmount {
            amount: detail.amount,
        });
    }
    if code.is_zero_dollar_remittance() {
        zero_dollar_issues(sec, entry, &mut issues);
    }
    if code == TransactionCode::LoanDebit && !reversal {
        issues.push(IssueKind::LoanDebitNotReversal);
    }
    // Notifications of change (COR) use return transaction codes too, but
    // carry type-98 addenda.
    if sec == StandardEntryClass::Cor {
        noc_issues(entry, &mut issues);
        addenda_limit_issues(sec, entry, &mut issues);
    } else {
        if entry
            .special_addendum
            .as_ref()
            .is_some_and(SpecialAddendum::is_noc)
        {
            issues.push(IssueKind::UnexpectedNocAddendum {
                standard_entry_class: sec,
            });
        }
        if code.is_return() {
            return_issues(entry, &mut issues);
        } else {
            if entry
                .special_addendum
                .as_ref()
                .is_some_and(SpecialAddendum::is_return)
            {
                issues.push(IssueKind::UnexpectedReturnAddendum {
                    transaction_code: code,
                });
            }
            addenda_limit_issues(sec, entry, &mut issues);
        }
    }
    if detail.addenda_record_indicator != u8::from(entry.addenda_count() > 0) {
        issues.push(IssueKind::AddendaIndicator {
            recorded: detail.addenda_record_indicator,
            count: entry.addenda_count(),
        });
    }
    for (index, addendum) in entry.addenda.iter().enumerate() {
        let position = index + 1;
        if usize::from(addendum.addenda_sequence_number) != position {
            issues.push(IssueKind::AddendaSequence {
                position,
                recorded: addendum.addenda_sequence_number,
            });
        }
        if addendum.entry_detail_sequence_number != detail.sequence_number() {
            issues.push(IssueKind::AddendaEntrySequence {
                recorded: addendum.entry_detail_sequence_number,
                expected: detail.sequence_number(),
            });
        }
    }
    issues
}

/// A notification of change has a zero amount, a return transaction code,
/// and one type-98 addendum (a notification or a refusal) with its own trace
/// number.
fn noc_issues(entry: &Entry, issues: &mut Vec<IssueKind>) {
    let detail = &entry.detail;
    if detail.amount != 0 {
        issues.push(IssueKind::NocAmount {
            amount: detail.amount,
        });
    }
    if !detail.transaction_code.is_return() {
        issues.push(IssueKind::NocTransactionCode {
            transaction_code: detail.transaction_code,
        });
    }
    match &entry.special_addendum {
        None => issues.push(IssueKind::MissingNocAddendum),
        Some(addendum) if addendum.is_return() => {
            issues.push(IssueKind::UnexpectedReturnAddendum {
                transaction_code: detail.transaction_code,
            });
        }
        Some(addendum) => {
            if addendum.trace_number() != detail.trace_number {
                issues.push(IssueKind::NocTraceNumber {
                    recorded: addendum.trace_number(),
                    expected: detail.trace_number,
                });
            }
        }
    }
}

/// A return entry carries exactly one return addendum and nothing else.
fn return_issues(entry: &Entry, issues: &mut Vec<IssueKind>) {
    if !entry.addenda.is_empty() {
        issues.push(IssueKind::ReturnEntryAddenda {
            count: entry.addenda.len(),
        });
    }
    let Some(addendum) = entry.special_addendum.as_ref().filter(|a| a.is_return()) else {
        issues.push(IssueKind::MissingReturnAddendum);
        return;
    };
    if addendum.trace_number() != entry.detail.trace_number {
        issues.push(IssueKind::ReturnTraceNumber {
            recorded: addendum.trace_number(),
            expected: entry.detail.trace_number,
        });
    }
    match addendum {
        SpecialAddendum::Return(addendum) => {
            let code = addendum.return_reason_code;
            if addendum.date_of_death.is_some() && !code.allows_date_of_death() {
                issues.push(IssueKind::DateOfDeath {
                    return_reason_code: code,
                });
            }
        }
        SpecialAddendum::Dishonored(addendum) => {
            if addendum.return_reason().is_none() {
                issues.push(IssueKind::UnknownReturnReasonCode {
                    field: "return_reason_code",
                    code: addendum.return_reason_code,
                });
            }
        }
        SpecialAddendum::Contested(addendum) => {
            if addendum.return_reason().is_none() {
                issues.push(IssueKind::UnknownReturnReasonCode {
                    field: "return_reason_code",
                    code: addendum.return_reason_code,
                });
            }
            if addendum.dishonored_return_reason().is_none() {
                issues.push(IssueKind::UnknownReturnReasonCode {
                    field: "dishonored_return_reason_code",
                    code: addendum.dishonored_return_reason_code,
                });
            }
        }
        // Not returns; filtered out above.
        SpecialAddendum::Noc(_) | SpecialAddendum::RefusedNoc(_) => {}
    }
}

/// A zero-dollar remittance entry moves no money, carries its remittance
/// data in type-05 addenda, and is allowed only by some entry classes.
fn zero_dollar_issues(sec: StandardEntryClass, entry: &Entry, issues: &mut Vec<IssueKind>) {
    let code = entry.detail.transaction_code;
    if entry.detail.amount != 0 {
        issues.push(IssueKind::ZeroDollarAmount {
            amount: entry.detail.amount,
        });
    }
    if !sec.allows_zero_dollar_remittance(code) {
        issues.push(IssueKind::ZeroDollarEntryClass {
            standard_entry_class: sec,
            transaction_code: code,
        });
    }
    if entry.addenda.is_empty() {
        issues.push(IssueKind::ZeroDollarAddenda);
    }
}

/// The type-05 addenda limits of the entry class.
fn addenda_limit_issues(sec: StandardEntryClass, entry: &Entry, issues: &mut Vec<IssueKind>) {
    let count = entry.addenda.len();
    let max = sec.max_addenda();
    if count > usize::from(max) {
        issues.push(IssueKind::TooManyAddenda {
            standard_entry_class: sec,
            max,
            count,
        });
    }
    let min = sec.min_addenda();
    if count < usize::from(min) {
        issues.push(IssueKind::TooFewAddenda {
            standard_entry_class: sec,
            min,
            count,
        });
    }
}
