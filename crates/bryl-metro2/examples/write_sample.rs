//! Writes a sample Metro 2 file to stdout:
//! `cargo run -p bryl-metro2 --example write_sample > sample.dat`
//! (add `-- --newline` for one record per line).

use std::io::{self, Write};

use chrono::NaiveDate;
use metro2::{
    AccountStatus, AccountType, BaseSegment, DataRecord, EcoaCode, HeaderRecord, J1Segment,
    PortfolioType, Writer,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let newline = std::env::args().any(|arg| arg == "--newline");
    let day = NaiveDate::from_ymd_opt(2024, 1, 31).ok_or("bad date")?;
    let header = HeaderRecord::builder()
        .activity_date(day)
        .date_created(day)
        .reporter_name("ACME LENDING")
        .reporter_address("123 MAIN ST ANYTOWN US 12345")
        .reporter_telephone_number(5_551_234_567)
        .build();
    let mut writer = Writer::new(io::stdout().lock()).newline(newline);
    let mut file = writer.begin_file(&header)?;
    for (account, status, surname) in [
        ("ACCT001", AccountStatus::Current, "SMITH"),
        ("ACCT002", AccountStatus::Dpd60, "JONES"),
        ("ACCT003", AccountStatus::PaidOrClosed, "GARCIA"),
    ] {
        let base = BaseSegment::builder()
            .identification_number("FURNISHER123")
            .consumer_account_number(account)
            .portfolio_type(PortfolioType::Installment)
            .account_type(AccountType::Unsecured)
            .date_opened(NaiveDate::from_ymd_opt(2022, 3, 1).ok_or("bad date")?)
            .account_status(status)
            .maybe_payment_rating(
                status
                    .requires_payment_rating()
                    .then_some(metro2::PaymentRating::Current),
            )
            .current_balance(1_200)
            .amount_past_due(if status == AccountStatus::Dpd60 {
                300
            } else {
                0
            })
            .date_of_account_information(day)
            .surname(surname)
            .first_name("ALEX")
            .ecoa_code(EcoaCode::Joint)
            .first_line_of_address("123 MAIN ST")
            .city("ANYTOWN")
            .state("CA")
            .zip_code("90210")
            .build();
        let co_borrower = J1Segment::builder()
            .surname(surname)
            .first_name("SAM")
            .ecoa_code(EcoaCode::Joint)
            .build();
        file.write(&DataRecord {
            j1: vec![co_borrower],
            ..DataRecord::new(base)
        })?;
    }
    file.finish()?;
    writer.into_inner().flush()?;
    Ok(())
}
