//! Shared test fixtures.

#![allow(dead_code)]

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use nacha::{
    BatchParams, ChangeCode, ContestedReturnCode, ContestedReturnParams, DishonoredReturnCode,
    DishonoredReturnParams, EntryDetail, EntryParams, FileParams, NocParams, ReturnParams,
    ReturnReasonCode, RoutingNumber, ServiceClassCode, StandardEntryClass, TransactionCode, Writer,
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

/// A return addendum for an entry that was sent with trace number
/// 091000010000001 to routing number 091000019.
pub fn return_params(return_reason_code: ReturnReasonCode) -> ReturnParams {
    ReturnParams::builder()
        .return_reason_code(return_reason_code)
        .original_entry_trace_number(91_000_010_000_001)
        .original_receiving_dfi_id(9_100_001)
        .build()
}

/// A notification of change for an entry that was sent with trace number
/// 091000010000001 to routing number 091000019.
pub fn noc_params(change_code: ChangeCode, corrected_data: &str) -> NocParams {
    NocParams::builder()
        .change_code(change_code)
        .original_entry_trace_number(91_000_010_000_001)
        .original_receiving_dfi_id(9_100_001)
        .corrected_data(corrected_data)
        .build()
}

/// A notification of change correcting the account number of a checking
/// debit.
pub fn noc_entry_params() -> EntryParams {
    EntryParams {
        special_addendum: Some(noc_params(ChangeCode::IncorrectAccountNumber, "987654321").into()),
        ..entry_params(TransactionCode::CheckingReturnedDebit, 0, &[])
    }
}

/// Batch values for a COR (notification of change) batch.
pub fn cor_batch_params() -> BatchParams {
    BatchParams {
        standard_entry_class: StandardEntryClass::Cor,
        company_entry_description: "NOC".into(),
        ..batch_params(ServiceClassCode::MixedDebitsAndCredits)
    }
}

/// A dishonor of an R01 return (trace 021000020000004) of the entry sent
/// with trace number 091000010000001.
pub fn dishonored_params(code: DishonoredReturnCode) -> DishonoredReturnParams {
    DishonoredReturnParams::builder()
        .dishonored_return_code(code)
        .original_entry_trace_number(91_000_010_000_001)
        .original_receiving_dfi_id(9_100_001)
        .return_trace_number(21_000_020_000_004)
        .return_settlement_date("152")
        .return_reason_code(1)
        .build()
}

/// A contest of an R68 dishonor (trace 091000010000009) of that return.
pub fn contested_params(code: ContestedReturnCode) -> ContestedReturnParams {
    ContestedReturnParams::builder()
        .contested_return_code(code)
        .original_entry_trace_number(91_000_010_000_001)
        .date_original_entry_returned(NaiveDate::from_ymd_opt(2023, 6, 10).unwrap())
        .original_receiving_dfi_id(9_100_001)
        .original_settlement_date("150")
        .return_trace_number(21_000_020_000_004)
        .return_settlement_date("152")
        .return_reason_code(1)
        .dishonored_return_trace_number(91_000_010_000_009)
        .dishonored_return_settlement_date("155")
        .dishonored_return_reason_code(68)
        .build()
}

/// A return of a checking debit.
pub fn return_entry_params(return_reason_code: ReturnReasonCode) -> EntryParams {
    EntryParams {
        special_addendum: Some(return_params(return_reason_code).into()),
        ..entry_params(TransactionCode::CheckingReturnedDebit, 2500, &[])
    }
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

/// Writes a file with one debits-only batch holding an R01 return.
pub fn return_file() -> String {
    let mut writer = Writer::new(Vec::new());
    let mut file = writer.begin_file(file_params()).unwrap();
    let mut batch = file
        .begin_batch(batch_params(ServiceClassCode::DebitsOnly))
        .unwrap();
    batch
        .entry(return_entry_params(ReturnReasonCode::InsufficientFunds))
        .unwrap();
    batch.finish().unwrap();
    file.finish().unwrap();
    String::from_utf8(writer.into_inner()).unwrap()
}

/// Writes a file with one COR batch holding a C01 notification of change.
pub fn noc_file() -> String {
    let mut writer = Writer::new(Vec::new());
    let mut file = writer.begin_file(file_params()).unwrap();
    let mut batch = file.begin_batch(cor_batch_params()).unwrap();
    batch.entry(noc_entry_params()).unwrap();
    batch.finish().unwrap();
    file.finish().unwrap();
    String::from_utf8(writer.into_inner()).unwrap()
}

/// Writes a file with one debits-only batch holding an R68 dishonored
/// return.
pub fn dishonored_file() -> String {
    let mut writer = Writer::new(Vec::new());
    let mut file = writer.begin_file(file_params()).unwrap();
    let mut batch = file
        .begin_batch(batch_params(ServiceClassCode::DebitsOnly))
        .unwrap();
    batch
        .entry(EntryParams {
            special_addendum: Some(dishonored_params(DishonoredReturnCode::UntimelyReturn).into()),
            ..entry_params(TransactionCode::CheckingReturnedDebit, 2500, &[])
        })
        .unwrap();
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
