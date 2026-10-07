//! A whole Metro 2 file, for parsing and validation.

use std::fmt;
use std::io::BufRead;

use bryl::read::ReadError;
use bryl::{FieldKind, Record};

use crate::data_record::DataRecord;
use crate::reader::Reader;
use crate::records::{HeaderRecord, TrailerRecord};
use crate::validate::Violation;

/// A parsed Metro 2 file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    /// The header record.
    pub header: HeaderRecord,
    /// Data records, in order.
    pub data_records: Vec<DataRecord>,
    /// The trailer record.
    pub trailer: TrailerRecord,
}

/// A problem found by [`File::validate`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Issue {
    /// A trailer total does not match the data records.
    TrailerMismatch {
        /// Trailer field name.
        field: &'static str,
        /// Value computed from the data records.
        computed: u64,
        /// Value in the trailer.
        recorded: u64,
    },
    /// A data record's base segment breaks a rule.
    Violation {
        /// Index of the data record.
        index: usize,
        /// Consumer account number.
        account: String,
        /// The rule broken.
        violation: Violation,
    },
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TrailerMismatch {
                field,
                computed,
                recorded,
            } => write!(f, "trailer {field} is {recorded}, computed {computed}"),
            Self::Violation {
                index,
                account,
                violation,
            } => write!(f, "data record {index} (account {account}): {violation}"),
        }
    }
}

impl File {
    /// Parses a whole RDW-framed file. Use [`Reader::newline`] and
    /// [`Reader::read_file`] for newline-delimited files.
    ///
    /// # Errors
    ///
    /// Returns the first record that is missing, out of place or invalid.
    pub fn read<R: BufRead>(input: R) -> Result<Self, ReadError> {
        Reader::new(input).read_file()
    }

    /// Recomputes the trailer and checks each base segment's rules.
    pub fn validate(&self) -> Vec<Issue> {
        let mut issues = Vec::new();
        for (index, record) in self.data_records.iter().enumerate() {
            if let Err(violations) = record.base.validate() {
                issues.extend(violations.into_iter().map(|violation| Issue::Violation {
                    index,
                    account: record.base.consumer_account_number.clone(),
                    violation,
                }));
            }
        }
        let computed = TrailerRecord::from_records(&self.data_records);
        issues.extend(trailer_differences(&computed, &self.trailer));
        issues
    }
}

/// Compares two trailers field by field via their encoded form.
fn trailer_differences(computed: &TrailerRecord, recorded: &TrailerRecord) -> Vec<Issue> {
    let (Ok(computed_raw), Ok(recorded_raw)) = (computed.encode(), recorded.encode()) else {
        return Vec::new();
    };
    TrailerRecord::FIELDS
        .iter()
        .filter(|spec| matches!(spec.kind, FieldKind::Numeric { .. }) && spec.constant.is_none())
        .filter_map(|spec| {
            let value = |raw: &str| raw[spec.range()].parse::<u64>().unwrap_or_default();
            let (computed, recorded) = (value(&computed_raw), value(&recorded_raw));
            (computed != recorded).then_some(Issue::TrailerMismatch {
                field: spec.name,
                computed,
                recorded,
            })
        })
        .collect()
}
