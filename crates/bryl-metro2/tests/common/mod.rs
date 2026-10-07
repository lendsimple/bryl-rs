//! Shared test fixtures.

#![allow(dead_code)]

use chrono::{NaiveDate, NaiveDateTime};
use metro2::{
    AccountStatus, AccountType, AgencyIdentifier, BaseSegment, ChangeIndicator,
    CreditorClassification, DataRecord, EcoaCode, HeaderRecord, J1Segment, J2Segment, K1Segment,
    K2Segment, K3Segment, K4Segment, L1Segment, N1Segment, PaymentRating, PortfolioType,
    PurchasedIndicator, SpecializedPaymentIndicator, Writer,
};

pub fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

pub fn datetime(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
    date(y, mo, d).and_hms_opt(h, mi, s).unwrap()
}

/// A minimal valid header.
pub fn header() -> HeaderRecord {
    HeaderRecord::builder()
        .activity_date(date(2020, 8, 20))
        .date_created(date(2020, 8, 20))
        .reporter_name("TEST REPORTER")
        .reporter_address("123 MAIN ST ANYTOWN US 12345")
        .reporter_telephone_number(1_234_567_890)
        .build()
}

/// A minimal valid base segment: a current installment account.
pub fn base() -> BaseSegment {
    BaseSegment::builder()
        .identification_number("FURNISHER123")
        .consumer_account_number("ACCT000001")
        .portfolio_type(PortfolioType::Installment)
        .account_type(AccountType::Unsecured)
        .date_opened(date(2019, 6, 15))
        .account_status(AccountStatus::Current)
        .date_of_account_information(date(2020, 8, 20))
        .surname("SMITH")
        .first_name("JOHN")
        .ecoa_code(EcoaCode::Individual)
        .first_line_of_address("123 MAIN ST")
        .city("ANYTOWN")
        .state("CA")
        .zip_code("90210")
        .build()
}

/// A base segment with `status`, and a payment rating if the status requires one.
pub fn base_with_status(status: AccountStatus) -> BaseSegment {
    BaseSegment {
        account_status: status,
        payment_rating: status
            .requires_payment_rating()
            .then_some(PaymentRating::Current),
        ..base()
    }
}

/// A minimal J1 segment (co-borrower at the same address).
pub fn j1() -> J1Segment {
    J1Segment::builder()
        .surname("SMITH")
        .first_name("JANE")
        .ecoa_code(EcoaCode::Joint)
        .build()
}

/// A minimal J2 segment (co-borrower at a different address).
pub fn j2() -> J2Segment {
    J2Segment::builder()
        .surname("JONES")
        .first_name("BOB")
        .ecoa_code(EcoaCode::Joint)
        .first_line_of_address("456 OAK AVE")
        .city("OTHERTOWN")
        .state("NY")
        .zip_code("10001")
        .build()
}

pub fn k1() -> K1Segment {
    K1Segment::builder()
        .original_creditor_name("BANK")
        .creditor_classification(CreditorClassification::Banking)
        .build()
}

pub fn k2() -> K2Segment {
    K2Segment::builder()
        .purchased_indicator(PurchasedIndicator::PurchasedFrom)
        .purchased_from_sold_to_name("BUYER")
        .build()
}

pub fn k3() -> K3Segment {
    K3Segment::builder()
        .agency_identifier(AgencyIdentifier::NotApplicable)
        .account_number("X")
        .mortgage_identification_number("Y")
        .build()
}

pub fn k4() -> K4Segment {
    K4Segment::builder()
        .specialized_payment_indicator(SpecializedPaymentIndicator::Balloon)
        .build()
}

pub fn l1() -> L1Segment {
    L1Segment::builder()
        .change_indicator(ChangeIndicator::AccountNumber)
        .new_consumer_account_number("NEW")
        .build()
}

pub fn n1() -> N1Segment {
    N1Segment::builder().employer_name("ACME").build()
}

/// A data record with every segment type.
pub fn all_segments() -> DataRecord {
    DataRecord {
        j1: vec![j1()],
        j2: vec![j2()],
        k1: Some(k1()),
        k2: Some(k2()),
        k3: Some(k3()),
        k4: Some(k4()),
        l1: Some(l1()),
        n1: Some(n1()),
        ..DataRecord::new(base())
    }
}

pub const ALL_SEGMENTS_LENGTH: usize = 426 + 100 + 200 + 34 + 34 + 40 + 30 + 54 + 146;

/// Writes a complete file (header, `records`, trailer) and returns it.
pub fn write_file(records: &[DataRecord], newline: bool) -> String {
    let mut writer = Writer::new(Vec::new()).newline(newline);
    let header = HeaderRecord {
        reporter_telephone_number: 5_551_234_567,
        ..header()
    };
    let mut file = writer.begin_file(&header).unwrap();
    for record in records {
        file.write(record).unwrap();
    }
    file.finish().unwrap();
    String::from_utf8(writer.into_inner()).unwrap()
}
