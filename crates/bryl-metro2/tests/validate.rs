//! Validation: the SSN, phone and date-of-birth helpers (`helpers`), payment
//! rating (`payment_rating`), amount past due (`amount_past_due`), date
//! closed and date of first delinquency (`dates`), payment history profile
//! (`payment_history`), `BaseSegment::validate` (`base_segment`) and
//! character rules (`characters`).

mod common;

use common::*;
use metro2::{
    AccountStatus, BaseSegment, DataRecord, PaymentHistoryError, PaymentHistoryProfile,
    PaymentRating, Violation, is_valid_dob, is_valid_phone, is_valid_ssn, validate_amount_past_due,
    validate_date_closed, validate_date_of_first_delinquency, validate_payment_history,
    validate_payment_rating,
};

mod helpers {
    use super::*;

    #[test]
    fn ssn() {
        assert!(is_valid_ssn(123_456_789));
        assert!(!is_valid_ssn(0));
        assert!(!is_valid_ssn(999_999_999));
        assert!(is_valid_ssn(1));
        assert!(is_valid_ssn(999_999_998));
    }

    #[test]
    fn phone() {
        assert!(is_valid_phone(5_551_234_567));
        assert!(!is_valid_phone(0));
    }

    #[test]
    fn dob() {
        assert!(is_valid_dob(Some(date(1990, 1, 1))));
        assert!(!is_valid_dob(None));
    }
}

mod payment_rating {
    //! A rating is required for statuses 05, 13, 65, 88, 89, 94 and 95, and
    //! must be blank otherwise.
    use super::*;
    use pretty_assertions::assert_eq;

    const REQUIRED: [&str; 7] = ["05", "13", "65", "88", "89", "94", "95"];

    #[test]
    fn statuses_requiring_a_rating() {
        let required: Vec<_> = AccountStatus::ALL
            .iter()
            .filter(|s| s.requires_payment_rating())
            .map(AccountStatus::as_code)
            .collect();
        assert_eq!(required, REQUIRED);
    }

    #[test]
    fn any_rating_when_required() {
        for &rating in PaymentRating::ALL {
            assert_eq!(
                validate_payment_rating(AccountStatus::PaidOrClosed, Some(rating)),
                Ok(())
            );
            assert_eq!(
                validate_payment_rating(AccountStatus::Transferred, Some(rating)),
                Ok(())
            );
        }
    }

    #[test]
    fn missing_when_required() {
        let err = validate_payment_rating(AccountStatus::PaidOrClosed, None).unwrap_err();
        assert_eq!(
            err,
            Violation::PaymentRatingRequired {
                status: AccountStatus::PaidOrClosed
            }
        );
        assert_eq!(
            err.to_string(),
            "Account status '13' requires a payment rating"
        );
    }

    #[test]
    fn blank_otherwise() {
        for &status in AccountStatus::ALL {
            if !status.requires_payment_rating() {
                assert_eq!(validate_payment_rating(status, None), Ok(()), "{status:?}");
            }
        }
    }

    #[test]
    fn present_when_not_allowed() {
        // A rating with a status that does not take one is rejected.
        for (status, rating) in [
            (AccountStatus::Current, PaymentRating::Current),
            (AccountStatus::Dpd30, PaymentRating::Past30),
            (AccountStatus::ChargeOff, PaymentRating::ChargeOff),
        ] {
            assert_eq!(
                validate_payment_rating(status, Some(rating)),
                Err(Violation::PaymentRatingNotAllowed { status, rating })
            );
        }
        let err = validate_payment_rating(AccountStatus::Current, Some(PaymentRating::Past30))
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Payment rating must be blank for account status '11', got '1'"
        );
    }
}

mod amount_past_due {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn zero_for_current_and_paid() {
        assert_eq!(validate_amount_past_due(AccountStatus::Current, 0), Ok(()));
        assert_eq!(
            validate_amount_past_due(AccountStatus::PaidOrClosed, 0),
            Ok(())
        );
    }

    #[test]
    fn nonzero_allowed_when_delinquent() {
        assert_eq!(validate_amount_past_due(AccountStatus::Dpd30, 5000), Ok(()));
        assert_eq!(
            validate_amount_past_due(AccountStatus::ChargeOff, 10_000),
            Ok(())
        );
    }

