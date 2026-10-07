//! Writer.
//!
//! | `test_nacha.py`              | Here |
//! |------------------------------|------|
//! | `TestWriter::*`              | `layout::*`, `totals::*` |
//! | `TestWriterEntryHash::*`     | `totals::entry_hash*` |
//! | `TestWriterBlockCount::*`    | `blocks::*` |
//! | `TestWriterContextErrors::*` | Compile errors now (typestate guards); see the `compile_fail` doctests on `nacha::Writer`. The bad routing number case is `records.rs::routing::bad_format`. |
//!
//! Plus the NACHA rules the writer enforces.

mod common;

use common::*;
use nacha::{
    BatchParams, EntryParams, Error, File, IssueKind, ServiceClassCode, StandardEntryClass,
    TransactionCode, Writer,
};

mod layout {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn record_order() {
        let output = sample_file();
        let types: String = record_lines(&output)
            .iter()
            .map(|line| &line[..1])
            .collect();
        assert_eq!(types, "15689");
    }

    #[test]
    fn addenda_follow_their_entry() {
        let output = write_sample_file(1, 10_000, TransactionCode::CheckingCredit, &["EXTRA INFO"]);
        let lines = record_lines(&output);
        assert_eq!(&lines[2][..1], "6");
        assert_eq!(&lines[2][78..79], "1", "addenda record indicator");
        assert_eq!(&lines[3][..1], "7");
    }

