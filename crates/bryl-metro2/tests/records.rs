//! Records and segments: code tables (`codes`), header (`header`), base
//! segment with its dates and timestamps (`base`), appended segments
//! (`segments`), trailer (`trailer`) and field offsets (`offsets`).

mod common;

use bryl::Record;
use common::*;
use metro2::{
    AccountStatus, AccountType, AddressIndicator, AgencyIdentifier, BaseSegment, ChangeIndicator,
    ComplianceConditionCode, ConsumerInformationIndicator, CreditorClassification, EcoaCode,
    GenerationCode, HeaderRecord, InterestTypeIndicator, J1Segment, J2Segment, K1Segment,
    K2Segment, K3Segment, K4Segment, L1Segment, N1Segment, PaymentHistoryCode, PaymentRating,
    PortfolioType, PurchasedIndicator, ResidenceCode, SpecialComment, SpecializedPaymentIndicator,
    TermsFrequency, TrailerRecord,
};

fn offset<R: Record>(name: &str) -> usize {
    R::field(name)
        .unwrap_or_else(|| panic!("no field {name}"))
        .offset
}

mod codes {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn lookup() {
        assert_eq!(PortfolioType::Installment.as_code(), "I");
        assert_eq!(PortfolioType::Mortgage.as_code(), "M");
        assert_eq!("NONEXISTENT".parse::<PortfolioType>().is_err(), true);
    }

    #[test]
    fn table_sizes() {
        assert_eq!(PortfolioType::ALL.len(), 5);
        assert_eq!(AccountType::ALL.len(), 66);
        assert_eq!(AccountStatus::ALL.len(), 23);
        assert_eq!(PaymentRating::ALL.len(), 9);
        // The blank code is a space in the profile, not a variant.
        assert_eq!(PaymentHistoryCode::ALL.len(), 16);
        assert_eq!(EcoaCode::ALL.len(), 9);
        assert_eq!(ConsumerInformationIndicator::ALL.len(), 25);
        assert_eq!(ComplianceConditionCode::ALL.len(), 10);
        assert_eq!(SpecialComment::ALL.len(), 51);
        assert_eq!(TermsFrequency::ALL.len(), 11);
        assert_eq!(AddressIndicator::ALL.len(), 9);
        assert_eq!(CreditorClassification::ALL.len(), 15);
        assert_eq!(GenerationCode::ALL.len(), 10);
        assert_eq!(ResidenceCode::ALL.len(), 2);
        assert_eq!(InterestTypeIndicator::ALL.len(), 2);
        assert_eq!(ChangeIndicator::ALL.len(), 3);
        assert_eq!(PurchasedIndicator::ALL.len(), 3);
        assert_eq!(AgencyIdentifier::ALL.len(), 3);
        assert_eq!(SpecializedPaymentIndicator::ALL.len(), 2);
    }

    #[test]
    fn values() {
        assert_eq!(AccountStatus::Current.as_code(), "11");
        assert_eq!(AccountStatus::ChargeOff.as_code(), "97");
        assert_eq!(AccountStatus::DeleteAccount.as_code(), "DA");
        assert_eq!(PaymentRating::Current.as_code(), "0");
        assert_eq!(PaymentRating::Collection.as_code(), "G");
        assert_eq!(PaymentRating::ChargeOff.as_code(), "L");
        assert_eq!(EcoaCode::Individual.as_code(), "1");
        assert_eq!(EcoaCode::Joint.as_code(), "2");
        assert_eq!(EcoaCode::Delete.as_code(), "Z");
        assert_eq!(TermsFrequency::Monthly.as_code(), "M");
        assert_eq!(TermsFrequency::Biweekly.as_code(), "B");
        assert_eq!(InterestTypeIndicator::Fixed.as_code(), "F");
        assert_eq!(InterestTypeIndicator::Variable.as_code(), "V");
        assert_eq!(ResidenceCode::Owns.as_code(), "O");
        assert_eq!(ResidenceCode::Rents.as_code(), "R");
        assert_eq!(ChangeIndicator::AccountNumber.as_code(), 1);
        assert_eq!(ChangeIndicator::Both.as_code(), 3);
        assert_eq!(PurchasedIndicator::PurchasedFrom.as_code(), 1);
        assert_eq!(PurchasedIndicator::Remove.as_code(), 9);
        assert_eq!(AgencyIdentifier::FannieMae.as_code(), 1);
        assert_eq!(AgencyIdentifier::FreddieMac.as_code(), 2);
        assert_eq!(SpecializedPaymentIndicator::Balloon.as_code(), 1);
        assert_eq!(SpecializedPaymentIndicator::Deferred.as_code(), 2);
        assert_eq!(GenerationCode::Third.as_code(), "3");
    }