    #[test]
    fn nonzero_for_current_or_paid_rejected() {
        let err = validate_amount_past_due(AccountStatus::Current, 500).unwrap_err();
        assert!(
            err.to_string().contains("must be 0 for status '11'"),
            "{err}"
        );
        let err = validate_amount_past_due(AccountStatus::PaidOrClosed, 100).unwrap_err();
        assert!(
            err.to_string().contains("must be 0 for status '13'"),
            "{err}"
        );
    }
}

mod dates {
    use super::*;
    use metro2::{ConsumerInformationIndicator as Cii, PortfolioType};
    use pretty_assertions::assert_eq;

    #[test]
    fn date_closed_required_for_paid_accounts() {
        // CRRG 2020 FAQ 39: 13 and 61-65.
        let paid: Vec<_> = AccountStatus::ALL
            .iter()
            .copied()
            .filter(|status| status.requires_date_closed())
            .collect();
        assert_eq!(
            paid,
            [
                AccountStatus::PaidOrClosed,
                AccountStatus::PaidVoluntarySurrender,
                AccountStatus::PaidCollection,
                AccountStatus::PaidRepossession,
                AccountStatus::PaidChargeOff,
                AccountStatus::PaidForeclosureStarted,
            ]
        );
        for status in paid {
            let installment = PortfolioType::Installment;
            assert_eq!(
                validate_date_closed(status, installment, Some(date(2020, 6, 30))),
                Ok(())
            );
            let err = validate_date_closed(status, installment, None).unwrap_err();
            assert_eq!(err, Violation::DateClosedRequired { status });
        }
    }

    #[test]
    fn date_closed_zero_for_installment_and_mortgage_charge_offs() {
        // CRRG 2020 FAQ 34(a).
        let status = AccountStatus::ChargeOff;
        for portfolio_type in [PortfolioType::Installment, PortfolioType::Mortgage] {
            assert_eq!(validate_date_closed(status, portfolio_type, None), Ok(()));
            assert_eq!(
                validate_date_closed(status, portfolio_type, Some(date(2020, 6, 30))),
                Err(Violation::DateClosedNotAllowed {
                    status,
                    portfolio_type
                })
            );
        }
        // A revolving charge-off reports the date closed to further purchases.
        for portfolio_type in [
            PortfolioType::Revolving,
            PortfolioType::Open,
            PortfolioType::LineOfCredit,
        ] {
            assert_eq!(
                validate_date_closed(status, portfolio_type, Some(date(2020, 6, 30))),
                Ok(())
            );
            assert_eq!(validate_date_closed(status, portfolio_type, None), Ok(()));
        }
    }

    #[test]
    fn date_closed_optional_for_open_statuses() {
        // A current revolving account may be closed with a balance due
        // (CRRG 2020 FAQ 40).
        for closed in [None, Some(date(2020, 6, 30))] {
            assert_eq!(
                validate_date_closed(AccountStatus::Current, PortfolioType::Revolving, closed),
                Ok(())
            );
        }
    }

    /// Whether the CRRG 2020 Field 25 hierarchy requires a date of first
    /// delinquency.
    fn dofd_required(
        status: AccountStatus,
        rating: Option<PaymentRating>,
        cii: Option<Cii>,
    ) -> bool {
        BaseSegment {
            account_status: status,
            payment_rating: rating,
            consumer_information_indicator: cii,
            ..base()
        }
        .requires_date_of_first_delinquency()
    }

    #[test]
    fn dofd_required_for_delinquent_and_derogatory_statuses() {
        // Rule 1.
        let statuses: Vec<_> = AccountStatus::ALL
            .iter()
            .filter(|status| status.requires_date_of_first_delinquency())
            .map(AccountStatus::as_code)
            .collect();
        assert_eq!(
            statuses,
            [
                "61", "62", "63", "64", "65", "71", "78", "80", "82", "83", "84", "88", "89", "93",
                "94", "95", "96", "97"
            ]
        );
        assert!(dofd_required(AccountStatus::ChargeOff, None, None));
        assert!(!dofd_required(AccountStatus::Current, None, None));
        assert!(!dofd_required(AccountStatus::DeleteAccount, None, None));
    }

