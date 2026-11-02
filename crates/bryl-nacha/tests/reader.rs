//! Reading: structured reading and `File::read` (`structured`), flat
//! iteration (`flat`), read errors (`errors`), `File::validate`
//! (`validate`), and lenient reading (`lenient`).

mod common;

use bryl::read::{Location, ReadErrorKind};
use common::*;
use nacha::{
    BatchHeader, EntryDetail, File, FileControl, Issue, IssueKind, NachaRecord, Reader,
    ReturnReasonCode, SpecialAddendum, StandardEntryClass, TransactionCode,
};

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
    fn second_return_addendum() {
        let output = return_file();
        let mut lines: Vec<&str> = output.lines().collect();
        lines.insert(4, lines[3]); // repeat the return addendum
        let input = lines.join("\n");
        let err = File::read(input.as_bytes()).unwrap_err();
        assert!(matches!(
            err.kind,
            ReadErrorKind::UnexpectedRecord {
                found: "ReturnAddendum",
                ..
            }
        ));
        assert_eq!(err.location, Location::Line(5));
    }

    #[test]
    fn second_notification_of_change_addendum() {
        let output = noc_file();
        let mut lines: Vec<&str> = output.lines().collect();
        lines.insert(4, lines[3]); // repeat the type-98 addendum
        let input = lines.join("\n");
        let err = File::read(input.as_bytes()).unwrap_err();
        assert!(matches!(
            err.kind,
            ReadErrorKind::UnexpectedRecord {
                found: "NocAddendum",
                ..
            }
        ));
        assert_eq!(err.location, Location::Line(5));
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
    fn return_entries() {
        let file = File::read(return_file().as_bytes()).unwrap();
        let entry = &file.batches[0].entries[0];
        let addendum = entry.return_addendum().unwrap();
        assert!(entry.is_rejection());
        assert_eq!(
            addendum.return_reason_code,
            ReturnReasonCode::InsufficientFunds
        );
        assert_eq!(file.validate(), []);
    }

    #[test]
    fn return_rules() {
        let mut file = File::read(return_file().as_bytes()).unwrap();
        let entry = &mut file.batches[0].entries[0];
        let trace = entry.detail.trace_number;
        let Some(SpecialAddendum::Return(addendum)) = &mut entry.special_addendum else {
            panic!("expected a return addendum");
        };
        addendum.trace_number = 1;
        assert_eq!(
            kinds(&file),
            [IssueKind::ReturnTraceNumber {
                recorded: 1,
                expected: trace
            }]
        );
        let entry = &mut file.batches[0].entries[0];
        entry.special_addendum = None;
        entry.detail.addenda_record_indicator = 0;
        // The control totals still count the removed addendum.
        assert_eq!(kinds(&file)[0], IssueKind::MissingReturnAddendum,);
    }

    #[test]
    fn dishonored_return_rules() {
        let mut file = File::read(dishonored_file().as_bytes()).unwrap();
        assert_eq!(kinds(&file), []);
        let entry = &mut file.batches[0].entries[0];
        let trace = entry.detail.trace_number;
        let Some(SpecialAddendum::Dishonored(addendum)) = &mut entry.special_addendum else {
            panic!("expected a dishonored return addendum");
        };
        addendum.trace_number = 1;
        assert_eq!(
            kinds(&file),
            [IssueKind::ReturnTraceNumber {
                recorded: 1,
                expected: trace
            }]
        );
    }

    #[test]
    fn notification_of_change_rules() {
        let mut file = File::read(noc_file().as_bytes()).unwrap();
        assert_eq!(kinds(&file), []);
        let entry = &mut file.batches[0].entries[0];
        let trace = entry.detail.trace_number;
        let Some(SpecialAddendum::Noc(addendum)) = &mut entry.special_addendum else {
            panic!("expected a notification of change addendum");
        };
        addendum.trace_number = 1;
        assert_eq!(
            kinds(&file),
            [IssueKind::NocTraceNumber {
                recorded: 1,
                expected: trace
            }]
        );
        // The same entry in a PPD batch.
        let mut file = File::read(noc_file().as_bytes()).unwrap();
        file.batches[0].header.standard_entry_class = StandardEntryClass::Ppd;
        assert_eq!(
            kinds(&file),
            [
                IssueKind::UnexpectedNocAddendum {
                    standard_entry_class: StandardEntryClass::Ppd
                },
                IssueKind::MissingReturnAddendum,
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

mod lenient {
    use super::*;
    use nacha::{ServiceClassCode, Writer};
    use pretty_assertions::assert_eq;

    /// Two credits and an R01 return in one batch (lines 3, 4, and 5-6).
    fn three_entries() -> String {
        let mut writer = Writer::new(Vec::new());
        let mut file = writer.begin_file(file_params()).unwrap();
        let mut batch = file
            .begin_batch(batch_params(ServiceClassCode::MixedDebitsAndCredits))
            .unwrap();
        for amount in [1000, 2000] {
            batch
                .entry(entry_params(TransactionCode::CheckingCredit, amount, &[]))
                .unwrap();
        }
        batch
            .entry(return_entry_params(ReturnReasonCode::InsufficientFunds))
            .unwrap();
        batch.finish().unwrap();
        file.finish().unwrap();
        String::from_utf8(writer.into_inner()).unwrap()
    }

    /// Entry 2's routing number gets a bad check digit (091000019 ->
    /// 091000018) and entry 3's return an unknown reason code (R01 -> R98).
    fn damaged() -> String {
        let output = three_entries();
        let mut lines: Vec<String> = output.lines().map(str::to_owned).collect();
        assert!(lines[3].starts_with("622091000019"));
        lines[3].replace_range(3..12, "091000018");
        assert!(lines[5].starts_with("799R01"));
        lines[5].replace_range(3..6, "R98");
        lines.join("\n") + "\n"
    }

    #[test]
    fn strict_reading_fails() {
        let err = File::read(damaged().as_bytes()).unwrap_err();
        assert_eq!(err.location, Location::Line(4));
    }

    #[test]
    fn lenient_reading_keeps_the_other_entries() {
        let input = damaged();
        let file = File::read_lenient(input.as_bytes()).unwrap();
        let batch = &file.batches[0];
        assert_eq!(batch.entries.len(), 1);
        assert_eq!(batch.entries[0].detail.amount, 1000);

        let [bad_routing, bad_code] = &batch.unread_entries[..] else {
            panic!(
                "expected two unread entries, got {:?}",
                batch.unread_entries
            );
        };
        let lines: Vec<&str> = input.lines().collect();
        assert_eq!(bad_routing.location, Location::Line(4));
        assert_eq!(bad_routing.error_location, Location::Line(4));
        assert_eq!(bad_routing.lines, [lines[3].as_bytes()]);
        assert!(bad_routing.error.field_kind().is_some());
        assert_eq!(bad_routing.detail_field::<u64>("amount"), Ok(2000));

        // The detail decodes but its return addendum does not; the entry is
        // kept whole.
        assert_eq!(bad_code.location, Location::Line(5));
        assert_eq!(bad_code.error_location, Location::Line(6));
        assert_eq!(bad_code.lines, [lines[4].as_bytes(), lines[5].as_bytes()]);
        assert_eq!(
            bad_code.detail_field::<TransactionCode>("transaction_code"),
            Ok(TransactionCode::CheckingReturnedDebit)
        );

        let traces: Vec<u64> = [bad_routing, bad_code]
            .iter()
            .map(|unread| unread.detail_field("trace_number").unwrap())
            .collect();
        assert_eq!(
            file.validate(),
            [
                Issue {
                    batch_number: Some(1),
                    trace_number: Some(traces[0]),
                    kind: IssueKind::UnreadEntry {
                        location: Location::Line(4),
                        error: bad_routing.error.clone(),
                    },
                },
                Issue {
                    batch_number: Some(1),
                    trace_number: Some(traces[1]),
                    kind: IssueKind::UnreadEntry {
                        location: Location::Line(6),
                        error: bad_code.error.clone(),
                    },
                },
            ]
        );
    }

    #[test]
    fn readable_files_read_the_same() {
        let output = three_entries();
        let strict = File::read(output.as_bytes()).unwrap();
        assert_eq!(File::read_lenient(output.as_bytes()).unwrap(), strict);
        assert_eq!(strict.batches[0].unread_entries, []);
    }

    #[test]
    fn bad_field_of_an_unread_entry_reports_its_error() {
        let file = File::read_lenient(damaged().as_bytes()).unwrap();
        let unread = &file.batches[0].unread_entries[0];
        let err = unread
            .detail_field::<nacha::RoutingNumber>("receiving_dfi")
            .unwrap_err();
        assert_eq!(&err, &unread.error);
    }

    #[test]
    fn structural_problems_still_fail() {
        let output = three_entries();
        let mut lines: Vec<String> = output.lines().map(str::to_owned).collect();
        // A bad routing number in the file header.
        lines[0].replace_range(3..13, " 091000018");
        assert!(File::read_lenient(lines.join("\n").as_bytes()).is_err());

        let mut lines: Vec<String> = output.lines().map(str::to_owned).collect();
        // A batch control that is not a number.
        lines[6].replace_range(4..10, "XXXXXX");
        let err = File::read_lenient(lines.join("\n").as_bytes()).unwrap_err();
        assert_eq!(err.location, Location::Line(7));

        let mut lines: Vec<String> = output.lines().map(str::to_owned).collect();
        // An unknown record type.
        lines[3].replace_range(0..1, "4");
        let err = File::read_lenient(lines.join("\n").as_bytes()).unwrap_err();
        assert!(matches!(err.kind, ReadErrorKind::UnknownRecord(_)));
    }
}