    #[test]
    fn all_lines_are_94_characters() {
        let output = write_sample_file(2, 10_000, TransactionCode::CheckingCredit, &["MEMO"]);
        for line in output.lines() {
            assert_eq!(line.len(), 94, "{line}");
        }
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn trace_numbers_are_sequential() {
        let file = File::read(
            write_sample_file(2, 10_000, TransactionCode::CheckingCredit, &[]).as_bytes(),
        )
        .unwrap();
        let traces: Vec<_> = file.batches[0]
            .entries
            .iter()
            .map(|e| e.detail.trace_number)
            .collect();
        assert_eq!(traces, [123_456_780_000_001, 123_456_780_000_002]);
    }

    #[test]
    fn trace_numbers_continue_across_batches() {
        // Python restarted at 1 in each batch, duplicating trace numbers.
        let mut writer = Writer::new(Vec::new());
        let mut file = writer.begin_file(file_params()).unwrap();
        for _ in 0..2 {
            let mut batch = file
                .begin_batch(batch_params(ServiceClassCode::CreditsOnly))
                .unwrap();
            batch
                .entry(entry_params(TransactionCode::CheckingCredit, 100, &[]))
                .unwrap();
            batch
                .entry(entry_params(TransactionCode::CheckingCredit, 100, &[]))
                .unwrap();
            batch.finish().unwrap();
        }
        file.finish().unwrap();
        let file = File::read(writer.get_ref().as_slice()).unwrap();
        let traces: Vec<_> = file
            .batches
            .iter()
            .flat_map(|b| &b.entries)
            .map(|e| e.detail.sequence_number())
            .collect();
        assert_eq!(traces, [1, 2, 3, 4]);
        assert_eq!(file.validate(), []);
    }

    #[test]
    fn explicit_trace_number() {
        let mut writer = Writer::new(Vec::new());
        let mut file = writer.begin_file(file_params()).unwrap();
        let mut batch = file
            .begin_batch(batch_params(ServiceClassCode::CreditsOnly))
            .unwrap();
        let entry = batch
            .entry(EntryParams {
                trace_number: Some(123_456_789_999_999),
                ..entry_params(TransactionCode::CheckingCredit, 100, &["A"])
            })
            .unwrap();
        assert_eq!(entry.detail.trace_number, 123_456_789_999_999);
        assert_eq!(entry.addenda[0].entry_detail_sequence_number, 9_999_999);
        batch.finish().unwrap();
        file.finish().unwrap();
    }

    #[test]
    fn addenda_sequence_starts_at_1() {
        // Python numbered addenda from 0.
        let mut writer = Writer::new(Vec::new());
        let mut file = writer.begin_file(file_params()).unwrap();
        let mut batch = file
            .begin_batch(BatchParams {
                standard_entry_class: StandardEntryClass::Ctx,
                ..batch_params(ServiceClassCode::CreditsOnly)
            })
            .unwrap();
        let entry = batch
            .entry(entry_params(
                TransactionCode::CheckingCredit,
                100,
                &["A", "B", "C"],
            ))
            .unwrap();
        let sequences: Vec<_> = entry
            .addenda
            .iter()
            .map(|a| a.addenda_sequence_number)
            .collect();
        assert_eq!(sequences, [1, 2, 3]);
        assert!(
            entry
                .addenda
                .iter()
                .all(|a| a.entry_detail_sequence_number == 1)
        );
        batch.finish().unwrap();
        file.finish().unwrap();
    }

    #[test]
    fn effective_date_defaults_to_creation_date() {
        let mut writer = Writer::new(Vec::new());
        let mut file = writer.begin_file(file_params()).unwrap();
        let batch = file
            .begin_batch(BatchParams {
                effective_entry_date: None,
                ..batch_params(ServiceClassCode::CreditsOnly)
            })
            .unwrap();
        assert_eq!(batch.header().effective_entry_date, sample_date());
        assert_eq!(batch.header().batch_number, 1);
        batch.finish().unwrap();
        file.finish().unwrap();
    }

    #[test]
    fn batch_numbers_ascend() {
        let mut writer = Writer::new(Vec::new());
        let mut file = writer.begin_file(file_params()).unwrap();
        let first = file
            .begin_batch(batch_params(ServiceClassCode::CreditsOnly))
            .unwrap();
        assert_eq!(first.header().batch_number, 1);
        drop(first); // abandoned batches still use up their number
        let second = file
            .begin_batch(batch_params(ServiceClassCode::CreditsOnly))
            .unwrap();
        assert_eq!(second.header().batch_number, 2);
        assert_eq!(second.finish().unwrap().batch_number, 2);
        assert_eq!(file.finish().unwrap().batch_count, 1);
    }

    #[test]
    fn text_is_uppercased() {
        let mut writer = Writer::new(Vec::new());
        let mut file = writer
            .begin_file(nacha::FileParams {
                immediate_destination_name: "dest bank".into(),
                ..file_params()
            })
            .unwrap();
        assert_eq!(file.header().immediate_destination_name, "dest bank");
        file.begin_batch(batch_params(ServiceClassCode::CreditsOnly))
            .unwrap()
            .finish()
            .unwrap();
        file.finish().unwrap();
        let output = String::from_utf8(writer.into_inner()).unwrap();
        assert!(output.contains("DEST BANK"));
    }

    #[test]
    fn sanitize_override() {
        let mut writer = Writer::new(Vec::new()).sanitize(bryl::Sanitize::NONE);
        let file = writer
            .begin_file(nacha::FileParams {
                immediate_destination_name: "dest bank".into(),
                ..file_params()
            })
            .unwrap();
        file.finish().unwrap();
        let output = String::from_utf8(writer.into_inner()).unwrap();
        assert!(output.contains("dest bank"));
    }
}

mod totals {
    use super::*;
    use pretty_assertions::assert_eq;

    fn read(output: &str) -> File {
        File::read(output.as_bytes()).unwrap()
    }

    #[test]
    fn batch_credit_total() {
        let file = read(&write_sample_file(
            2,
            5000,
            TransactionCode::CheckingCredit,
            &[],
        ));
        assert_eq!(file.batches[0].entries.len(), 2);
        assert_eq!(file.batches[0].control.total_credit_amount, 10_000);
    }

    #[test]
    fn file_totals() {
        let file = read(&write_sample_file(
            3,
            1000,
            TransactionCode::CheckingCredit,
            &[],
        ));
        assert_eq!(file.control.total_credit_amount, 3000);
        assert_eq!(file.control.batch_count, 1);
    }

    #[test]
    fn debit_totals() {
        let file = read(&write_sample_file(
            1,
            7500,
            TransactionCode::CheckingDebit,
            &[],
        ));
        assert_eq!(file.batches[0].control.total_debit_amount, 7500);
        assert_eq!(file.batches[0].control.total_credit_amount, 0);
        assert_eq!(file.control.total_debit_amount, 7500);
    }

