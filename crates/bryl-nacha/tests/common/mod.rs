//! Shared test fixtures.

#![allow(dead_code)]

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use nacha::{
    BatchParams, EntryDetail, EntryParams, FileParams, RoutingNumber, ServiceClassCode,
    StandardEntryClass, TransactionCode, Writer,
};

pub fn sample_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2023, 6, 15).unwrap()
}

pub fn sample_time() -> NaiveTime {
    NaiveTime::from_hms_opt(14, 30, 0).unwrap()
}

pub fn sample_datetime() -> NaiveDateTime {
    sample_date().and_time(sample_time())
}

/// 091000019 (a valid ABA routing number).
pub fn routing() -> RoutingNumber {
    "091000019".parse().unwrap()
}

/// 021000021, another valid routing number.
pub fn other_routing() -> RoutingNumber {
    "021000021".parse().unwrap()
}

/// A minimal valid entry detail.
pub fn entry_detail(transaction_code: TransactionCode) -> EntryDetail {
    EntryDetail::builder()
        .transaction_code(transaction_code)
        .receiving_dfi(routing())
        .receiving_dfi_account_number("9876543210")
        .amount(10_000)
        .individual_id("ID123")
        .individual_name("JOHN DOE")
        .trace_number(123_456_789_012_345)
        .build()
}

pub fn file_params() -> FileParams {
    FileParams::builder()
        .immediate_destination(routing())
        .immediate_destination_name("DEST BANK")
        .immediate_origin("9876543210")
        .immediate_origin_name("ORIGIN BANK")
        .created_at(sample_datetime())
        .build()
}

pub fn batch_params(service_class_code: ServiceClassCode) -> BatchParams {
    BatchParams::builder()
        .service_class_code(service_class_code)
        .company_name("ACME CORP")
        .company_id("1234567890")
        .standard_entry_class(StandardEntryClass::Ppd)
        .company_entry_description("PAYROLL")
        .originating_dfi_id(12_345_678)
        .effective_entry_date(sample_date())
        .build()
}

pub fn entry_params(
    transaction_code: TransactionCode,
    amount: u64,
    addenda: &[&str],
) -> EntryParams {
    EntryParams::builder()
        .transaction_code(transaction_code)
        .receiving_dfi(routing())
        .account_number("123456789")
        .amount(amount)
        .individual_id("EMP001")
        .individual_name("JANE SMITH")
        .addenda(addenda.iter().map(ToString::to_string).collect())
        .build()
}

/// Writes a file with one mixed batch of identical entries and returns it.
pub fn write_sample_file(
    entry_count: usize,
    amount: u64,
    transaction_code: TransactionCode,
    addenda: &[&str],
) -> String {
    let mut writer = Writer::new(Vec::new());
    let mut file = writer.begin_file(file_params()).unwrap();
    let mut batch = file
        .begin_batch(batch_params(ServiceClassCode::MixedDebitsAndCredits))
        .unwrap();
    for _ in 0..entry_count {
        batch
            .entry(entry_params(transaction_code, amount, addenda))
            .unwrap();
    }
    batch.finish().unwrap();
    file.finish().unwrap();
    String::from_utf8(writer.into_inner()).unwrap()
}

pub fn sample_file() -> String {
    write_sample_file(1, 10_000, TransactionCode::CheckingCredit, &[])
}

/// Lines that are records, excluding `9…9` filler.
pub fn record_lines(output: &str) -> Vec<&str> {
    output
        .lines()
        .filter(|line| *line != "9".repeat(94))
        .collect()
}