    #[test]
    fn account_status_names_match_their_codes() {
        // These names were wrong in metro2.py (e.g. 61 was
        // VOLUNTARY_SURRENDER, DF was DEFERRED). Pin them so a rename can't
        // silently move a name to another code.
        for (status, code) in [
            (AccountStatus::PaidVoluntarySurrender, "61"),
            (AccountStatus::PaidCollection, "62"),
            (AccountStatus::PaidRepossession, "63"),
            (AccountStatus::PaidChargeOff, "64"),
            (AccountStatus::PaidForeclosureStarted, "65"),
            (AccountStatus::GovernmentClaimFiled, "88"),
            (AccountStatus::ForeclosureCompleted, "94"),
            (AccountStatus::VoluntarySurrender, "95"),
            (AccountStatus::Repossession, "96"),
            (AccountStatus::DeleteAccountFraud, "DF"),
        ] {
            assert_eq!(status.as_code(), code, "{status:?}");
        }
    }
}

mod header {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn length() {
        assert_eq!(HeaderRecord::LENGTH, 426);
        assert_eq!(header().encode().unwrap().len(), 426);
    }

    #[test]
    fn constants() {
        assert_eq!(HeaderRecord::RECORD_DESCRIPTOR_WORD, 426);
        assert_eq!(HeaderRecord::RECORD_IDENTIFIER, "HEADER");
        assert!(header().encode().unwrap().starts_with("0426HEADER"));
    }

    #[test]
    fn roundtrip() {
        let h2 = HeaderRecord::decode(header().encode().unwrap().as_bytes()).unwrap();
        assert_eq!(h2.activity_date, date(2020, 8, 20));
        assert_eq!(h2.date_created, date(2020, 8, 20));
        assert_eq!(h2.reporter_name, "TEST REPORTER");
        assert_eq!(h2.reporter_telephone_number, 1_234_567_890);
        assert_eq!(h2, header());
    }

    #[test]
    fn optional_dates_default_to_none_and_dump_as_zeros() {
        let h = header();
        assert_eq!(h.program_date, None);
        assert_eq!(h.program_revision_date, None);
        assert_eq!(&h.encode().unwrap()[63..79], "0".repeat(16));
    }

    #[test]
    fn optional_date_roundtrip() {
        let h = HeaderRecord {
            program_date: Some(date(2019, 5, 10)),
            ..header()
        };
        let h2 = HeaderRecord::decode(h.encode().unwrap().as_bytes()).unwrap();
        assert_eq!(h2.program_date, Some(date(2019, 5, 10)));
    }

    #[test]
    fn optional_alphanumeric_defaults_to_blank() {
        assert_eq!(header().cycle_identifier, "");
        assert_eq!(header().software_vendor_name, "");
    }

    #[test]
    fn clone() {
        let h = header();
        let h2 = h.clone();
        assert_eq!(h2.activity_date, h.activity_date);
    }
}

mod base {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn length() {
        assert_eq!(BaseSegment::LENGTH, 426);
        assert_eq!(base().encode().unwrap().len(), 426);
    }

    #[test]
    fn constants() {
        assert_eq!(BaseSegment::PROCESSING_INDICATOR, 1);
        assert_eq!(base().record_descriptor_word, 426);
    }

    #[test]
    fn roundtrip() {
        let b = BaseSegment {
            time_stamp: Some(datetime(2020, 1, 1, 11, 22, 33)),
            credit_limit: 50_000,
            current_balance: 12_345,
            social_security_number: 123_456_789,
            ..base()
        };
        let b2 = BaseSegment::decode(b.encode().unwrap().as_bytes()).unwrap();
        assert_eq!(b2, b);
        assert_eq!(b2.identification_number, "FURNISHER123");
        assert_eq!(b2.consumer_account_number, "ACCT000001");
        assert_eq!(b2.portfolio_type, PortfolioType::Installment);
        assert_eq!(b2.account_type, AccountType::Unsecured);
        assert_eq!(b2.date_opened, date(2019, 6, 15));
        assert_eq!(b2.social_security_number, 123_456_789);
    }

    #[test]
    fn timestamp() {
        let ts = datetime(2020, 8, 20, 14, 30, 45);
        let b = BaseSegment {
            time_stamp: Some(ts),
            ..base()
        };
        let b2 = BaseSegment::decode(b.encode().unwrap().as_bytes()).unwrap();
        assert_eq!(b2.time_stamp, Some(ts));
    }