    #[test]
    fn entry_addenda_counts() {
        let file = read(&write_sample_file(
            2,
            3000,
            TransactionCode::CheckingCredit,
            &["MEMO LINE"],
        ));
        assert_eq!(file.batches[0].control.entry_addenda_count, 4);
        assert_eq!(file.control.entry_addenda_count, 4);
    }

    #[test]
    fn entry_hash() {
        let file = read(&write_sample_file(
            2,
            100,
            TransactionCode::CheckingCredit,
            &[],
        ));
        let trn = u64::from(routing().trn());
        assert_eq!(file.batches[0].control.entry_hash, trn * 2);
    }

    #[test]
    fn entry_hash_keeps_ten_digits() {
        assert_eq!(nacha::HASH_MODULUS, 10_u64.pow(10));
        let mut totals = nacha::Totals {
            entry_hash: nacha::HASH_MODULUS - 1,
            ..Default::default()
        };
        totals.add(&nacha::Totals {
            entry_hash: 5,
            ..Default::default()
        });
        assert_eq!(totals.entry_hash, 4);
    }

    #[test]
    fn controls_returned_by_finish() {
        let mut writer = Writer::new(Vec::new());
        let mut file = writer.begin_file(file_params()).unwrap();
        let mut batch = file
            .begin_batch(batch_params(ServiceClassCode::MixedDebitsAndCredits))
            .unwrap();
        batch
            .entry(entry_params(TransactionCode::CheckingCredit, 300, &[]))
            .unwrap();
        batch
            .entry(entry_params(TransactionCode::CheckingDebit, 200, &[]))
            .unwrap();
        let batch_control = batch.finish().unwrap();
        let file_control = file.finish().unwrap();
        assert_eq!(batch_control.total_credit_amount, 300);
        assert_eq!(batch_control.total_debit_amount, 200);
        assert_eq!(file_control.total_credit_amount, 300);
        assert_eq!(file_control.total_debit_amount, 200);
        assert_eq!(file_control.entry_hash, batch_control.entry_hash);
    }

    #[test]
    fn written_files_validate() {
        for output in [
            sample_file(),
            write_sample_file(25, 1, TransactionCode::SavingsCredit, &["MEMO"]),
            write_sample_file(3, 0, TransactionCode::CheckingPrenoteDebit, &[]),
        ] {
            assert_eq!(read(&output).validate(), []);
        }
    }
}

mod blocks {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn block_count_one_entry() {
        // header, batch header, entry, batch control, file control = 5 → 1 block
        let file = File::read(sample_file().as_bytes()).unwrap();
        assert_eq!(file.control.block_count, 1);
    }

    #[test]
    fn block_count_ten_entries() {
        // 4 + 10 entries = 14 records → 2 blocks
        let file =
            File::read(write_sample_file(10, 100, TransactionCode::CheckingCredit, &[]).as_bytes())
                .unwrap();
        assert_eq!(file.control.block_count, 2);
    }

    #[test]
    fn padded_to_whole_blocks() {
        // Python wrote no filler lines.
        let output = write_sample_file(10, 100, TransactionCode::CheckingCredit, &[]);
        assert_eq!(output.lines().count(), 20);
        assert_eq!(record_lines(&output).len(), 14);
        assert!(output.lines().skip(14).all(|line| line == "9".repeat(94)));
    }

    #[test]
    fn exact_block_needs_no_filler() {
        // 4 + 6 entries = 10 records
        let output = write_sample_file(6, 100, TransactionCode::CheckingCredit, &[]);
        assert_eq!(output.lines().count(), 10);
        assert_eq!(record_lines(&output).len(), 10);
    }

    #[test]
    fn crlf_line_endings() {
        let mut writer = Writer::new(Vec::new()).line_ending(nacha::LineEnding::CrLf);
        let mut file = writer.begin_file(file_params()).unwrap();
        let mut batch = file
            .begin_batch(batch_params(ServiceClassCode::CreditsOnly))
            .unwrap();
        batch
            .entry(entry_params(
                TransactionCode::CheckingCredit,
                100,
                &["MEMO"],
            ))
            .unwrap();
        batch.finish().unwrap();
        file.finish().unwrap();
        let output = String::from_utf8(writer.into_inner()).unwrap();
        assert_eq!(output.matches("\r\n").count(), 10);
        assert_eq!(output.matches('\n').count(), 10);
        assert!(output.ends_with("\r\n"));
        assert_eq!(File::read(output.as_bytes()).unwrap().validate(), []);
    }

