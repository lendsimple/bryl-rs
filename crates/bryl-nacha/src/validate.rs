//! NACHA rules checked when writing entries and when validating files.

use std::fmt;

use thiserror::Error;

use crate::codes::{REVERSAL, ServiceClassCode, StandardEntryClass, TransactionCode};
use crate::entry::Entry;
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
    let max = batch.standard_entry_class.max_addenda();
    if entry.addenda.len() > usize::from(max) {
        issues.push(IssueKind::TooManyAddenda {
            standard_entry_class: batch.standard_entry_class,
            max,
            count: entry.addenda.len(),
        });
    }
    let min = batch.standard_entry_class.min_addenda();
    if entry.addenda.len() < usize::from(min) {
        issues.push(IssueKind::TooFewAddenda {
            standard_entry_class: batch.standard_entry_class,
            min,
            count: entry.addenda.len(),
        });
    }
    if detail.addenda_record_indicator != u8::from(!entry.addenda.is_empty()) {
        issues.push(IssueKind::AddendaIndicator {
            recorded: detail.addenda_record_indicator,
            count: entry.addenda.len(),
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
