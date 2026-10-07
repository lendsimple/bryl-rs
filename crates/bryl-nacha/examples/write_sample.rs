//! Writes a sample NACHA file to stdout:
//! `cargo run -p bryl-nacha --example write_sample > sample.ach`.

use std::io::{self, Write};

use nacha::{
    BatchParams, EntryParams, FileParams, ServiceClassCode, StandardEntryClass, TransactionCode,
    Writer,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = Writer::new(io::stdout().lock());
    let mut file = writer.begin_file(
        FileParams::builder()
            .immediate_destination("091000019".parse()?)
            .immediate_destination_name("DEST BANK")
            .immediate_origin("1234567890")
            .immediate_origin_name("ACME CORP")
            .created_at(FileParams::now())
            .build(),
    )?;
    let mut batch = file.begin_batch(
        BatchParams::builder()
            .service_class_code(ServiceClassCode::MixedDebitsAndCredits)
            .company_name("ACME CORP")
            .company_id("1234567890")
            .standard_entry_class(StandardEntryClass::Ppd)
            .company_entry_description("PAYROLL")
            .originating_dfi_id(9_100_001)
            .build(),
    )?;
    for (name, amount, code) in [
        ("JANE SMITH", 125_000, TransactionCode::CheckingCredit),
        ("JOHN DOE", 98_050, TransactionCode::SavingsCredit),
        ("ACME VENDOR", 4_200, TransactionCode::CheckingDebit),
    ] {
        batch.entry(
            EntryParams::builder()
                .transaction_code(code)
                .receiving_dfi("021000021".parse()?)
                .account_number("000123456789")
                .amount(amount)
                .individual_id("ID0001")
                .individual_name(name)
                .addenda(vec!["SAMPLE PAYMENT".into()])
                .build(),
        )?;
    }
    batch.finish()?;
    file.finish()?;
    writer.into_inner().flush()?;
    Ok(())
}