    #[test]
    fn dofd_required_for_paid_accounts_that_were_late() {
        // Rule 2: 05 and 13 with a delinquent payment rating.
        for status in [AccountStatus::PaidOrClosed, AccountStatus::Transferred] {
            for &rating in PaymentRating::ALL {
                let late = rating != PaymentRating::Current;
                assert_eq!(
                    dofd_required(status, Some(rating), None),
                    late,
                    "{status:?} {rating:?}"
                );
            }
        }
    }

    #[test]
    fn dofd_required_for_bankruptcy_on_current_accounts() {
        // Rule 3: bankruptcy or personal receivership on a current account.
        let bankruptcy = [
            Cii::BankruptcyCh7Petition,
            Cii::BankruptcyCh13Discharged,
            Cii::UndesignatedChapter,
            Cii::PersonalReceivership,
            Cii::RescissionReaffirmation,
        ];
        for cii in bankruptcy {
            assert!(
                dofd_required(AccountStatus::Current, None, Some(cii)),
                "{cii:?}"
            );
            assert!(
                dofd_required(
                    AccountStatus::PaidOrClosed,
                    Some(PaymentRating::Current),
                    Some(cii)
                ),
                "{cii:?}"
            );
        }
        for cii in [
            Cii::BankruptcyCh7Dismissed,
            Cii::RemovePriorBankruptcy,
            Cii::ReaffirmationDebt,
            Cii::CannotLocateConsumer,
            Cii::LeaseAssumption,
        ] {
            assert!(
                !dofd_required(AccountStatus::Current, None, Some(cii)),
                "{cii:?}"
            );
        }
    }

    #[test]
    fn dofd_violations() {
        let charge_off = BaseSegment {
            account_status: AccountStatus::ChargeOff,
            amount_past_due: 1500,
            ..base()
        };
        assert_eq!(
            validate_date_of_first_delinquency(&charge_off),
            Err(Violation::DateOfFirstDelinquencyRequired {
                status: AccountStatus::ChargeOff
            })
        );
        let charge_off = BaseSegment {
            date_of_first_delinquency: Some(date(2020, 1, 15)),
            ..charge_off
        };
        assert_eq!(validate_date_of_first_delinquency(&charge_off), Ok(()));

        // A current account carries no date of first delinquency.
        let current = BaseSegment {
            date_of_first_delinquency: Some(date(2020, 1, 15)),
            ..base()
        };
        let err = validate_date_of_first_delinquency(&current).unwrap_err();
        assert_eq!(
            err,
            Violation::DateOfFirstDelinquencyNotAllowed {
                status: AccountStatus::Current
            }
        );
        assert!(err.to_string().contains("'11'"), "{err}");
    }

    #[test]
    fn data_record_reports_date_rules() {
        let paid = DataRecord::new(BaseSegment {
            account_status: AccountStatus::PaidOrClosed,
            payment_rating: Some(PaymentRating::Past30),
            ..base()
        });
        assert_eq!(
            paid.validate(),
            Err(vec![
                Violation::DateClosedRequired {
                    status: AccountStatus::PaidOrClosed
                },
                Violation::DateOfFirstDelinquencyRequired {
                    status: AccountStatus::PaidOrClosed
                },
            ])
        );
    }
}

mod payment_history {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn valid_profiles() {
        for profile in [
            "000000000000000000000000",
            "000012BBBBBBBBBBBBBBBBBB",
            "0D0D0D000000000000000000",
            "",
            "00B                     ",
        ] {
            assert_eq!(validate_payment_history(profile), Ok(()), "{profile:?}");
        }
    }

    #[test]
    fn invalid_code() {
        let err = validate_payment_history("0X0000000000000000000000").unwrap_err();
        assert_eq!(
            err,
            PaymentHistoryError::InvalidCode {
                code: 'X',
                position: 1
            }
        );
        assert!(err.to_string().contains("Invalid payment history code"));
    }

    #[test]
    fn embedded_b() {
        let err = validate_payment_history("00B100000000000000000000").unwrap_err();
        assert_eq!(
            err,
            PaymentHistoryError::EmbeddedB {
                code: '1',
                position: 3
            }
        );
        assert!(err.to_string().contains("Code 'B' cannot be embedded"));
    }

    #[test]
    fn too_long() {
        assert_eq!(
            validate_payment_history(&"0".repeat(25)),
            Err(PaymentHistoryError::TooLong(25))
        );
    }

