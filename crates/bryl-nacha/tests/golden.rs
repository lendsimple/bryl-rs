//! Byte-for-byte comparison with files written by lms-python's `nacha.py`
//! (`tools/gen_golden.py`). Each intentional difference from DEVIATIONS.md is
//! applied to the Python output by a named function below, so the remaining
//! bytes must match exactly.

mod common;

use common::*;
use nacha::{BatchParams, ServiceClassCode, TransactionCode, Writer};
use pretty_assertions::assert_eq;

fn golden(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/golden/{name}.ach",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

/// N14: account number (columns 13–29) is left-aligned, not right-aligned.
fn left_align_account_numbers(line: &mut String) {
    if line.starts_with('6') {
        let account = line[12..29].trim().to_owned();
        line.replace_range(12..29, &format!("{account:<17}"));
    }
}

/// N1: addenda sequence numbers (columns 84–87) start at 1, not 0.
fn number_addenda_from_one(line: &mut String) {
    if line.starts_with('7') {
        let sequence: u32 = line[83..87].parse().unwrap();
        line.replace_range(83..87, &format!("{:04}", sequence + 1));
    }
}

/// N2: the trace sequence (last 7 digits of the trace number, and the
/// addendum's entry detail sequence number) runs across the whole file
/// instead of restarting in each batch.
fn renumber_traces(lines: &mut [String]) {
    let mut sequence = 0;
    for line in lines {
        if line.starts_with('6') {
            sequence += 1;
            line.replace_range(87..94, &format!("{sequence:07}"));
        } else if line.starts_with('7') {
            line.replace_range(87..94, &format!("{sequence:07}"));
        }
    }
}

/// N3: pad to a multiple of 10 lines with 94 `9`s.
fn pad_blocks(lines: &mut Vec<String>) {
    while lines.len() % 10 != 0 {
        lines.push("9".repeat(94));
    }
}

fn expected(name: &str) -> String {
    let mut lines: Vec<String> = golden(name).lines().map(str::to_owned).collect();
    for line in &mut lines {
        left_align_account_numbers(line);
        number_addenda_from_one(line);
    }
    renumber_traces(&mut lines);
    pad_blocks(&mut lines);
    lines.iter().map(|line| line.clone() + "\n").collect()
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
    assert_eq!(output, expected("single_entry"));
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
    assert_eq!(output, expected("entries_with_addenda"));
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
    assert_eq!(output, expected("two_batches"));
}
