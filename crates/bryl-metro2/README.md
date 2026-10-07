# bryl-metro2

Write, read and validate Metro 2 credit reporting files, character format
(`use metro2::...`).

```rust
use chrono::NaiveDate;
use metro2::{
    AccountStatus, AccountType, BaseSegment, DataRecord, EcoaCode, HeaderRecord,
    PortfolioType, Writer,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let as_of = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
    let header = HeaderRecord::builder()
        .activity_date(as_of)
        .date_created(as_of)
        .reporter_name("ACME LENDING")
        .reporter_address("123 MAIN ST ANYTOWN CA 90210")
        .reporter_telephone_number(5_551_234_567)
        .build();
    let account = BaseSegment::builder()
        .identification_number("FURNISHER123")
        .consumer_account_number("LOAN0001")
        .portfolio_type(PortfolioType::Installment)
        .account_type(AccountType::Unsecured)
        .date_opened(NaiveDate::from_ymd_opt(2022, 3, 1).unwrap())
        .current_balance(6_512) // whole dollars
        .account_status(AccountStatus::Current)
        .date_of_account_information(as_of)
        .surname("SMITH")
        .first_name("JANE")
        .ecoa_code(EcoaCode::Individual)
        .first_line_of_address("123 MAIN ST")
        .city("ANYTOWN")
        .state("CA")
        .zip_code("90210")
        .build();

    let mut writer = Writer::new(Vec::new());
    let mut file = writer.begin_file(&header)?;
    file.write(&DataRecord::new(account))?; // validated before writing
    let trailer = file.finish()?; // totals computed for you
    assert_eq!(trailer.total_status_code_11, 1);

    // Read it back and re-check every record and the trailer.
    let file = metro2::File::read(writer.get_ref().as_slice())?;
    assert_eq!(file.data_records[0].base.consumer_account_number, "LOAN0001");
    assert!(file.validate().is_empty());
    Ok(())
}
```

- **Records:** header, base segment, J1/J2/K1–K4/L1/N1 segments and
  trailer. Every code table is an enum, so an unknown code cannot be
  written. A `DataRecord` is a base segment and its segments, and its length
  is set when written.
- **Writing:** `Writer` checks each record before writing it (payment rating
  against account status, amount past due, payment history, the characters
  allowed in names, addresses and account numbers, the retired status 05),
  then computes the trailer.
- **Reading:** `Reader` reads RDW-framed, newline-delimited and
  variable-blocked files; `File::read` reads a whole file and
  `File::validate` reports every problem.

The packed (binary) format is not supported, and blocked files can be read
but not written. The crate documentation (`cargo doc -p bryl-metro2 --open`)
covers account statuses, the validation rules and reading. Spec decisions are
listed in the repository's `DEVIATIONS.md`.

## License

Licensed under the [MIT License](LICENSE).

The moov-io reference files in `tests/fixtures/moov` are under moov-io's
Apache License 2.0 (see `tests/fixtures/moov/LICENSE`).