    #[test]
    fn padding_can_be_disabled() {
        let mut writer = Writer::new(Vec::new()).pad_blocks(false);
        let file = writer.begin_file(file_params()).unwrap();
        let control = file.finish().unwrap();
        assert_eq!(control.block_count, 1);
        assert_eq!(
            String::from_utf8(writer.into_inner())
                .unwrap()
                .lines()
                .count(),
            2
        );
    }
}

mod rules {
    use super::*;
    use pretty_assertions::assert_eq;

    fn try_entry(
        service_class: ServiceClassCode,
        sec: StandardEntryClass,
        params: EntryParams,
    ) -> (Result<nacha::Entry, Error>, usize) {
        let mut writer = Writer::new(Vec::new());
        let mut file = writer.begin_file(file_params()).unwrap();
        let mut batch = file
            .begin_batch(BatchParams {
                standard_entry_class: sec,
                ..batch_params(service_class)
            })
            .unwrap();
        let result = batch.entry(params);
        drop(batch);
        drop(file);
        let written = writer.into_inner().len();
        (result, written)
    }

    fn rejected(
        service_class: ServiceClassCode,
        sec: StandardEntryClass,
        params: EntryParams,
    ) -> IssueKind {
        let (result, written) = try_entry(service_class, sec, params);
        // Nothing beyond the two headers was written (atomic entries).
        assert_eq!(written, 2 * 95);
        match result {
            Err(Error::InvalidEntry(kind)) => kind,
            other => panic!("expected InvalidEntry, got {other:?}"),
        }
    }

    #[test]
    fn credits_only_rejects_debits() {
        // N5
        let kind = rejected(
            ServiceClassCode::CreditsOnly,
            StandardEntryClass::Ppd,
            entry_params(TransactionCode::CheckingDebit, 100, &[]),
        );
        assert_eq!(
            kind,
            IssueKind::ServiceClass {
                service_class_code: ServiceClassCode::CreditsOnly,
                transaction_code: TransactionCode::CheckingDebit
            }
        );
    }

    #[test]
    fn debits_only_rejects_credits() {
        let kind = rejected(
            ServiceClassCode::DebitsOnly,
            StandardEntryClass::Ppd,
            entry_params(TransactionCode::SavingsCredit, 100, &[]),
        );
        assert!(matches!(kind, IssueKind::ServiceClass { .. }));
    }

    #[test]
    fn prenote_requires_zero_amount() {
        // N6
        let kind = rejected(
            ServiceClassCode::MixedDebitsAndCredits,
            StandardEntryClass::Ppd,
            entry_params(TransactionCode::CheckingPrenoteCredit, 100, &[]),
        );
        assert_eq!(kind, IssueKind::PrenoteAmount { amount: 100 });
    }

    #[test]
    fn addenda_limit_per_sec() {
        // N8
        let kind = rejected(
            ServiceClassCode::MixedDebitsAndCredits,
            StandardEntryClass::Ppd,
            entry_params(TransactionCode::CheckingCredit, 100, &["A", "B"]),
        );
        assert_eq!(
            kind,
            IssueKind::TooManyAddenda {
                standard_entry_class: StandardEntryClass::Ppd,
                max: 1,
                count: 2
            }
        );
        let kind = rejected(
            ServiceClassCode::MixedDebitsAndCredits,
            StandardEntryClass::Tel,
            entry_params(TransactionCode::CheckingDebit, 100, &["A"]),
        );
        assert!(matches!(kind, IssueKind::TooManyAddenda { max: 0, .. }));
    }

    #[test]
    fn required_addenda() {
        let kind = rejected(
            ServiceClassCode::MixedDebitsAndCredits,
            StandardEntryClass::Dne,
            entry_params(TransactionCode::CheckingPrenoteCredit, 0, &[]),
        );
        assert_eq!(
            kind,
            IssueKind::TooFewAddenda {
                standard_entry_class: StandardEntryClass::Dne,
                min: 1,
                count: 0
            }
        );
    }