    #[test]
    fn timestamp_format() {
        let b = BaseSegment {
            time_stamp: Some(datetime(2020, 1, 15, 9, 5, 30)),
            ..base()
        };
        assert_eq!(&b.encode().unwrap()[5..19], "01152020090530");
    }

    #[test]
    fn timestamp_defaults_to_none_and_zeros() {
        let b = base();
        assert_eq!(b.time_stamp, None);
        let raw = b.encode().unwrap();
        assert_eq!(&raw[5..19], "0".repeat(14));
        assert_eq!(
            BaseSegment::decode(raw.as_bytes()).unwrap().time_stamp,
            None
        );
    }

    #[test]
    fn date_fields_roundtrip() {
        let b = BaseSegment {
            date_closed: Some(date(2020, 3, 15)),
            date_of_last_payment: Some(date(2020, 7, 1)),
            date_of_first_delinquency: Some(date(2020, 2, 1)),
            ..base()
        };
        let b2 = BaseSegment::decode(b.encode().unwrap().as_bytes()).unwrap();
        assert_eq!(b2.date_closed, Some(date(2020, 3, 15)));
        assert_eq!(b2.date_of_last_payment, Some(date(2020, 7, 1)));
        assert_eq!(b2.date_of_first_delinquency, Some(date(2020, 2, 1)));
    }

    #[test]
    fn zero_dates() {
        let b2 = BaseSegment::decode(base().encode().unwrap().as_bytes()).unwrap();
        assert_eq!(b2.date_closed, None);
        assert_eq!(b2.date_of_first_delinquency, None);
        assert_eq!(b2.date_of_last_payment, None);
    }

    #[test]
    fn numerics_default_to_zero() {
        let b = base();
        for value in [
            b.credit_limit,
            b.highest_credit,
            b.scheduled_monthly_payment,
            b.actual_payment_amount,
            b.current_balance,
            b.amount_past_due,
            b.original_charge_off_amount,
        ] {
            assert_eq!(value, 0);
        }
    }

    #[test]
    fn text_is_uppercased_on_write_not_read() {
        let b = BaseSegment {
            surname: "smith".into(),
            ..base()
        };
        let raw = b.encode().unwrap();
        assert_eq!(&raw[231..236], "SMITH");
        // Decoding does not change case.
        let mut lower = raw.clone();
        lower.replace_range(231..236, "smith");
        assert_eq!(
            BaseSegment::decode(lower.as_bytes()).unwrap().surname,
            "smith"
        );
    }

    #[test]
    fn optional_codes() {
        let b = BaseSegment {
            special_comment: Some(SpecialComment::Au),
            compliance_condition_code: Some(ComplianceConditionCode::FcraDispute),
            terms_frequency: Some(TermsFrequency::Monthly),
            ..base()
        };
        let b2 = BaseSegment::decode(b.encode().unwrap().as_bytes()).unwrap();
        assert_eq!(b2, b);
        assert_eq!(
            BaseSegment::decode(base().encode().unwrap().as_bytes())
                .unwrap()
                .special_comment,
            None
        );
    }

    #[test]
    fn unknown_code_rejected_on_read() {
        let mut raw = base().encode().unwrap();
        raw.replace_range(123..125, "99");
        let err = BaseSegment::decode(raw.as_bytes()).unwrap_err();
        assert!(matches!(
            err,
            bryl::Error::Field {
                field: "account_status",
                kind: bryl::FieldErrorKind::UnknownCode(_),
                ..
            }
        ));
    }

    #[test]
    fn reserved_fields_must_be_blank() {
        let mut raw = base().encode().unwrap();
        raw.replace_range(19..20, "5");
        assert!(BaseSegment::decode(raw.as_bytes()).is_err());
    }
}

mod segments {
    use super::*;
    use pretty_assertions::assert_eq;

    fn roundtrip<R: Record + PartialEq + std::fmt::Debug>(record: &R) -> R {
        let raw = record.encode().unwrap();
        assert_eq!(raw.len(), R::LENGTH);
        let decoded = R::decode(raw.as_bytes()).unwrap();
        assert_eq!(&decoded, record);
        decoded
    }

