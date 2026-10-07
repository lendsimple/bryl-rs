//! Snapshot tests: the writer's output for fixed scenarios must match the
//! committed files in `tests/fixtures/snapshots` byte for byte, so any change
//! to written bytes shows up as a reviewable diff.
//!
//! Regenerate with `UPDATE_SNAPSHOTS=1 cargo test -p bryl-metro2 --test snapshots`.

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

/// Compares `actual` with the snapshot `tests/fixtures/snapshots/{name}`.
/// Run with `UPDATE_SNAPSHOTS=1` to write the snapshot instead, and review
/// the diff before committing it.
fn assert_snapshot(name: &str, actual: &str) {
    let path = format!(
        "{}/tests/fixtures/snapshots/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing snapshot {path}; run with UPDATE_SNAPSHOTS=1"));
    assert_eq!(actual, expected, "{name} differs from its snapshot");
}

fn read_snapshot(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/snapshots/{name}",
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
    assert_snapshot("base_only.dat", &write(&[DataRecord::new(base())], false));
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
            new_consumer_account_number: "NEWACCT999".into(),
            new_identification_number: "NEWID888".into(),
            ..l1()
        }),
        n1: Some(N1Segment {
            employer_name: "ACME CORPORATION".into(),
            occupation: "SOFTWARE ENGINEER".into(),
            ..n1()
        }),
    };
    assert_snapshot("all_segments.dat", &write(&[record], false));
}

fn statuses() -> Vec<DataRecord> {
    vec![
        DataRecord::new(BaseSegment {
            consumer_account_number: "ACCT001".into(),
            ..base()
        }),
        DataRecord::new(BaseSegment {
            consumer_account_number: "ACCT002".into(),
            account_status: AccountStatus::ChargeOff,
            amount_past_due: 1500,
            original_charge_off_amount: 1500,
            date_of_first_delinquency: Some(date(2020, 1, 15)),
            special_comment: Some(SpecialComment::Au),
            ..base()
        }),
        DataRecord::new(BaseSegment {
            consumer_account_number: "ACCT003".into(),
            account_status: AccountStatus::DeleteAccount,
            ecoa_code: EcoaCode::Delete,
            compliance_condition_code: Some(ComplianceConditionCode::FcraDispute),
            date_closed: Some(date(2020, 7, 31)),
            ..base()
        }),
        DataRecord::new(BaseSegment {
            consumer_account_number: "ACCT004".into(),
            account_status: AccountStatus::PaidOrClosed,
            payment_rating: Some(PaymentRating::Past30),
            date_closed: Some(date(2020, 6, 30)),
            ..base()
        }),
    ]
}

#[test]
fn statuses_rdw() {
    assert_snapshot("statuses.dat", &write(&statuses(), false));
}

#[test]
fn statuses_newline() {
    assert_snapshot("statuses_newline.dat", &write(&statuses(), true));
}

#[test]
fn snapshots_read_back_and_validate() {
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        return; // the snapshots are being rewritten by the other tests
    }
    for name in ["base_only", "all_segments", "statuses"] {
        let file = metro2::File::read(read_snapshot(&format!("{name}.dat")).as_bytes()).unwrap();
        assert_eq!(file.validate(), [], "{name}");
    }
    let newline = metro2::Reader::new(read_snapshot("statuses_newline.dat").as_bytes())
        .newline(true)
        .read_file()
        .unwrap();
    assert_eq!(newline.validate(), []);
}
