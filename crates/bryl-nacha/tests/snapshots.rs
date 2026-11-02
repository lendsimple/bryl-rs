//! Snapshot tests: the writer's output for fixed scenarios must match the
//! committed files in `tests/fixtures/snapshots` byte for byte, so any change
//! to written bytes shows up as a reviewable diff.
//!
//! Regenerate with `UPDATE_SNAPSHOTS=1 cargo test -p bryl-nacha --test snapshots`.

mod common;

use chrono::NaiveDate;
use common::*;
use nacha::{
    BatchParams, ReturnParams, ReturnReasonCode, ServiceClassCode, StandardEntryClass,
    TransactionCode, Writer,
};
use pretty_assertions::assert_eq;

/// Compares `actual` with the snapshot `tests/fixtures/snapshots/{name}`.
/// Run with `UPDATE_SNAPSHOTS=1` to write the snapshot instead, and review
/// the diff before committing it.
fn assert_snapshot(name: &str, actual: &str) {
    let path = format!(
        "{}/tests/fixtures/snapshots/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing snapshot {path}; run with UPDATE_SNAPSHOTS=1"));
    assert_eq!(actual, expected, "{name} differs from its snapshot");
}

fn read_snapshot(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/snapshots/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn write(build: impl FnOnce(&mut nacha::FileWriter<'_, Vec<u8>>)) -> String {
    let mut writer = Writer::new(Vec::new());
    let mut file = writer.begin_file(file_params()).unwrap();
    build(&mut file);
    file.finish().unwrap();
    String::from_utf8(writer.into_inner()).unwrap()
}

fn named(name: &str, amount: u64, code: TransactionCode, addenda: &[&str]) -> nacha::EntryParams {
    nacha::EntryParams {
        individual_name: name.into(),
        ..entry_params(code, amount, addenda)
    }
}

#[test]
fn single_entry() {
    let output = write(|file| {
        let mut batch = file
            .begin_batch(batch_params(ServiceClassCode::MixedDebitsAndCredits))
            .unwrap();
        batch
            .entry(entry_params(TransactionCode::CheckingCredit, 10_000, &[]))
            .unwrap();
        batch.finish().unwrap();
    });
    assert_snapshot("single_entry.ach", &output);
}

#[test]
fn entries_with_addenda() {
    let output = write(|file| {
        let mut batch = file
            .begin_batch(batch_params(ServiceClassCode::MixedDebitsAndCredits))
            .unwrap();
        batch
            .entry(named(
                "JANE SMITH",
                3000,
                TransactionCode::CheckingCredit,
                &["MEMO LINE"],
            ))
            .unwrap();
        batch
            .entry(named(
                "JOHN DOE",
                1500,
                TransactionCode::CheckingDebit,
                &["SECOND MEMO"],
            ))
            .unwrap();
        batch.finish().unwrap();
    });
    assert_snapshot("entries_with_addenda.ach", &output);
}

#[test]
fn two_batches() {
    let output = write(|file| {
        let mut batch = file
            .begin_batch(batch_params(ServiceClassCode::CreditsOnly))
            .unwrap();
        for amount in [1000, 2000, 3000] {
            batch
                .entry(entry_params(TransactionCode::CheckingCredit, amount, &[]))
                .unwrap();
        }
        batch.finish().unwrap();
        let mut batch = file
            .begin_batch(BatchParams {
                company_entry_description: "BILLING".into(),
                ..batch_params(ServiceClassCode::DebitsOnly)
            })
            .unwrap();
        for amount in [500, 700] {
            batch
                .entry(entry_params(TransactionCode::CheckingDebit, amount, &[]))
                .unwrap();
        }
        batch.finish().unwrap();
    });
    assert_snapshot("two_batches.ach", &output);
}

#[test]
fn returns() {
    let output = write(|file| {
        let mut batch = file
            .begin_batch(batch_params(ServiceClassCode::DebitsOnly))
            .unwrap();
        batch
            .entry(return_entry_params(ReturnReasonCode::InsufficientFunds))
            .unwrap();
        batch
            .entry(nacha::EntryParams {
                special_addendum: Some(
                    ReturnParams {
                        original_entry_trace_number: 91_000_010_000_002,
                        date_of_death: NaiveDate::from_ymd_opt(2023, 5, 1),
                        addenda_information: "SSA NOTICE".into(),
                        ..return_params(ReturnReasonCode::AccountHolderDeceased)
                    }
                    .into(),
                ),
                ..named("JOHN DOE", 1500, TransactionCode::SavingsReturnedDebit, &[])
            })
            .unwrap();
        batch.finish().unwrap();
    });
    assert_snapshot("returns.ach", &output);
}

#[test]
fn loan_and_gl_entries() {
    let output = write(|file| {
        // Loan disbursement, a loan prenote and GL movements.
        let mut batch = file
            .begin_batch(batch_params(ServiceClassCode::MixedDebitsAndCredits))
            .unwrap();
        for (name, amount, code) in [
            ("JANE SMITH", 250_000, TransactionCode::LoanCredit),
            ("JOHN DOE", 0, TransactionCode::LoanPrenoteCredit),
            (
                "ACME OPERATING",
                12_000,
                TransactionCode::GeneralLedgerCredit,
            ),
            ("ACME RESERVE", 12_000, TransactionCode::GeneralLedgerDebit),
        ] {
            batch.entry(named(name, amount, code, &[])).unwrap();
        }
        batch.finish().unwrap();

        // Reversing a loan credit sent in error: loan debits (55) are for
        // reversals only.
        let mut batch = file
            .begin_batch(BatchParams {
                company_entry_description: nacha::REVERSAL.into(),
                ..batch_params(ServiceClassCode::DebitsOnly)
            })
            .unwrap();
        batch
            .entry(named(
                "JANE SMITH",
                250_000,
                TransactionCode::LoanDebit,
                &[],
            ))
            .unwrap();
        batch.finish().unwrap();

        // Remittance data without money moving.
        let mut batch = file
            .begin_batch(BatchParams {
                standard_entry_class: StandardEntryClass::Ccd,
                company_entry_description: "REMITTANCE".into(),
                ..batch_params(ServiceClassCode::CreditsOnly)
            })
            .unwrap();
        batch
            .entry(named(
                "ACME SUPPLIER",
                0,
                TransactionCode::CheckingZeroDollarCredit,
                &["RMR*IV*INV0001*PI*1500.00\\"],
            ))
            .unwrap();
        batch.finish().unwrap();
    });
    assert_snapshot("loan_and_gl_entries.ach", &output);
}

#[test]
fn snapshots_read_back_and_validate() {
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        return; // the snapshots are being rewritten by the other tests
    }
    for name in [
        "single_entry",
        "entries_with_addenda",
        "two_batches",
        "returns",
        "loan_and_gl_entries",
    ] {
        let file = nacha::File::read(read_snapshot(&format!("{name}.ach")).as_bytes()).unwrap();
        assert_eq!(file.validate(), [], "{name}");
    }
}
