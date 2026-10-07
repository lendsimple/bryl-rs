//! Reader, `File` and validation.
//!
//! | `test_nacha.py`             | Here |
//! |-----------------------------|------|
//! | `TestReader::*`             | `structured::*`, `flat::*` |
//! | `TestWriterRoundtrip::*`    | `structured::*` |
//! | `TestMalformedError::*`     | `errors::*` (and `bryl`'s reader tests) |
//!
//! Plus `File::validate` (new) and reading the Python-generated files.

mod common;

use bryl::read::{Location, ReadErrorKind};
use common::*;
use nacha::{
    BatchHeader, EntryDetail, File, FileControl, FileHeader, Issue, IssueKind, NachaRecord, Reader,
    TransactionCode,
};

fn golden(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/golden/{name}.ach",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

mod structured {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn file_header() {
        let output = sample_file();
        let header = Reader::new(output.as_bytes()).file_header().unwrap();
        assert_eq!(header.immediate_destination_name, "DEST BANK");
        assert_eq!(header.immediate_destination, routing());
    }

    #[test]
    fn batches() {
        let output = sample_file();
        let mut reader = Reader::new(output.as_bytes());
        reader.file_header().unwrap();
        let mut headers: Vec<BatchHeader> = Vec::new();
        while let Some(header) = reader.next_batch().unwrap() {
            assert_eq!(reader.entries().count(), 1);
            reader.batch_control().unwrap();
            headers.push(header);
        }
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].company_name, "ACME CORP");
    }

    #[test]
    fn entries() {
        let output = write_sample_file(3, 100, TransactionCode::CheckingCredit, &[]);
        let mut reader = Reader::new(output.as_bytes());
        reader.file_header().unwrap();
        reader.next_batch().unwrap().unwrap();
        let entries: Vec<_> = reader.entries().collect::<Result<_, _>>().unwrap();
        assert_eq!(entries.len(), 3);
        assert!(
            entries
                .iter()
                .all(|e| e.detail.individual_name == "JANE SMITH")
        );
        let control = reader.batch_control().unwrap();
        assert_eq!(control.entry_addenda_count, 3);
    }

    #[test]
    fn full_structured_read() {
        let output = write_sample_file(2, 3000, TransactionCode::CheckingCredit, &["MEMO LINE"]);
        let mut reader = Reader::new(output.as_bytes());
        let header = reader.file_header().unwrap();
        assert_eq!(header.immediate_origin_name, "ORIGIN BANK");
        let mut batches = 0;
        let mut entries = 0;
        while reader.next_batch().unwrap().is_some() {
            batches += 1;
            for entry in reader.entries() {
                let entry = entry.unwrap();
                entries += 1;
                assert_eq!(entry.addenda.len(), 1);
                assert_eq!(entry.addenda[0].payment_related_information, "MEMO LINE");
            }
            reader.batch_control().unwrap();
        }
        let control = reader.file_control().unwrap();
        assert_eq!((batches, entries), (1, 2));
        assert_eq!(control.entry_addenda_count, 4);
        assert_eq!(reader.finish().unwrap(), 2, "filler lines");
    }

    #[test]
    fn read_file() {
        let output = write_sample_file(2, 3000, TransactionCode::CheckingCredit, &["MEMO LINE"]);
        let file = File::read(output.as_bytes()).unwrap();
        assert_eq!(file.batches.len(), 1);
        assert_eq!(file.batches[0].entries.len(), 2);
        assert_eq!(file.filler_count, 2);
        assert_eq!(file.record_count(), 8);
        assert_eq!(file.validate(), []);
    }

    #[test]
    fn crlf_lines() {
        let output = sample_file().replace('\n', "\r\n");
        assert_eq!(File::read(output.as_bytes()).unwrap().validate(), []);
    }
}

mod flat {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn iterate_all_types() {
        let output = sample_file();
        let names: Vec<_> = Reader::new(output.as_bytes())
            .map(|r| bryl::read::Dispatch::type_name(&r.unwrap()))
            .collect();
        assert_eq!(
            &names[..5],
            [
                "FileHeader",
                "BatchHeader",
                "EntryDetail",
                "BatchControl",
                "FileControl"
            ]
        );
        assert!(names[5..].iter().all(|name| *name == "Filler"));
    }

    #[test]
    fn filter_entry_details() {
        let output = write_sample_file(2, 100, TransactionCode::CheckingCredit, &[]);
        let details: Vec<EntryDetail> = Reader::new(output.as_bytes())
            .filter_map(|r| match r.unwrap() {
                NachaRecord::EntryDetail(detail) => Some(detail),
                _ => None,
            })
            .collect();
        assert_eq!(details.len(), 2);
    }
}

mod errors {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn malformed_record() {
        let input = format!("XINVALID{}\n", " ".repeat(86));
        let err = Reader::new(input.as_bytes()).next().unwrap().unwrap_err();
        assert!(matches!(err.kind, ReadErrorKind::UnknownRecord(_)));
        assert_eq!(err.location, Location::Line(1));
    }