    #[test]
    fn lengths() {
        assert_eq!(J1Segment::LENGTH, 100);
        assert_eq!(J2Segment::LENGTH, 200);
        assert_eq!(K1Segment::LENGTH, 34);
        assert_eq!(K2Segment::LENGTH, 34);
        assert_eq!(K3Segment::LENGTH, 40);
        assert_eq!(K4Segment::LENGTH, 30);
        assert_eq!(L1Segment::LENGTH, 54);
        assert_eq!(N1Segment::LENGTH, 146);
    }

    #[test]
    fn identifiers() {
        assert_eq!(J1Segment::SEGMENT_IDENTIFIER, "J1");
        assert_eq!(J2Segment::SEGMENT_IDENTIFIER, "J2");
        assert_eq!(K1Segment::SEGMENT_IDENTIFIER, "K1");
        assert_eq!(K2Segment::SEGMENT_IDENTIFIER, "K2");
        assert_eq!(K3Segment::SEGMENT_IDENTIFIER, "K3");
        assert_eq!(K4Segment::SEGMENT_IDENTIFIER, "K4");
        assert_eq!(L1Segment::SEGMENT_IDENTIFIER, "L1");
        assert_eq!(N1Segment::SEGMENT_IDENTIFIER, "N1");
        assert!(common::j1().encode().unwrap().starts_with("J1"));
    }

    #[test]
    fn j1() {
        let segment = J1Segment {
            social_security_number: 987_654_321,
            date_of_birth: Some(date(1985, 3, 15)),
            telephone_number: 5_551_234_567,
            ..common::j1()
        };
        let decoded = roundtrip(&segment);
        assert_eq!(decoded.surname, "SMITH");
        assert_eq!(decoded.ecoa_code, EcoaCode::Joint);
    }

    #[test]
    fn j1_defaults() {
        let segment = common::j1();
        assert_eq!(segment.social_security_number, 0);
        assert_eq!(segment.date_of_birth, None);
        assert_eq!(segment.telephone_number, 0);
    }

    #[test]
    fn j2() {
        let segment = J2Segment {
            social_security_number: 111_223_333,
            country_code: "US".into(),
            address_indicator: Some(AddressIndicator::Confirmed),
            residence_code: Some(ResidenceCode::Owns),
            ..common::j2()
        };
        let decoded = roundtrip(&segment);
        assert_eq!(decoded.city, "OTHERTOWN");
        assert_eq!(decoded.residence_code, Some(ResidenceCode::Owns));
    }

    #[test]
    fn k1() {
        let segment = K1Segment {
            original_creditor_name: "FIRST NATIONAL BANK".into(),
            ..common::k1()
        };
        assert_eq!(
            roundtrip(&segment).creditor_classification,
            CreditorClassification::Banking
        );
    }

    #[test]
    fn k2() {
        let segment = K2Segment {
            purchased_indicator: PurchasedIndicator::SoldTo,
            purchased_from_sold_to_name: "ACME COLLECTIONS".into(),
            ..common::k2()
        };
        assert_eq!(
            roundtrip(&segment).purchased_indicator,
            PurchasedIndicator::SoldTo
        );
    }

    #[test]
    fn k3() {
        let segment = K3Segment {
            agency_identifier: AgencyIdentifier::FannieMae,
            account_number: "FNM-12345678".into(),
            mortgage_identification_number: "MIN-99887766".into(),
            ..common::k3()
        };
        assert_eq!(roundtrip(&segment).account_number, "FNM-12345678");
        assert_eq!(
            &common::k3().encode().unwrap()[2..4],
            "00",
            "agency 0 is written"
        );
    }

    #[test]
    fn k4() {
        let segment = K4Segment {
            balloon_payment_due_date: Some(date(2025, 12, 31)),
            balloon_payment_amount: 50_000,
            ..common::k4()
        };
        assert_eq!(roundtrip(&segment).balloon_payment_amount, 50_000);
        let deferred = K4Segment {
            specialized_payment_indicator: SpecializedPaymentIndicator::Deferred,
            ..common::k4()
        };
        assert_eq!(deferred.deferred_payment_start_date, None);
        assert_eq!(deferred.balloon_payment_due_date, None);
    }

    #[test]
    fn l1() {
        let segment = L1Segment {
            change_indicator: ChangeIndicator::Both,
            new_consumer_account_number: "NEWACCT999".into(),
            new_identification_number: "NEWID888".into(),
            ..common::l1()
        };
        assert_eq!(roundtrip(&segment).new_identification_number, "NEWID888");
    }

