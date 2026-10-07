//! [Metro 2](https://www.cdiaonline.org/) credit reporting files (character
//! format):
//!
//! ```text
//! HeaderRecord          (426 characters)
//! BaseSegment           (426 characters, one per account)
//!   J1Segment           (100, associated consumer, same address)     0..n
//!   J2Segment           (200, associated consumer, other address)    0..n
//!   K1Segment           (34, original creditor)                      0..1
//!   K2Segment           (34, purchased from / sold to)               0..1
//!   K3Segment           (40, mortgage information)                   0..1
//!   K4Segment           (30, specialized payment information)        0..1
//!   L1Segment           (54, account number / ID change)             0..1
//!   N1Segment           (146, employment)                            0..1
//! TrailerRecord         (426 characters)
//! ```
//!
//! A base segment and its appended segments form a [`DataRecord`] (one
//! account). [`Writer`] writes files: it validates each data record, sets
//! each record's length, and computes the trailer. [`Reader`] reads files
//! record by record (RDW-framed, newline-delimited or variable-blocked),
//! [`File::read`] reads them whole, and [`File::validate`] checks them.
//!
//! # Writing a file
//!
//! ```
//! use chrono::NaiveDate;
//! use metro2::{
//!     AccountStatus, AccountType, BaseSegment, DataRecord, EcoaCode, HeaderRecord,
//!     J1Segment, PortfolioType, Writer,
//! };
//!
//! let as_of = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
//! let header = HeaderRecord::builder()
//!     .activity_date(as_of)
//!     .date_created(as_of)
//!     .reporter_name("ACME LENDING")
//!     .reporter_address("123 MAIN ST ANYTOWN CA 90210")
//!     .reporter_telephone_number(5_551_234_567)
//!     .equifax_program_identifier("EFX123") // identifiers assigned by each bureau
//!     .build();
//!
//! let base = BaseSegment::builder()
//!     .identification_number("FURNISHER123")
//!     .consumer_account_number("LOAN0001")
//!     .portfolio_type(PortfolioType::Installment)
//!     .account_type(AccountType::Unsecured)
//!     .date_opened(NaiveDate::from_ymd_opt(2022, 3, 1).unwrap())
//!     .highest_credit(10_000) // whole dollars
//!     .terms_duration("036")
//!     .scheduled_monthly_payment(320)
//!     .current_balance(6_512)
//!     .account_status(AccountStatus::Current)
//!     .date_of_account_information(as_of)
//!     .surname("SMITH")
//!     .first_name("JANE")
//!     .social_security_number(123_456_789)
//!     .date_of_birth(NaiveDate::from_ymd_opt(1985, 7, 4).unwrap())
//!     .ecoa_code(EcoaCode::Joint)
//!     .first_line_of_address("123 MAIN ST")
//!     .city("ANYTOWN")
//!     .state("CA")
//!     .zip_code("90210")
//!     .build();
//! // A co-borrower at the same address.
//! let co_borrower = J1Segment::builder()
//!     .surname("SMITH")
//!     .first_name("ALEX")
//!     .ecoa_code(EcoaCode::Joint)
//!     .build();
//!
//! let mut writer = Writer::new(Vec::new());
//! let mut file = writer.begin_file(&header)?;
//! file.write(&DataRecord {
//!     j1: vec![co_borrower],
//!     ..DataRecord::new(base)
//! })?;
//! let trailer = file.finish()?;
//!
//! assert_eq!(trailer.total_base_records, 1);
//! assert_eq!(trailer.total_j1_segments, 1);
//! assert_eq!(trailer.total_ssns_all_segments, 1);
//! assert_eq!(writer.into_inner().len(), 426 + 526 + 426);
//! # Ok::<(), metro2::Error>(())
//! ```
//!
//! Amounts are whole dollars. Optional fields default to blanks, zeros or
//! `None`; an optional date of `None` is written as `00000000`. Text is
//! uppercased when written. Records are written back to back, each starting
//! with its length (the record descriptor word); use [`Writer::newline`] to
//! end each record with a newline instead. Set the other segments (J2, K1–K4,
//! L1, N1) the same way as `j1` above.
//!
//! # Account status and payment rating
//!
//! [`AccountStatus`] describes the account's condition as of the date of
//! account information. The payment rating is reported only for statuses
//! that close or transfer an account (05, 13, 65, 88, 89, 94, 95), where it
//! records the account's condition just before; it must be blank for every
//! other status ([`AccountStatus::requires_payment_rating`]).
//!
//! ```
//! # use chrono::NaiveDate;
//! # use metro2::*;
//! # fn base() -> BaseSegment {
//! #     let day = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
//! #     BaseSegment::builder().identification_number("FURNISHER123").consumer_account_number("LOAN0001")
//! #         .portfolio_type(PortfolioType::Installment).account_type(AccountType::Unsecured)
//! #         .date_opened(NaiveDate::from_ymd_opt(2022, 3, 1).unwrap()).account_status(AccountStatus::Current)
//! #         .date_of_account_information(day).surname("SMITH").first_name("JANE")
//! #         .ecoa_code(EcoaCode::Individual).first_line_of_address("123 MAIN ST").city("ANYTOWN")
//! #         .state("CA").zip_code("90210").build()
//! # }
//! // A loan paid off after being 30 days late: status 13 with rating 1.
//! let paid = BaseSegment {
//!     account_status: AccountStatus::PaidOrClosed,
//!     payment_rating: Some(PaymentRating::Past30),
//!     current_balance: 0,
//!     date_closed: Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()),
//!     ..base()
//! };
//! assert_eq!(paid.validate(), Ok(()));
//!
//! // A current account has no payment rating.
//! let current = BaseSegment {
//!     account_status: AccountStatus::Current,
//!     payment_rating: Some(PaymentRating::Current),
//!     ..base()
//! };
//! assert_eq!(
//!     current.validate().unwrap_err()[0].to_string(),
//!     "Payment rating must be blank for account status '11', got '0'"
//! );
//! ```
//!
//! Status 05 (transferred) was retired in April 2022 and is rejected when
//! writing. To report a transfer, keep the status the account had at the
//! time of transfer and add special comment AT (transferred within the
//! company) or O (transferred to another company), with zero balance, amount
//! past due and scheduled payment:
//!
//! ```
//! # use chrono::NaiveDate;
//! # use metro2::*;
//! # fn base() -> BaseSegment {
//! #     let day = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
//! #     BaseSegment::builder().identification_number("FURNISHER123").consumer_account_number("LOAN0001")
//! #         .portfolio_type(PortfolioType::Installment).account_type(AccountType::Unsecured)
//! #         .date_opened(NaiveDate::from_ymd_opt(2022, 3, 1).unwrap()).account_status(AccountStatus::Current)
//! #         .date_of_account_information(day).surname("SMITH").first_name("JANE")
//! #         .ecoa_code(EcoaCode::Individual).first_line_of_address("123 MAIN ST").city("ANYTOWN")
//! #         .state("CA").zip_code("90210").build()
//! # }
//! let transferred = BaseSegment {
//!     special_comment: Some(SpecialComment::O),
//!     current_balance: 0,
//!     amount_past_due: 0,
//!     scheduled_monthly_payment: 0,
//!     ..base()
//! };
//! assert_eq!(transferred.validate(), Ok(()));
//! ```
//!
//! # Rules the writer enforces
//!
//! [`FileWriter::write`] checks each record with [`DataRecord::validate`]
//! and writes nothing for a record that breaks a rule:
//!
//! - **Payment rating** against account status, as above.
//! - **Amount past due** is 0 for statuses 11 (current) and 13 (paid or
//!   closed).
//! - **Payment history profile:** each month is a [`PaymentHistoryCode`] or
//!   a space, and `B` (no history) may only be followed by `B` or spaces.
//! - **Characters** ([`CharClass`]): names allow letters, spaces and
//!   hyphens; address lines and city also digits, `/`, `-` and `.`;
//!   consumer account and identification numbers only letters and digits.
//! - **Retired codes:** account status 05.
//!
//! Unknown codes cannot be written at all: every code field is an enum.
//!
//! ```
//! # use chrono::NaiveDate;
//! # use metro2::*;
//! # fn base() -> BaseSegment {
//! #     let day = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
//! #     BaseSegment::builder().identification_number("FURNISHER123").consumer_account_number("LOAN0001")
//! #         .portfolio_type(PortfolioType::Installment).account_type(AccountType::Unsecured)
//! #         .date_opened(NaiveDate::from_ymd_opt(2022, 3, 1).unwrap()).account_status(AccountStatus::Current)
//! #         .date_of_account_information(day).surname("SMITH").first_name("JANE")
//! #         .ecoa_code(EcoaCode::Individual).first_line_of_address("123 MAIN ST").city("ANYTOWN")
//! #         .state("CA").zip_code("90210").build()
//! # }
//! # let header = HeaderRecord::builder().activity_date(NaiveDate::from_ymd_opt(2024, 1, 31).unwrap())
//! #     .date_created(NaiveDate::from_ymd_opt(2024, 1, 31).unwrap()).reporter_name("ACME")
//! #     .reporter_address("ADDRESS").reporter_telephone_number(5_551_234_567).build();
//! let mut writer = Writer::new(Vec::new());
//! let mut file = writer.begin_file(&header)?;
//! let err = file
//!     .write(&DataRecord::new(BaseSegment {
//!         surname: "O'BRIEN".into(),
//!         ..base()
//!     }))
//!     .unwrap_err();
//! assert_eq!(
//!     err.to_string(),
//!     "account LOAN0001: surname contains '\\'' at position 1; \
//!      only letters, spaces and hyphens are allowed"
//! );
//! # Ok::<(), metro2::Error>(())
//! ```
//!
//! Clean such data before writing (here, `OBRIEN`), or turn every check off
//! with `Writer::new(out).validate(false)`.
//!
//! # Reading and validating a file
//!
//! [`File::read`] parses a whole file and checks its structure (record
//! lengths, field formats, codes). Then [`File::validate`] recomputes the
//! trailer and re-checks every record, returning every problem it finds.
//!
//! ```
//! # fn sample() -> Result<Vec<u8>, metro2::Error> {
//! #     use chrono::NaiveDate;
//! #     use metro2::*;
//! #     let day = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
//! #     let header = HeaderRecord::builder().activity_date(day).date_created(day).reporter_name("ACME")
//! #         .reporter_address("ADDRESS").reporter_telephone_number(5_551_234_567).build();
//! #     let base = BaseSegment::builder().identification_number("FURNISHER123").consumer_account_number("LOAN0001")
//! #         .portfolio_type(PortfolioType::Installment).account_type(AccountType::Unsecured)
//! #         .date_opened(day).account_status(AccountStatus::Current).date_of_account_information(day)
//! #         .surname("SMITH").first_name("JANE").ecoa_code(EcoaCode::Individual)
//! #         .first_line_of_address("123 MAIN ST").city("ANYTOWN").state("CA").zip_code("90210").build();
//! #     let mut writer = Writer::new(Vec::new());
//! #     let mut file = writer.begin_file(&header)?;
//! #     file.write(&DataRecord::new(base))?;
//! #     file.finish()?;
//! #     Ok(writer.into_inner())
//! # }
//! # let input = sample()?;
//! let file = metro2::File::read(input.as_slice())?;
//! for record in &file.data_records {
//!     println!("{} {}", record.base.consumer_account_number, record.base.account_status);
//! }
//! let issues = file.validate();
//! for issue in &issues {
//!     eprintln!("{issue}"); // e.g. "trailer total_status_code_11 is 2, computed 1"
//! }
//! assert!(issues.is_empty());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! For newline-delimited files use
//! `Reader::new(input).newline(true).read_file()`. Variable-blocked files
//! (block descriptor words) are recognized automatically in either mode. For
//! large files, read record by record with [`Reader`]; see its documentation.
//! Read errors name the input (set with [`Reader::with_name`]) and the byte
//! offset or line, e.g. `report.dat @ offset 426 - truncated J1 segment`.
//!
//! Intentional differences from `lms-python`'s `common/metro2.py`, which
//! this crate replaces, are listed in the repository's `DEVIATIONS.md`.

