//! A whole NACHA file, for parsing and validation.

use std::collections::HashSet;
use std::io::BufRead;

use bryl::read::ReadError;

use crate::entry::Entry;
use crate::reader::Reader;
use crate::records::{BatchControl, BatchHeader, FileControl, FileHeader};
use crate::totals::Totals;
use crate::validate::{Issue, IssueKind, batch_issues, entry_issues};

/// A parsed NACHA file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    /// The file header.
    pub header: FileHeader,
    /// Batches, in order.
    pub batches: Vec<Batch>,
    /// The file control.
    pub control: FileControl,
    /// Filler lines after the file control.
    pub filler_count: usize,
}

/// A batch: header, entries and control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    /// The batch header.
    pub header: BatchHeader,
    /// Entries with their addenda.
    pub entries: Vec<Entry>,
    /// The batch control.
    pub control: BatchControl,
}

impl File {
    /// Parses a whole file.
    ///
    /// Only the structure is checked here; use [`File::validate`] for control
    /// totals and other rules.
    ///
    /// # Errors
    ///
    /// Returns the first record that is missing, out of place or invalid.
    pub fn read<R: BufRead>(input: R) -> Result<Self, ReadError> {
        Reader::new(input).read_file()
    }

    /// Records in the file, excluding filler lines.
    pub fn record_count(&self) -> usize {
        2 + self
            .batches
            .iter()
            .map(|batch| {
                2 + batch
                    .entries
                    .iter()
                    .map(|entry| 1 + entry.addenda.len())
                    .sum::<usize>()
            })
            .sum::<usize>()
    }

    /// Checks control totals, counts, batch header/control agreement, batch
    /// and entry rules, batch number and trace number order, and block
    /// padding. Returns every problem found.
    pub fn validate(&self) -> Vec<Issue> {
        let mut issues = Vec::new();
        let mut file_totals = Totals::default();
        let mut traces = HashSet::new();
        let mut previous_batch_number = None;
        for batch in &self.batches {
            let number = batch.header.batch_number;
            if let Some(previous) = previous_batch_number.filter(|&previous| number <= previous) {
                issues.push(Issue {
                    batch_number: Some(number),
                    trace_number: None,
                    kind: IssueKind::BatchNumberOrder { previous },
                });
            }
            previous_batch_number = Some(number);
            let batch_totals = batch.validate(&mut traces, &mut issues);
            file_totals.add(&batch_totals);
        }

        let file_issue = |kind| Issue {
            batch_number: None,
            trace_number: None,
            kind,
        };
        let records = self.record_count();
        let counts = [
            (
                "batch_count",
                self.batches.len() as u64,
                u64::from(self.control.batch_count),
            ),
            (
                "block_count",
                records.div_ceil(10) as u64,
                u64::from(self.control.block_count),
            ),
        ];
        for (field, computed, recorded) in counts.into_iter().chain(control_fields(
            &file_totals,
            u64::from(self.control.entry_addenda_count),
            self.control.entry_hash,
            self.control.total_debit_amount,
            self.control.total_credit_amount,
        )) {
            if computed != recorded {
                issues.push(file_issue(IssueKind::ControlMismatch {
                    field,
                    computed,
                    recorded,
                }));
            }
        }
        if (records + self.filler_count) % 10 != 0 || self.filler_count >= 10 {
            issues.push(file_issue(IssueKind::BlockPadding {
                lines: records,
                filler: self.filler_count,
            }));
        }
        issues
    }
}

impl Batch {
    /// Adds this batch's issues and returns its computed totals.
    fn validate(&self, traces: &mut HashSet<u64>, issues: &mut Vec<Issue>) -> Totals {
        let batch_number = Some(self.header.batch_number);
        issues.extend(batch_issues(&self.header).into_iter().map(|kind| Issue {
            batch_number,
            trace_number: None,
            kind,
        }));
        let mut totals = Totals::default();
        let mut previous_trace = None;
        for entry in &self.entries {
            let trace = entry.detail.trace_number;
            let entry_issue = |kind| Issue {
                batch_number,
                trace_number: Some(trace),
                kind,
            };
            issues.extend(
                entry_issues(&self.header, entry)
                    .into_iter()
                    .map(entry_issue),
            );
            if previous_trace.is_some_and(|previous| trace <= previous) {
                issues.push(entry_issue(IssueKind::TraceOrder));
            }
            if !traces.insert(trace) {
                issues.push(entry_issue(IssueKind::DuplicateTrace));
            }
            previous_trace = Some(trace);
            totals.add_entry(entry);
        }

        let batch_issue = |kind| Issue {
            batch_number,
            trace_number: None,
            kind,
        };
        let (header, control) = (&self.header, &self.control);
        let agreement = [
            (
                "service_class_code",
                header.service_class_code == control.service_class_code,
            ),
            ("company_id", header.company_id == control.company_id),
            (
                "originating_dfi_id",
                header.originating_dfi_id == control.originating_dfi_id,
            ),
            ("batch_number", header.batch_number == control.batch_number),
        ];
        for (field, agrees) in agreement {
            if !agrees {
                issues.push(batch_issue(IssueKind::HeaderMismatch { field }));
            }
        }
        for (field, computed, recorded) in control_fields(
            &totals,
            u64::from(control.entry_addenda_count),
            control.entry_hash,
            control.total_debit_amount,
            control.total_credit_amount,
        ) {
            if computed != recorded {
                issues.push(batch_issue(IssueKind::ControlMismatch {
                    field,
                    computed,
                    recorded,
                }));
            }
        }
        totals
    }
}

/// `(field, computed, recorded)` for the four totals shared by batch and file
/// control records.
fn control_fields(
    computed: &Totals,
    entry_addenda_count: u64,
    entry_hash: u64,
    total_debit_amount: u64,
    total_credit_amount: u64,
) -> [(&'static str, u64, u64); 4] {
    [
        (
            "entry_addenda_count",
            u64::from(computed.entry_addenda_count),
            entry_addenda_count,
        ),
        ("entry_hash", computed.entry_hash, entry_hash),
        (
            "total_debit_amount",
            computed.total_debit_amount,
            total_debit_amount,
        ),
        (
            "total_credit_amount",
            computed.total_credit_amount,
            total_credit_amount,
        ),
    ]
}
