//! Validation helpers.
//!
//! | `test_metro2.py`              | Here |
//! |-------------------------------|------|
//! | `TestIsValidSSN`/`Phone`/`DOB` | `helpers::*` |
//! | `TestValidatePaymentRating`   | `payment_rating::*`, rewritten for the CRRG rule (M8) |
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
    //! Python required e.g. rating `0` for status 11 (M8). The CRRG rule, as
    //! implemented by moov-io/metro2: a rating is required for 05, 13, 65, 88,
    //! 89, 94 and 95, and must be blank otherwise.
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
        // Python required these ratings; the CRRG rule rejects them.
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
    fn every_status_with_its_required_rating_is_valid() {
        for &status in AccountStatus::ALL {
            assert_eq!(base_with_status(status).validate(), Ok(()), "{status:?}");
        }
    }
}