mod codes;
mod data_record;
mod error;
mod file;
mod profile;
mod reader;
mod records;
mod trailer;
mod validate;
mod writer;

/// Text normalization applied when writing; see [`Writer::sanitize`].
pub use bryl::Sanitize;
pub use codes::{
    AccountStatus, AccountType, AddressIndicator, AgencyIdentifier, ChangeIndicator,
    ComplianceConditionCode, ConsumerInformationIndicator, CreditorClassification, EcoaCode,
    GenerationCode, InterestTypeIndicator, PaymentHistoryCode, PaymentRating, PortfolioType,
    PurchasedIndicator, ResidenceCode, SpecialComment, SpecializedPaymentIndicator, TermsFrequency,
};
pub use data_record::{DataRecord, MAX_RECORD_LENGTH, Segment};
pub use error::Error;
pub use file::{File, Issue};
pub use profile::{PROFILE_MONTHS, PaymentHistoryError, PaymentHistoryProfile};
pub use reader::{DataRecords, Metro2Record, Reader};
pub use records::{
    BaseSegment, HeaderRecord, J1Segment, J2Segment, K1Segment, K2Segment, K3Segment, K4Segment,
    L1Segment, N1Segment, TrailerRecord,
};
pub use validate::{
    CharClass, Violation, is_valid_dob, is_valid_phone, is_valid_ssn, validate_amount_past_due,
    validate_payment_history, validate_payment_rating,
};
pub use writer::{FileWriter, Writer};

/// Runs the code in README.md as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
