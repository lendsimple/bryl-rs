//! Byte-for-byte comparison with files written by lms-python's `metro2.py`
//! (`tools/gen_golden.py`). None of the Metro 2 deviations change written
//! bytes, so the output must match exactly.

mod common;

use common::*;
use metro2::{
    AccountStatus, AddressIndicator, AgencyIdentifier, BaseSegment, ChangeIndicator,
    ComplianceConditionCode, CreditorClassification, DataRecord, EcoaCode, GenerationCode,
    HeaderRecord, J1Segment, J2Segment, K1Segment, K2Segment, K3Segment, K4Segment, L1Segment,
    N1Segment, PaymentRating, PurchasedIndicator, ResidenceCode, SpecialComment,
    SpecializedPaymentIndicator, TermsFrequency, Writer,
};
use pretty_assertions::assert_eq;

fn golden(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/golden/{name}.dat",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn write(records: &[DataRecord], newline: bool) -> String {
    let header = HeaderRecord {
        reporter_telephone_number: 5_551_234_567,
        program_date: Some(date(2019, 5, 10)),
        software_vendor_name: "BRYL".into(),
        ..header()
    };
    let mut writer = Writer::new(Vec::new()).newline(newline);
    let mut file = writer.begin_file(&header).unwrap();
    for record in records {
        file.write(record).unwrap();
    }
    file.finish().unwrap();
    String::from_utf8(writer.into_inner()).unwrap()
}

#[test]
fn base_only() {
    assert_eq!(
        write(&[DataRecord::new(base())], false),
        golden("base_only")
    );
}

#[test]
fn all_segments() {
    let record = DataRecord {
        base: BaseSegment {
            time_stamp: Some(datetime(2020, 8, 20, 14, 30, 45)),
            credit_limit: 50_000,
            highest_credit: 42_000,
            terms_duration: "036".into(),
            terms_frequency: Some(TermsFrequency::Monthly),
            scheduled_monthly_payment: 350,
            actual_payment_amount: 350,
            payment_history_profile: "000000000000BBBBBBBBBBBB".parse().unwrap(),
            current_balance: 12_345,
            date_of_last_payment: Some(date(2020, 8, 1)),
            middle_name: "Q".into(),
            generation_code: Some(GenerationCode::Junior),
            social_security_number: 123_456_789,
            date_of_birth: Some(date(1990, 5, 15)),
            telephone_number: 5_559_876_543,
            country_code: "US".into(),
            address_indicator: Some(AddressIndicator::Confirmed),
            residence_code: Some(ResidenceCode::Owns),
            ..base()
        },
        j1: vec![J1Segment {
            social_security_number: 987_654_321,
            date_of_birth: Some(date(1985, 3, 20)),
            ..j1()
        }],
        j2: vec![J2Segment {
            ecoa_code: EcoaCode::Delete,
            telephone_number: 5_550_001_111,
            ..j2()
        }],
        k1: Some(K1Segment {
            original_creditor_name: "FIRST NATIONAL BANK".into(),
            creditor_classification: CreditorClassification::Banking,
            ..k1()
        }),
        k2: Some(K2Segment {
            purchased_indicator: PurchasedIndicator::SoldTo,
            purchased_from_sold_to_name: "ACME COLLECTIONS".into(),
            ..k2()
        }),
        k3: Some(K3Segment {
            agency_identifier: AgencyIdentifier::FannieMae,
            account_number: "FNM-12345678".into(),
            mortgage_identification_number: "MIN-99887766".into(),
            ..k3()
        }),
        k4: Some(K4Segment {
            specialized_payment_indicator: SpecializedPaymentIndicator::Balloon,
            balloon_payment_due_date: Some(date(2025, 12, 31)),
            balloon_payment_amount: 50_000,
            ..k4()
        }),
        l1: Some(L1Segment {
            change_indicator: ChangeIndicator::Both,
            new_consumer_account_number: "NEW-ACCT-999".into(),
            new_identification_number: "NEW-ID-888".into(),
            ..l1()
        }),
        n1: Some(N1Segment {
            employer_name: "ACME CORPORATION".into(),
            occupation: "SOFTWARE ENGINEER".into(),
            ..n1()
        }),
    };
    assert_eq!(write(&[record], false), golden("all_segments"));
}

fn statuses() -> Vec<DataRecord> {
    vec![
        DataRecord::new(BaseSegment {
            consumer_account_number: "ACCT-001".into(),
            ..base()
        }),
        DataRecord::new(BaseSegment {
            consumer_account_number: "ACCT-002".into(),
            account_status: AccountStatus::ChargeOff,
            payment_rating: Some(PaymentRating::ChargeOff),
            amount_past_due: 1500,
            original_charge_off_amount: 1500,
            date_of_first_delinquency: Some(date(2020, 1, 15)),
            special_comment: Some(SpecialComment::Au),
            ..base()
        }),
        DataRecord::new(BaseSegment {
            consumer_account_number: "ACCT-003".into(),
            account_status: AccountStatus::DeleteAccount,
            payment_rating: None,
            ecoa_code: EcoaCode::Delete,
            compliance_condition_code: Some(ComplianceConditionCode::FcraDispute),
            date_closed: Some(date(2020, 7, 31)),
            ..base()
        }),
    ]
}

#[test]
fn statuses_rdw() {
    assert_eq!(write(&statuses(), false), golden("statuses"));
}

#[test]
fn statuses_newline() {
    assert_eq!(write(&statuses(), true), golden("statuses_newline"));
}

#[test]
fn python_files_validate() {
    for name in ["base_only", "all_segments", "statuses"] {
        let file = metro2::File::read(golden(name).as_bytes()).unwrap();
        assert_eq!(file.validate(), [], "{name}");
    }
}
