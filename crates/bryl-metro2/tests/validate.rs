//! Validation helpers.
//!
//! | `test_metro2.py`              | Here |
//! |-------------------------------|------|
//! | `TestIsValidSSN`/`Phone`/`DOB` | `helpers::*` |
//! | `TestValidatePaymentRating`   | `payment_rating::*` |
//! | `TestValidateAmountPastDue`   | `amount_past_due::*` |
//! | `TestValidatePaymentHistory`  | `payment_history::*` |
//!
//! Plus `BaseSegment::validate` (new; the Python writer never called the
//! helpers, M7).

mod common;

use common::*;
use metro2::{
    AccountStatus, BaseSegment, PaymentHistoryError, PaymentHistoryProfile, PaymentRating,
    Violation, is_valid_dob, is_valid_phone, is_valid_ssn, validate_amount_past_due,
    validate_payment_history, validate_payment_rating,
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
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn valid() {
        assert_eq!(
            validate_payment_rating(AccountStatus::Current, Some(PaymentRating::Current)),
            Ok(())
        );
        assert_eq!(
            validate_payment_rating(AccountStatus::Dpd30, Some(PaymentRating::Past30)),
            Ok(())
        );
        assert_eq!(
            validate_payment_rating(AccountStatus::ChargeOff, Some(PaymentRating::ChargeOff)),
            Ok(())
        );
    }

    #[test]
    fn statuses_without_a_required_rating() {
        for rating in [
            None,
            Some(PaymentRating::Current),
            Some(PaymentRating::ChargeOff),
        ] {
            assert_eq!(
                validate_payment_rating(AccountStatus::Transferred, rating),
                Ok(())
            );
            assert_eq!(
                validate_payment_rating(AccountStatus::Deferred, rating),
                Ok(())
            );
        }
    }

    #[test]
    fn mismatch() {
        let err = validate_payment_rating(AccountStatus::Current, Some(PaymentRating::Past30))
            .unwrap_err();
        assert!(
            err.to_string().contains("requires payment rating '0'"),
            "{err}"
        );
        let err = validate_payment_rating(AccountStatus::Dpd90, Some(PaymentRating::Current))
            .unwrap_err();
        assert!(
            err.to_string().contains("requires payment rating '3'"),
            "{err}"
        );
    }

    #[test]
    fn missing_rating() {
        assert_eq!(
            validate_payment_rating(AccountStatus::Current, None),
            Err(Violation::PaymentRating {
                status: AccountStatus::Current,
                expected: PaymentRating::Current,
                found: " ".into()
            })
        );
    }

    #[test]
    fn required_ratings_match_python_table() {
        let table: Vec<_> = AccountStatus::ALL
            .iter()
            .filter_map(|s| {
                s.required_payment_rating()
                    .map(|r| (s.as_code(), r.as_code()))
            })
            .collect();
        assert_eq!(
            table,
            [
                ("11", "0"),
                ("13", "0"),
                ("71", "1"),
                ("78", "2"),
                ("80", "3"),
                ("82", "4"),
                ("83", "5"),
                ("84", "6"),
                ("93", "G"),
                ("97", "L"),
            ]
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
        assert!(matches!(violations[0], Violation::PaymentRating { .. }));
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
    fn every_status_with_its_required_rating_is_valid() {
        for &status in AccountStatus::ALL {
            assert_eq!(base_with_status(status).validate(), Ok(()), "{status:?}");
        }
    }
}
