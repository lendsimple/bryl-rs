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
//! A base segment and its appended segments form a [`DataRecord`], whose
//! length is written in the base segment's record descriptor word.
//!
//! Write files with [`Writer`], which validates base segments and computes
//! the trailer. Read them with [`Reader`], or all at once with
//! [`File::read`], and check them with [`File::validate`].
//!
//! This is a port of `lms-python`'s `common/metro2.py`. Intentional
//! differences are listed in the repository's `DEVIATIONS.md`.

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
    Violation, is_valid_dob, is_valid_phone, is_valid_ssn, validate_amount_past_due,
    validate_payment_history, validate_payment_rating,
};
pub use writer::{FileWriter, Writer};