    #[test]
    fn profile_type() {
        let profile: PaymentHistoryProfile = "0D1 B".parse().unwrap();
        assert_eq!(profile.as_str(), "0D1 B");
        assert_eq!(profile.months().filter(Option::is_none).count(), 1);
        assert_eq!(PaymentHistoryProfile::default().as_str(), "");
        assert!("0x".parse::<PaymentHistoryProfile>().is_err());
        // The type allows an embedded B; validation reports it.
        assert!("B0".parse::<PaymentHistoryProfile>().is_ok());
    }
}

mod base_segment {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn valid() {
        assert_eq!(base().validate(), Ok(()));
    }

    #[test]
    fn collects_every_violation() {
        let b = BaseSegment {
            payment_rating: Some(PaymentRating::Past30),
            amount_past_due: 10,
            payment_history_profile: "B0".parse().unwrap(),
            ..base()
        };
        let violations = b.validate().unwrap_err();
        assert_eq!(violations.len(), 3);
        assert!(matches!(
            violations[0],
            Violation::PaymentRatingNotAllowed { .. }
        ));
        assert!(matches!(
            violations[1],
            Violation::AmountPastDue { amount: 10, .. }
        ));
        assert!(matches!(
            violations[2],
            Violation::PaymentHistory(PaymentHistoryError::EmbeddedB { .. })
        ));
    }

    #[test]
    fn every_current_status_with_its_required_rating_is_valid() {
        for &status in AccountStatus::ALL {
            if status != AccountStatus::Transferred {
                assert_eq!(base_with_status(status).validate(), Ok(()), "{status:?}");
            }
        }
    }

    #[test]
    fn status_05_is_retired() {
        // Retired by CDIA in April 2022.
        let err = base_with_status(AccountStatus::Transferred)
            .validate()
            .unwrap_err();
        assert_eq!(
            err,
            [Violation::RetiredStatus {
                status: AccountStatus::Transferred
            }]
        );
        assert!(err[0].to_string().contains("special comment AT or O"));
    }
}

mod crrg_rules {
    use super::*;
    use metro2::{
        AccountType, ComplianceConditionCode, File, HeaderRecord, Issue, SpecialComment, Writer,
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn header_needs_reporter_name_and_address() {
        // CRRG 2020 header record layout (page 3-7): required fields.
        assert_eq!(header().validate(), Ok(()));
        let blank = HeaderRecord {
            reporter_name: "   ".into(),
            reporter_address: String::new(),
            ..header()
        };
        assert_eq!(
            blank.validate(),
            Err(vec![
                Violation::HeaderFieldRequired {
                    field: "reporter_name"
                },
                Violation::HeaderFieldRequired {
                    field: "reporter_address"
                },
            ])
        );
    }

    #[test]
    fn writer_rejects_an_invalid_header() {
        let blank = HeaderRecord {
            reporter_name: String::new(),
            ..header()
        };
        let mut writer = Writer::new(Vec::new());
        let err = writer.begin_file(&blank).unwrap_err();
        assert!(matches!(
            &err,
            metro2::Error::InvalidHeader { violations }
                if violations[..] == [Violation::HeaderFieldRequired { field: "reporter_name" }]
        ));
        assert_eq!(writer.into_inner(), b"");
    }

    #[test]
    fn file_validation_reports_the_header() {
        let mut writer = Writer::new(Vec::new());
        let mut file = writer.begin_file(&header()).unwrap();
        file.write(&DataRecord::new(base())).unwrap();
        file.finish().unwrap();
        let mut parsed = File::read(writer.into_inner().as_slice()).unwrap();
        parsed.header.reporter_address = String::new();
        assert_eq!(
            parsed.validate(),
            [Issue::Header {
                violation: Violation::HeaderFieldRequired {
                    field: "reporter_address"
                }
            }]
        );
    }

    #[test]
    fn closed_by_grantor_and_at_consumer_request_conflict() {
        // CRRG 2020 FAQs 39 and 40: only one closed code applies.
        for code in [
            ComplianceConditionCode::ClosedAtRequest,
            ComplianceConditionCode::ClosedAndFcraDispute,
            ComplianceConditionCode::ClosedAndFcraInvestigated,
            ComplianceConditionCode::ClosedAndFcbaDispute,
        ] {
            let both = BaseSegment {
                special_comment: Some(SpecialComment::M),
                compliance_condition_code: Some(code),
                ..base()
            };
            assert_eq!(
                both.validate(),
                Err(vec![Violation::ConflictingClosedCodes {
                    special_comment: SpecialComment::M,
                    compliance_condition_code: code,
                }]),
                "{code:?}"
            );
        }
        // A dispute code is not a closed code.
        let disputed = BaseSegment {
            special_comment: Some(SpecialComment::M),
            compliance_condition_code: Some(ComplianceConditionCode::FcraDispute),
            ..base()
        };
        assert_eq!(disputed.validate(), Ok(()));
    }

    #[test]
    fn collection_and_debt_buyer_accounts_need_k1() {
        // CRRG 2020 K1 Segment (page 4-36) and Exhibit 1: account types 48,
        // 0C and 77 belong to the reporters that must send K1.
        for code in ["48", "0C", "77"] {
            let account_type: AccountType = code.parse().unwrap();
            assert!(account_type.requires_k1(), "{code}");
            let record = DataRecord::new(BaseSegment {
                account_type,
                ..base()
            });
            assert_eq!(
                record.validate(),
                Err(vec![Violation::K1Required { account_type }]),
                "{code}"
            );
            let with_k1 = DataRecord {
                k1: Some(k1()),
                ..record
            };
            assert_eq!(with_k1.validate(), Ok(()), "{code}");
        }
        assert!(!AccountType::Unsecured.requires_k1());
    }
}

mod characters {
    //! Character rules from the CRRG's field descriptions.
    use super::*;
    use metro2::{CharClass, DataRecord, J1Segment, J2Segment, L1Segment};
    use pretty_assertions::assert_eq;