    #[test]
    fn missing_batch_control() {
        let output = sample_file();
        let without_control: String = output
            .lines()
            .filter(|line| !line.starts_with('8'))
            .map(|line| line.to_owned() + "\n")
            .collect();
        let err = File::read(without_control.as_bytes()).unwrap_err();
        assert!(matches!(
            err.kind,
            ReadErrorKind::UnexpectedRecord {
                found: "FileControl",
                ..
            }
        ));
        assert_eq!(err.location, Location::Line(4));
        assert!(err.to_string().starts_with("<memory> @ line 4 - "), "{err}");
    }

    #[test]
    fn named_input() {
        let err = Reader::new(&b""[..])
            .with_name("ach.txt")
            .file_header()
            .unwrap_err();
        assert!(err.to_string().starts_with("ach.txt @ "), "{err}");
        assert!(matches!(err.kind, ReadErrorKind::UnexpectedEof { .. }));
    }

    #[test]
    fn records_after_file_control() {
        let mut output = sample_file();
        output.push_str(&sample_file());
        let err = File::read(output.as_bytes()).unwrap_err();
        assert!(matches!(
            err.kind,
            ReadErrorKind::UnexpectedRecord {
                found: "FileHeader",
                ..
            }
        ));
    }

    #[test]
    fn wrong_line_length() {
        let output = sample_file().replacen("DEST BANK ", "DEST BANK", 1);
        let err = File::read(output.as_bytes()).unwrap_err();
        assert!(matches!(err.kind, ReadErrorKind::Record(_)), "{err}");
    }
}

mod validate {
    use super::*;
    use pretty_assertions::assert_eq;

    fn file() -> File {
        File::read(write_sample_file(2, 100, TransactionCode::CheckingCredit, &["MEMO"]).as_bytes())
            .unwrap()
    }

    fn kinds(file: &File) -> Vec<IssueKind> {
        file.validate()
            .into_iter()
            .map(|issue| issue.kind)
            .collect()
    }

    #[test]
    fn control_totals() {
        let mut file = file();
        file.batches[0].control.entry_hash += 1;
        file.control = FileControl {
            total_credit_amount: 1,
            ..file.control.clone()
        };
        let issues = file.validate();
        assert_eq!(
            issues,
            [
                Issue {
                    batch_number: Some(1),
                    trace_number: None,
                    kind: IssueKind::ControlMismatch {
                        field: "entry_hash",
                        computed: 18_200_002,
                        recorded: 18_200_003
                    }
                },
                Issue {
                    batch_number: None,
                    trace_number: None,
                    kind: IssueKind::ControlMismatch {
                        field: "total_credit_amount",
                        computed: 200,
                        recorded: 1
                    }
                },
            ]
        );
        assert_eq!(
            issues[0].to_string(),
            "batch 1: entry_hash is 18200003, computed 18200002"
        );
    }

    #[test]
    fn counts() {
        let mut file = file();
        file.control.batch_count = 2;
        file.control.block_count = 3;
        file.batches[0].control.entry_addenda_count = 3;
        let kinds = kinds(&file);
        assert!(kinds.contains(&IssueKind::ControlMismatch {
            field: "batch_count",
            computed: 1,
            recorded: 2
        }));
        assert!(kinds.contains(&IssueKind::ControlMismatch {
            field: "block_count",
            computed: 1,
            recorded: 3
        }));
        assert!(kinds.contains(&IssueKind::ControlMismatch {
            field: "entry_addenda_count",
            computed: 4,
            recorded: 3
        }));
    }

    #[test]
    fn header_control_agreement() {
        let mut file = file();
        file.batches[0].control.company_id = "OTHER".into();
        file.batches[0].control.batch_number = 9;
        assert_eq!(
            kinds(&file),
            [
                IssueKind::HeaderMismatch {
                    field: "company_id"
                },
                IssueKind::HeaderMismatch {
                    field: "batch_number"
                }
            ]
        );
    }

    #[test]
    fn entry_rules() {
        let mut file = file();
        let entry = &mut file.batches[0].entries[1];
        entry.detail.addenda_record_indicator = 0;
        entry.addenda[0].addenda_sequence_number = 0;
        entry.addenda[0].entry_detail_sequence_number = 7;
        let issues = file.validate();
        assert!(
            issues
                .iter()
                .all(|i| i.trace_number == Some(123_456_780_000_002))
        );
        assert_eq!(
            issues.into_iter().map(|i| i.kind).collect::<Vec<_>>(),
            [
                IssueKind::AddendaIndicator {
                    recorded: 0,
                    count: 1
                },
                IssueKind::AddendaSequence {
                    position: 1,
                    recorded: 0
                },
                IssueKind::AddendaEntrySequence {
                    recorded: 7,
                    expected: 2
                },
            ]
        );
    }