    #[test]
    fn n1() {
        let segment = N1Segment::builder()
            .employer_name("ACME CORPORATION")
            .first_line_of_employer_address("100 INDUSTRIAL BLVD")
            .employer_city("METROPOLIS")
            .employer_state("IL")
            .zip_code("60601")
            .occupation("SOFTWARE ENGINEER")
            .build();
        assert_eq!(roundtrip(&segment).occupation, "SOFTWARE ENGINEER");
    }
}

mod trailer {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn length_and_constants() {
        assert_eq!(TrailerRecord::LENGTH, 426);
        assert_eq!(TrailerRecord::RECORD_IDENTIFIER, "TRAILER");
        let raw = TrailerRecord::default().encode().unwrap();
        assert_eq!(raw.len(), 426);
        assert!(raw.starts_with("0426TRAILER"));
    }

    #[test]
    fn counters_default_to_zero() {
        let t = TrailerRecord::default();
        assert_eq!(
            [
                t.total_base_records,
                t.block_count,
                t.total_status_code_11,
                t.total_ecoa_code_z
            ],
            [0; 4]
        );
    }

    #[test]
    fn roundtrip() {
        let t = TrailerRecord {
            total_base_records: 100,
            total_j1_segments: 25,
            total_j2_segments: 10,
            block_count: 102,
            total_status_code_11: 80,
            total_status_code_71: 15,
            total_status_code_97: 5,
            total_ssns_all_segments: 135,
            total_ssns_base_segments: 100,
            total_ssns_j1_segments: 25,
            total_ssns_j2_segments: 10,
            ..TrailerRecord::default()
        };
        assert_eq!(
            TrailerRecord::decode(t.encode().unwrap().as_bytes()).unwrap(),
            t
        );
    }

    #[test]
    fn reserved_fields_are_blank() {
        let raw = TrailerRecord::default().encode().unwrap();
        assert_eq!(&raw[20..29], " ".repeat(9));
        assert_eq!(&raw[407..], " ".repeat(19));
    }
}

mod offsets {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn header() {
        for (name, expected) in [
            ("record_descriptor_word", 0),
            ("record_identifier", 4),
            ("cycle_identifier", 10),
            ("activity_date", 47),
            ("reporter_name", 79),
            ("reporter_address", 119),
            ("reporter_telephone_number", 215),
            ("reserved", 280),
        ] {
            assert_eq!(offset::<HeaderRecord>(name), expected, "{name}");
        }
    }

    #[test]
    fn base() {
        for (name, expected) in [
            ("record_descriptor_word", 0),
            ("processing_indicator", 4),
            ("time_stamp", 5),
            ("reserved_1", 19),
            ("identification_number", 20),
            ("consumer_account_number", 42),
            ("portfolio_type", 72),
            ("account_type", 73),
            ("date_opened", 75),
            ("account_status", 123),
            ("payment_history_profile", 126),
            ("current_balance", 154),
            ("date_of_account_information", 181),
            ("surname", 231),
            ("first_name", 256),
            ("social_security_number", 297),
            ("date_of_birth", 306),
            ("ecoa_code", 324),
            ("first_line_of_address", 329),
            ("city", 393),
            ("state", 413),
            ("zip_code", 415),
            ("residence_code", 425),
        ] {
            assert_eq!(offset::<BaseSegment>(name), expected, "{name}");
        }
    }

    #[test]
    fn j1() {
        for (name, expected) in [
            ("segment_identifier", 0),
            ("reserved_1", 2),
            ("surname", 3),
            ("social_security_number", 69),
            ("date_of_birth", 78),
            ("ecoa_code", 96),
            ("reserved_2", 99),
        ] {
            assert_eq!(offset::<J1Segment>(name), expected, "{name}");
        }
    }

    #[test]
    fn j2() {
        for (name, expected) in [
            ("segment_identifier", 0),
            ("surname", 3),
            ("country_code", 99),
            ("first_line_of_address", 101),
            ("residence_code", 197),
            ("reserved_2", 198),
        ] {
            assert_eq!(offset::<J2Segment>(name), expected, "{name}");
        }
    }

    #[test]
    fn trailer() {
        for (name, expected) in [
            ("record_descriptor_word", 0),
            ("record_identifier", 4),
            ("total_base_records", 11),
            ("reserved_1", 20),
            ("total_status_code_df", 29),
            ("total_j1_segments", 38),
            ("total_j2_segments", 47),
            ("block_count", 56),
            ("total_ecoa_code_z", 263),
            ("total_telephone_numbers", 398),
            ("reserved_2", 407),
        ] {
            assert_eq!(offset::<TrailerRecord>(name), expected, "{name}");
        }
    }
}