    #[test]
    fn classes() {
        assert!("SMITH-JONES".chars().all(|c| CharClass::Name.allows(c)));
        assert!("VAN DYKE".chars().all(|c| CharClass::Name.allows(c)));
        for c in ['\'', '.', '1', ',', '&'] {
            assert!(!CharClass::Name.allows(c), "{c:?}");
        }
        assert!(
            "123 N. MAIN ST-B/2"
                .chars()
                .all(|c| CharClass::Address.allows(c))
        );
        for c in ['#', ',', '+', '&'] {
            assert!(!CharClass::Address.allows(c), "{c:?}");
        }
        assert!(
            "ACCT000001"
                .chars()
                .all(|c| CharClass::Identifier.allows(c))
        );
        for c in [' ', '-', '/'] {
            assert!(!CharClass::Identifier.allows(c), "{c:?}");
        }
    }

    #[test]
    fn base_segment_fields() {
        let b = BaseSegment {
            surname: "O'BRIEN".into(),
            first_line_of_address: "12 MAIN ST, APT 4".into(),
            consumer_account_number: "ACCT 1".into(),
            ..base()
        };
        let fields: Vec<_> = b
            .validate()
            .unwrap_err()
            .into_iter()
            .map(|v| match v {
                Violation::Characters { field, ch, .. } => (field, ch),
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert_eq!(
            fields,
            [
                ("consumer_account_number".to_owned(), ' '),
                ("surname".to_owned(), '\''),
                ("first_line_of_address".to_owned(), ','),
            ]
        );
    }

    #[test]
    fn message() {
        let err = CharClass::Name.check("surname", "O'BRIEN").unwrap_err();
        assert_eq!(
            err.to_string(),
            "surname contains '\\'' at position 1; only letters, spaces and hyphens are allowed"
        );
    }

    #[test]
    fn segments_are_prefixed() {
        let record = DataRecord {
            j1: vec![J1Segment {
                first_name: "ANN-MARIE".into(),
                ..j1()
            }],
            j2: vec![
                j2(),
                J2Segment {
                    city: "ST. LOUIS #2".into(),
                    ..j2()
                },
            ],
            l1: Some(L1Segment {
                new_identification_number: "ID/7".into(),
                ..l1()
            }),
            ..DataRecord::new(base())
        };
        let fields: Vec<_> = record
            .validate()
            .unwrap_err()
            .into_iter()
            .map(|v| match v {
                Violation::Characters { field, .. } => field,
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert_eq!(fields, ["j2[1].city", "l1.new_identification_number"]);
    }
}