    #[test]
    fn trace_order_and_duplicates() {
        let mut file = file();
        let first = file.batches[0].entries[0].detail.trace_number;
        file.batches[0].entries[1].detail.trace_number = first;
        file.batches[0].entries[1].addenda[0].entry_detail_sequence_number = 1;
        assert_eq!(
            kinds(&file),
            [IssueKind::TraceOrder, IssueKind::DuplicateTrace]
        );
    }

    #[test]
    fn batch_number_order() {
        let mut file =
            File::read(write_sample_file(1, 100, TransactionCode::CheckingCredit, &[]).as_bytes())
                .unwrap();
        let mut second = file.batches[0].clone();
        second.header.batch_number = 1;
        second.control.batch_number = 1;
        second.entries[0].detail.trace_number += 1;
        file.batches.push(second);
        file.control.batch_count = 2;
        let issues: Vec<_> = file
            .validate()
            .into_iter()
            .filter(|i| matches!(i.kind, IssueKind::BatchNumberOrder { .. }))
            .collect();
        assert_eq!(
            issues,
            [Issue {
                batch_number: Some(1),
                trace_number: None,
                kind: IssueKind::BatchNumberOrder { previous: 1 }
            }]
        );
    }

    #[test]
    fn batch_and_entry_class_rules() {
        let mut file =
            File::read(write_sample_file(1, 100, TransactionCode::CheckingCredit, &[]).as_bytes())
                .unwrap();
        file.batches[0].header.standard_entry_class = nacha::StandardEntryClass::Enr;
        assert_eq!(
            kinds(&file),
            [
                IssueKind::EntryDescription {
                    standard_entry_class: nacha::StandardEntryClass::Enr,
                    expected: "AUTOENROLL",
                    found: "PAYROLL".into()
                },
                IssueKind::TooFewAddenda {
                    standard_entry_class: nacha::StandardEntryClass::Enr,
                    min: 1,
                    count: 0
                }
            ]
        );
        file.batches[0].header.standard_entry_class = nacha::StandardEntryClass::Tel;
        assert_eq!(
            kinds(&file),
            [IssueKind::EntryClassDirection {
                standard_entry_class: nacha::StandardEntryClass::Tel,
                transaction_code: TransactionCode::CheckingCredit
            }]
        );
    }

    #[test]
    fn block_padding() {
        let mut file = file();
        file.filler_count = 3;
        assert_eq!(
            kinds(&file),
            [IssueKind::BlockPadding {
                lines: 8,
                filler: 3
            }]
        );
    }
}

mod python_files {
    //! Files written by lms-python read correctly; `validate` reports exactly
    //! the Python quirks fixed in DEVIATIONS.md.
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn single_entry() {
        let file = File::read(golden("single_entry").as_bytes()).unwrap();
        assert_eq!(file.header.immediate_origin, "9876543210");
        // N14: Python right-aligned the account number. Decoding keeps the
        // field's bytes, so the leading padding survives.
        assert_eq!(
            file.batches[0].entries[0]
                .detail
                .receiving_dfi_account_number,
            "        123456789"
        );
        // N3: no filler lines.
        assert_eq!(
            file.validate()
                .into_iter()
                .map(|i| i.kind)
                .collect::<Vec<_>>(),
            [IssueKind::BlockPadding {
                lines: 5,
                filler: 0
            }]
        );
    }

    #[test]
    fn entries_with_addenda() {
        let file = File::read(golden("entries_with_addenda").as_bytes()).unwrap();
        let kinds: Vec<_> = file.validate().into_iter().map(|i| i.kind).collect();
        // N1: addenda numbered from 0.
        assert_eq!(
            kinds,
            [
                IssueKind::AddendaSequence {
                    position: 1,
                    recorded: 0
                },
                IssueKind::AddendaSequence {
                    position: 1,
                    recorded: 0
                },
                IssueKind::BlockPadding {
                    lines: 8,
                    filler: 0
                },
            ]
        );
    }

    #[test]
    fn two_batches() {
        let file = File::read(golden("two_batches").as_bytes()).unwrap();
        assert_eq!(file.batches.len(), 2);
        // N2: the second batch reuses trace numbers 1 and 2.
        let issues = file.validate();
        let duplicates: Vec<_> = issues
            .iter()
            .filter(|i| i.kind == IssueKind::DuplicateTrace)
            .map(|i| (i.batch_number, i.trace_number))
            .collect();
        assert_eq!(
            duplicates,
            [
                (Some(2), Some(123_456_780_000_001)),
                (Some(2), Some(123_456_780_000_002))
            ]
        );
    }

    #[test]
    fn header_fields() {
        let header: FileHeader = Reader::new(golden("single_entry").as_bytes())
            .file_header()
            .unwrap();
        assert_eq!(header.file_creation(), sample_datetime());
    }
}