    #[test]
    fn tel_is_debit_only() {
        let kind = rejected(
            ServiceClassCode::MixedDebitsAndCredits,
            StandardEntryClass::Tel,
            entry_params(TransactionCode::CheckingCredit, 100, &[]),
        );
        assert_eq!(
            kind,
            IssueKind::EntryClassDirection {
                standard_entry_class: StandardEntryClass::Tel,
                transaction_code: TransactionCode::CheckingCredit
            }
        );
    }

    #[test]
    fn cie_is_credit_only() {
        let kind = rejected(
            ServiceClassCode::MixedDebitsAndCredits,
            StandardEntryClass::Cie,
            entry_params(TransactionCode::SavingsDebit, 100, &[]),
        );
        assert!(matches!(kind, IssueKind::EntryClassDirection { .. }));
    }

    #[test]
    fn reversal_batches_may_credit_debit_only_classes() {
        let mut writer = Writer::new(Vec::new());
        let mut file = writer.begin_file(file_params()).unwrap();
        let mut batch = file
            .begin_batch(BatchParams {
                standard_entry_class: StandardEntryClass::Tel,
                company_entry_description: nacha::REVERSAL.into(),
                ..batch_params(ServiceClassCode::MixedDebitsAndCredits)
            })
            .unwrap();
        batch
            .entry(entry_params(TransactionCode::CheckingCredit, 100, &[]))
            .unwrap();
        batch.finish().unwrap();
        file.finish().unwrap();
        let file = File::read(writer.get_ref().as_slice()).unwrap();
        assert_eq!(file.validate(), []);
    }

    #[test]
    fn required_entry_descriptions() {
        let mut writer = Writer::new(Vec::new());
        let mut file = writer.begin_file(file_params()).unwrap();
        let err = file
            .begin_batch(BatchParams {
                standard_entry_class: StandardEntryClass::Enr,
                ..batch_params(ServiceClassCode::MixedDebitsAndCredits)
            })
            .unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidBatch(IssueKind::EntryDescription {
                expected: "AUTOENROLL",
                ..
            })
        ));
        // Nothing was written for the rejected batch, and its number is unused.
        let batch = file
            .begin_batch(BatchParams {
                standard_entry_class: StandardEntryClass::Rck,
                company_entry_description: "redepcheck".into(),
                ..batch_params(ServiceClassCode::DebitsOnly)
            })
            .unwrap();
        assert_eq!(batch.header().batch_number, 1);
        batch.finish().unwrap();
        file.finish().unwrap();
        let output = String::from_utf8(writer.into_inner()).unwrap();
        assert!(output.contains("REDEPCHECK"));
        assert_eq!(record_lines(&output).len(), 4);
    }

    #[test]
    fn encoding_error_writes_nothing() {
        let (result, written) = try_entry(
            ServiceClassCode::MixedDebitsAndCredits,
            StandardEntryClass::Ppd,
            EntryParams {
                individual_name: "A NAME THAT IS FAR TOO LONG FOR THE FIELD".into(),
                ..entry_params(TransactionCode::CheckingCredit, 100, &[])
            },
        );
        assert!(matches!(result, Err(Error::Record(_))));
        assert_eq!(written, 2 * 95);
    }

    #[test]
    fn rejected_entry_does_not_use_a_trace_number() {
        let mut writer = Writer::new(Vec::new());
        let mut file = writer.begin_file(file_params()).unwrap();
        let mut batch = file
            .begin_batch(batch_params(ServiceClassCode::CreditsOnly))
            .unwrap();
        assert!(
            batch
                .entry(entry_params(TransactionCode::CheckingDebit, 1, &[]))
                .is_err()
        );
        let entry = batch
            .entry(entry_params(TransactionCode::CheckingCredit, 1, &[]))
            .unwrap();
        assert_eq!(entry.detail.sequence_number(), 1);
        batch.finish().unwrap();
        file.finish().unwrap();
    }

    #[test]
    fn dropped_file_writes_no_control() {
        // Python: an exception inside `with begin_file` skipped the file control.
        let mut writer = Writer::new(Vec::new());
        let file = writer.begin_file(file_params()).unwrap();
        drop(file);
        let output = String::from_utf8(writer.into_inner()).unwrap();
        assert_eq!(output.lines().count(), 1);
    }
}
