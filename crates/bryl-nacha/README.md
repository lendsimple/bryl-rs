# bryl-nacha

Write, read and validate NACHA ACH files (`use nacha::...`).

```rust
use chrono::NaiveDate;
use nacha::{
    BatchParams, EntryParams, FileParams, ServiceClassCode, StandardEntryClass,
    TransactionCode, Writer,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let created_at = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap().and_hms_opt(14, 30, 0).unwrap();
    let mut writer = Writer::new(Vec::new());
    let mut file = writer.begin_file(
        FileParams::builder()
            .immediate_destination("091000019".parse()?)
            .immediate_destination_name("DEST BANK")
            .immediate_origin("1234567890")
            .immediate_origin_name("ACME LENDING")
            .created_at(created_at)
            .build(),
    )?;
    let mut batch = file.begin_batch(
        BatchParams::builder()
            .service_class_code(ServiceClassCode::CreditsOnly)
            .company_name("ACME LENDING")
            .company_id("1234567890")
            .standard_entry_class(StandardEntryClass::Ppd)
            .company_entry_description("LOAN")
            .originating_dfi_id(9_100_001)
            .build(),
    )?;
    batch.entry(
        EntryParams::builder()
            .transaction_code(TransactionCode::CheckingCredit)
            .receiving_dfi("021000021".parse()?)
            .account_number("000123456789")
            .amount(250_000) // cents
            .individual_id("LOAN0001")
            .individual_name("JANE SMITH")
            .build(),
    )?;
    batch.finish()?;
    file.finish()?;
    let ach = writer.into_inner();

    // Read it back and check every control total and rule.
    let file = nacha::File::read(ach.as_slice())?;
    assert_eq!(file.control.total_credit_amount, 250_000);
    assert!(file.validate().is_empty());
    Ok(())
}
```

- **Writing:** `Writer` → `FileWriter` → `BatchWriter` are nested guards, so
  an entry outside a batch does not compile. The writer computes hashes,
  totals, counts and the block count, pads to whole blocks, and rejects
  batches and entries that break NACHA rules (service class, SEC code
  debit/credit and addenda rules, prenote amounts) before writing anything.
  Lines end with `\n` or, optionally, `\r\n`.
- **Reading:** `Reader` reads record by record; `File::read` reads a whole
  file and `File::validate` reports every problem.
- **Values:** `RoutingNumber` checks the ABA checksum; `TransactionCode::for_entry`
  picks a code from a signed amount.

The crate documentation (`cargo doc -p bryl-nacha --open`) covers the rules,
errors and configuration. Spec decisions are listed in the repository's
`DEVIATIONS.md`.
