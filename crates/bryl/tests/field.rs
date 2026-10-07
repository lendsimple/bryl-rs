//! Field-level tests.
//!
//! | `test_bryl.py`             | Here |
//! |----------------------------|------|
//! | `TestField`                | `field_spec::*`, `constant::*` |
//! | `TestFieldPackUnpack`      | `pack_unpack::*` |
//! | `TestFieldProbe`           | `probe::*` (byte slices instead of seekable IO, B14) |
//! | `TestNumeric`              | `numeric::*` |
//! | `TestAlphanumeric`         | `alpha::*` (sanitize cases are in `sanitize.rs`) |
//! | `TestDatetime`             | `datetime::*` |
//! | `TestDate`                 | `date::*` |
//! | `TestTime`                 | `time::*` |
//! | `TestFieldEnum`            | Stage 2 (`#[derive(Code)]`) |
//!
//! Adapted: Python's runtime type checks ("must be a string", "must be a
//! number", "must be a datetime") are enforced by the Rust type system, so
//! those tests become kind-mismatch tests.

mod common;

use bryl::{Align, Const, Constant, FieldErrorKind, FieldKind, FieldSpec, Token};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use common::{MMDDYYYY, pack, unpack};

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn time(h: u32, m: u32, s: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, s).unwrap()
}

fn datetime(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
    date(y, mo, d).and_time(time(h, mi, s))
}

const YYYYMMDD: &[Token] = &[Token::Year4, Token::Month, Token::Day];
const YYMMDD: &[Token] = &[Token::Year2, Token::Month, Token::Day];
const YYYYMMDDHHMM: &[Token] = &[
    Token::Year4,
    Token::Month,
    Token::Day,
    Token::Hour24,
    Token::Minute,
];
const HHMM: &[Token] = &[Token::Hour24, Token::Minute];
const HHMMSS: &[Token] = &[Token::Hour24, Token::Minute, Token::Second];

mod field_spec {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn numeric_defaults() {
        let f = FieldSpec::numeric("n", 0, 5);
        assert_eq!(f.length, 5);
        assert_eq!(f.pad, b'0');
        assert_eq!(f.align, Align::Right);
        assert_eq!(
            f.kind,
            FieldKind::Numeric {
                min: None,
                max: None
            }
        );
    }

    #[test]
    fn alpha_defaults() {
        let f = FieldSpec::alpha("a", 0, 10);
        assert_eq!(f.pad, b' ');
        assert_eq!(f.align, Align::Left);
        assert_eq!(f.constant, None);
    }

    #[test]
    fn date_length_is_pattern_width() {
        assert_eq!(FieldSpec::datetime("d", 0, YYYYMMDDHHMM).length, 12);
        assert_eq!(FieldSpec::date("d", 0, YYMMDD).length, 6);
    }

    #[test]
    fn range_covers_offset_and_length() {
        assert_eq!(FieldSpec::alpha("a", 15, 5).range(), 15..20);
    }

    #[test]
    fn bounds_only_apply_to_numeric() {
        let f = FieldSpec::numeric("n", 0, 5).with_min(10).with_max(100);
        assert_eq!(
            f.kind,
            FieldKind::Numeric {
                min: Some(10),
                max: Some(100)
            }
        );
        assert_eq!(
            FieldSpec::alpha("a", 0, 5).with_min(10).kind,
            FieldKind::Alpha
        );
    }
}

mod constant {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn reserved_alpha_is_blank_constant() {
        let f = FieldSpec::alpha("filler", 0, 5).reserved();
        assert_eq!(f.constant, Some(Constant::Str("")));
        assert_eq!(pack(&f, &Const).unwrap(), "     ");
    }

    #[test]
    fn reserved_numeric_is_zero_constant() {
        let f = FieldSpec::numeric("reserved", 0, 1).reserved();
        assert_eq!(f.constant, Some(Constant::Num(0)));
        assert_eq!(pack(&f, &Const).unwrap(), "0");
    }

    #[test]
    fn reserved_has_no_effect_on_dates() {
        // Python raises TypeError("does not have a default"); the derive makes
        // this a compile error (Stage 2).
        let f = FieldSpec::date("d", 0, MMDDYYYY).reserved();
        assert_eq!(f.constant, None);
    }

    #[test]
    fn numeric_constant_is_written() {
        let f = FieldSpec::numeric("rdw", 0, 4).with_constant_num(426);
        assert_eq!(pack(&f, &Const).unwrap(), "0426");
        assert_eq!(f.check(), Ok(()));
    }

    #[test]
    fn constant_too_long_fails_check() {
        let f = FieldSpec::numeric("n", 0, 2).with_constant_num(999);
        assert_eq!(
            f.check(),
            Err(FieldErrorKind::TooLong { max: 2, actual: 3 })
        );
    }

    #[test]
    fn constant_of_wrong_kind_fails_check() {
        let f = FieldSpec::numeric("n", 0, 2).with_constant_str("AB");
        assert!(matches!(
            f.check(),
            Err(FieldErrorKind::KindMismatch { .. })
        ));
    }

    #[test]
    fn constant_matches_on_decode() {
        let f = FieldSpec::alpha("id", 0, 6).with_constant_str("HEADER");
        assert_eq!(unpack::<Const>(&f, "HEADER"), Ok(Const));
    }

    #[test]
    fn alpha_constant_mismatch_on_decode() {
        // Python never checked alphanumeric constants on load.
        let f = FieldSpec::alpha("id", 0, 2).with_constant_str("J1");
        assert_eq!(
            unpack::<Const>(&f, "J2"),
            Err(FieldErrorKind::ConstantMismatch {
                expected: "J1".into(),
                found: "J2".into()
            })
        );
    }

    #[test]
    fn numeric_constant_mismatch_on_decode() {
        let f = FieldSpec::numeric("n", 0, 3).with_constant_num(42);
        assert!(matches!(
            unpack::<Const>(&f, "099"),
            Err(FieldErrorKind::ConstantMismatch { .. })
        ));
    }

    #[test]
    fn reserved_alpha_rejects_content() {
        let f = FieldSpec::alpha("filler", 0, 3).reserved();
        assert!(matches!(
            unpack::<Const>(&f, "ab "),
            Err(FieldErrorKind::ConstantMismatch { .. })
        ));
    }

    #[test]
    fn const_without_constant_errors() {
        let f = FieldSpec::alpha("a", 0, 3);
        assert_eq!(pack(&f, &Const), Err(FieldErrorKind::MissingConstant));
        assert_eq!(
            unpack::<Const>(&f, "   "),
            Err(FieldErrorKind::MissingConstant)
        );
    }
}

mod pack_unpack {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn numeric_pack_right_padded() {
        assert_eq!(
            pack(&FieldSpec::numeric("n", 0, 5), &42u32).unwrap(),
            "00042"
        );
    }

    #[test]
    fn numeric_unpack() {
        assert_eq!(
            unpack::<u32>(&FieldSpec::numeric("n", 0, 5), "00042"),
            Ok(42)
        );
    }

    #[test]
    fn alpha_pack_left_aligned() {
        assert_eq!(
            pack(&FieldSpec::alpha("a", 0, 10), &"hello".to_owned()).unwrap(),
            "hello     "
        );
    }

    #[test]
    fn alpha_unpack() {
        assert_eq!(
            unpack::<String>(&FieldSpec::alpha("a", 0, 10), "hello     ").unwrap(),
            "hello"
        );
    }

    #[test]
    fn numeric_pack_left_align() {
        let f = FieldSpec::numeric("n", 0, 5).with_align(Align::Left);
        assert_eq!(pack(&f, &42u32).unwrap(), "42000");
    }

    #[test]
    fn alpha_right_align() {
        // NACHA receiving_dfi_account_number.
        let f = FieldSpec::alpha("a", 0, 6).with_align(Align::Right);
        assert_eq!(pack(&f, &"AB1".to_owned()).unwrap(), "   AB1");
        assert_eq!(unpack::<String>(&f, "   AB1").unwrap(), "AB1");
    }

    #[test]
    fn numeric_space_padded() {
        // NACHA immediate_destination = Numeric(10, pad=" ").
        let f = FieldSpec::numeric("n", 0, 10).with_pad(b' ');
        assert_eq!(pack(&f, &123_456_789u32).unwrap(), " 123456789");
        assert_eq!(unpack::<u32>(&f, " 123456789"), Ok(123_456_789));
    }

    #[test]
    fn roundtrip_numeric() {
        let f = FieldSpec::numeric("n", 0, 8);
        assert_eq!(unpack::<u32>(&f, &pack(&f, &12345u32).unwrap()), Ok(12345));
    }

    #[test]
    fn roundtrip_alpha() {
        let f = FieldSpec::alpha("a", 0, 15);
        let packed = pack(&f, &"hello world".to_owned()).unwrap();
        assert_eq!(unpack::<String>(&f, &packed).unwrap(), "hello world");
    }
}

mod probe {
    use super::*;
    use crate::common::SampleRecord;
    use bryl::{Record, codec};
    use pretty_assertions::assert_eq;

    #[test]
    fn reads_field_at_offset() {
        let raw = b"123YYYYY";
        let f = FieldSpec::numeric("n", 0, 3);
        assert_eq!(codec::decode_field::<SampleRecord, u32>(&f, raw), Ok(123));
    }

    #[test]
    fn invalid_field_is_error() {
        let f = FieldSpec::numeric("n", 0, 3);
        assert!(codec::decode_field::<SampleRecord, u32>(&f, b"abc").is_err());
    }

    #[test]
    fn short_input_is_length_error() {
        // Python: unpack("123") on Numeric(10) -> "Length must be >= 10".
        let f = FieldSpec::numeric("n", 0, 10);
        assert_eq!(
            codec::decode_field::<SampleRecord, u32>(&f, b"123"),
            Err(bryl::Error::Length {
                record: "SampleRecord",
                expected: SampleRecord::LENGTH,
                actual: 3
            })
        );
    }
}

mod numeric {
    use super::*;
    use pretty_assertions::assert_eq;

    fn n(len: usize) -> FieldSpec {
        FieldSpec::numeric("n", 0, len)
    }

    #[test]
    fn valid_number() {
        assert_eq!(pack(&n(5), &123u32).unwrap(), "00123");
    }

    #[test]
    fn length_exceeded() {
        assert_eq!(
            pack(&n(3), &12345u32),
            Err(FieldErrorKind::TooLong { max: 3, actual: 5 })
        );
    }

    #[test]
    fn min_value() {
        let f = n(5).with_min(10);
        let err = pack(&f, &5u32).unwrap_err();
        assert_eq!(err, FieldErrorKind::BelowMin { min: 10, value: 5 });
        assert!(err.to_string().contains(">="));
    }

    #[test]
    fn max_value() {
        let f = n(5).with_max(100);
        let err = pack(&f, &200u32).unwrap_err();
        assert_eq!(
            err,
            FieldErrorKind::AboveMax {
                max: 100,
                value: 200
            }
        );
        assert!(err.to_string().contains("<="));
    }

    #[test]
    fn bounds_checked_on_decode() {
        assert!(matches!(
            unpack::<u32>(&n(3).with_max(100), "200"),
            Err(FieldErrorKind::AboveMax { .. })
        ));
    }

    #[test]
    fn blank_decodes_to_zero() {
        assert_eq!(unpack::<u32>(&n(5), "00000"), Ok(0));
        assert_eq!(unpack::<u32>(&n(5).with_pad(b' '), "     "), Ok(0));
    }

    #[test]
    fn non_digits_rejected() {
        assert_eq!(
            unpack::<u32>(&n(3), "abc"),
            Err(FieldErrorKind::NotNumeric { raw: "abc".into() })
        );
    }

    #[test]
    fn strict_digits_only() {
        // Python's int() accepted " 12" and "+12".
        assert!(unpack::<u32>(&n(4), "0+12").is_err());
        assert!(unpack::<u32>(&n(4), "0 12").is_err());
        assert!(unpack::<u32>(&n(4), "-012").is_err());
    }

    #[test]
    fn overflow_of_target_type() {
        assert_eq!(
            unpack::<u8>(&n(3), "300"),
            Err(FieldErrorKind::Overflow {
                raw: "300".into(),
                target: "u8"
            })
        );
    }

    #[test]
    fn overflow_of_u64() {
        assert!(matches!(
            unpack::<u64>(&n(21), "999999999999999999999"),
            Err(FieldErrorKind::Overflow { target: "u64", .. })
        ));
    }

    #[test]
    fn integer_in_alpha_field_is_kind_mismatch() {
        assert!(matches!(
            pack(&FieldSpec::alpha("a", 0, 5), &1u32),
            Err(FieldErrorKind::KindMismatch { kind: "alpha", .. })
        ));
    }
}

mod alpha {
    use super::*;
    use pretty_assertions::assert_eq;

    fn a(len: usize) -> FieldSpec {
        FieldSpec::alpha("a", 0, len)
    }

    #[test]
    fn valid_string() {
        assert_eq!(pack(&a(10), &"hello".to_owned()).unwrap(), "hello     ");
    }

    #[test]
    fn length_exceeded() {
        let err = pack(&a(5), &"toolongstring".to_owned()).unwrap_err();
        assert_eq!(err, FieldErrorKind::TooLong { max: 5, actual: 13 });
        assert!(err.to_string().contains("length"));
    }

    #[test]
    fn invalid_char() {
        let err = pack(&a(10), &"hi\0".to_owned()).unwrap_err();
        assert_eq!(err, FieldErrorKind::InvalidChar { ch: '\0', index: 2 });
        assert!(err.to_string().contains("invalid character"));
    }

    #[test]
    fn printable_ascii_only() {
        // Python's string.printable allowed \t \n \r \x0b \x0c.
        for ch in ['\t', '\n', '\r', '\x0b', '\x0c', 'é'] {
            let value = format!("a{ch}");
            assert_eq!(
                pack(&a(5), &value),
                Err(FieldErrorKind::InvalidChar { ch, index: 1 }),
                "{ch:?}"
            );
        }
    }

    #[test]
    fn invalid_char_on_decode() {
        assert_eq!(
            unpack::<String>(&a(3), "a\tb"),
            Err(FieldErrorKind::InvalidChar { ch: '\t', index: 1 })
        );
    }

    #[test]
    fn decode_does_not_sanitize() {
        // Python uppercased on load because load re-ran sanitize.
        assert_eq!(unpack::<String>(&a(5), "smith").unwrap(), "smith");
    }

    #[test]
    fn length_counts_characters() {
        // A multi-byte character is one character that is still invalid.
        assert_eq!(
            pack(&a(2), &"éé".to_owned()),
            Err(FieldErrorKind::InvalidChar { ch: 'é', index: 0 })
        );
    }

    #[test]
    fn string_in_numeric_field_is_kind_mismatch() {
        assert!(matches!(
            pack(&FieldSpec::numeric("n", 0, 5), &"123".to_owned()),
            Err(FieldErrorKind::KindMismatch {
                kind: "numeric",
                ..
            })
        ));
    }
}

mod datetime {
    use super::*;
    use pretty_assertions::assert_eq;

    fn f() -> FieldSpec {
        FieldSpec::datetime("dt", 0, YYYYMMDDHHMM)
    }

    #[test]
    fn decode() {
        assert_eq!(
            unpack::<NaiveDateTime>(&f(), "202301151430"),
            Ok(datetime(2023, 1, 15, 14, 30, 0))
        );
    }

    #[test]
    fn encode() {
        assert_eq!(
            pack(&f(), &datetime(2023, 1, 15, 14, 30, 0)).unwrap(),
            "202301151430"
        );
    }

    #[test]
    fn roundtrip() {
        let dt = datetime(2023, 6, 15, 9, 45, 0);
        let packed = pack(&f(), &dt).unwrap();
        assert_eq!(packed.len(), 12);
        assert_eq!(unpack::<NaiveDateTime>(&f(), &packed), Ok(dt));
    }

    #[test]
    fn metro2_timestamp() {
        // MMDDYYYYhhmmss, see test_metro2.py::test_timestamp_format.
        let pattern: &[Token] = &[
            Token::Month,
            Token::Day,
            Token::Year4,
            Token::Hour24,
            Token::Minute,
            Token::Second,
        ];
        let spec = FieldSpec::datetime("ts", 0, pattern);
        let ts = datetime(2020, 1, 15, 9, 5, 30);
        assert_eq!(pack(&spec, &ts).unwrap(), "01152020090530");
        assert_eq!(unpack::<NaiveDateTime>(&spec, "01152020090530"), Ok(ts));
    }

    #[test]
    fn twelve_hour_clock() {
        let pattern: &[Token] = &[Token::Hour12, Token::Minute, Token::AmPm];
        let spec = FieldSpec::time("t", 0, pattern);
        assert_eq!(pack(&spec, &time(0, 5, 0)).unwrap(), "1205AM");
        assert_eq!(pack(&spec, &time(12, 5, 0)).unwrap(), "1205PM");
        assert_eq!(pack(&spec, &time(15, 30, 0)).unwrap(), "0330PM");
        assert_eq!(unpack::<NaiveTime>(&spec, "1205AM"), Ok(time(0, 5, 0)));
        assert_eq!(unpack::<NaiveTime>(&spec, "1205PM"), Ok(time(12, 5, 0)));
        assert_eq!(unpack::<NaiveTime>(&spec, "0330pm"), Ok(time(15, 30, 0)));
        assert!(unpack::<NaiveTime>(&spec, "1330PM").is_err());
        assert!(unpack::<NaiveTime>(&spec, "0330XM").is_err());
    }

    #[test]
    fn twelve_hour_without_ampm_is_am() {
        // Matches Python's strptime %I without %p.
        let pattern: &[Token] = &[Token::Hour12, Token::Minute];
        let spec = FieldSpec::time("t", 0, pattern);
        assert_eq!(unpack::<NaiveTime>(&spec, "1200"), Ok(time(0, 0, 0)));
    }

    #[test]
    fn literals() {
        let pattern: &[Token] = &[
            Token::Year4,
            Token::Literal(b'-'),
            Token::Month,
            Token::Literal(b'-'),
            Token::Day,
        ];
        let spec = FieldSpec::date("d", 0, pattern);
        assert_eq!(spec.length, 10);
        assert_eq!(pack(&spec, &date(2024, 1, 31)).unwrap(), "2024-01-31");
        assert_eq!(
            unpack::<NaiveDate>(&spec, "2024-01-31"),
            Ok(date(2024, 1, 31))
        );
        assert!(unpack::<NaiveDate>(&spec, "2024/01/31").is_err());
    }

    #[test]
    fn missing_date_components_default_to_1900_01_01() {
        let spec = FieldSpec::datetime("t", 0, HHMM);
        assert_eq!(
            unpack::<NaiveDateTime>(&spec, "1430"),
            Ok(datetime(1900, 1, 1, 14, 30, 0))
        );
    }

    #[test]
    fn wrong_type_for_kind() {
        assert!(matches!(
            pack(&f(), &date(2023, 1, 1)),
            Err(FieldErrorKind::KindMismatch {
                kind: "datetime",
                ..
            })
        ));
        assert!(matches!(
            pack(
                &FieldSpec::alpha("a", 0, 12),
                &datetime(2023, 1, 1, 0, 0, 0)
            ),
            Err(FieldErrorKind::KindMismatch { kind: "alpha", .. })
        ));
    }
}

mod date {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn decode() {
        let f = FieldSpec::date("d", 0, YYYYMMDD);
        assert_eq!(unpack::<NaiveDate>(&f, "20230115"), Ok(date(2023, 1, 15)));
    }

    #[test]
    fn encode() {
        let f = FieldSpec::date("d", 0, YYYYMMDD);
        assert_eq!(pack(&f, &date(2023, 1, 15)).unwrap(), "20230115");
    }

    #[test]
    fn short_year() {
        let f = FieldSpec::date("d", 0, YYMMDD);
        assert_eq!(f.length, 6);
        assert_eq!(pack(&f, &date(2023, 3, 5)).unwrap(), "230305");
    }

    #[test]
    fn short_year_decodes_as_2000s() {
        // Python's strptime pivot gives 69-99 -> 19xx; we always use 20xx.
        let f = FieldSpec::date("d", 0, YYMMDD);
        assert_eq!(unpack::<NaiveDate>(&f, "990101"), Ok(date(2099, 1, 1)));
        assert_eq!(unpack::<NaiveDate>(&f, "000229"), Ok(date(2000, 2, 29)));
    }

    #[test]
    fn short_year_rejects_years_outside_2000s() {
        let f = FieldSpec::date("d", 0, YYMMDD);
        assert!(matches!(
            pack(&f, &date(1999, 12, 31)),
            Err(FieldErrorKind::InvalidDateTime { .. })
        ));
    }

    #[test]
    fn four_digit_year_range() {
        let f = FieldSpec::date("d", 0, YYYYMMDD);
        assert!(pack(&f, &date(10000, 1, 1)).is_err());
        assert!(pack(&f, &date(-1, 1, 1)).is_err());
        assert_eq!(pack(&f, &date(1, 1, 1)).unwrap(), "00010101");
    }

    #[test]
    fn roundtrip() {
        let f = FieldSpec::date("d", 0, YYYYMMDD);
        let d = date(2023, 6, 15);
        assert_eq!(unpack::<NaiveDate>(&f, &pack(&f, &d).unwrap()), Ok(d));
    }

    #[test]
    fn invalid_dates_rejected() {
        let f = FieldSpec::date("d", 0, MMDDYYYY);
        for raw in ["13012024", "02302024", "00000000", "0101202X"] {
            assert!(
                matches!(
                    unpack::<NaiveDate>(&f, raw),
                    Err(FieldErrorKind::InvalidDateTime { .. })
                ),
                "{raw}"
            );
        }
    }

    #[test]
    fn day_of_year() {
        let pattern: &[Token] = &[Token::Year4, Token::DayOfYear];
        let f = FieldSpec::date("d", 0, pattern);
        assert_eq!(f.length, 7);
        assert_eq!(pack(&f, &date(2024, 12, 31)).unwrap(), "2024366");
        assert_eq!(unpack::<NaiveDate>(&f, "2024366"), Ok(date(2024, 12, 31)));
        assert!(unpack::<NaiveDate>(&f, "2023366").is_err());
    }

    #[test]
    fn day_of_year_must_agree_with_month_and_day() {
        let pattern: &[Token] = &[Token::Year4, Token::Month, Token::Day, Token::DayOfYear];
        let f = FieldSpec::date("d", 0, pattern);
        assert_eq!(unpack::<NaiveDate>(&f, "20240201032"), Ok(date(2024, 2, 1)));
        assert!(unpack::<NaiveDate>(&f, "20240201033").is_err());
    }
}

mod time {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn decode() {
        let f = FieldSpec::time("t", 0, HHMM);
        assert_eq!(unpack::<NaiveTime>(&f, "1430"), Ok(time(14, 30, 0)));
    }

    #[test]
    fn encode_with_seconds() {
        let f = FieldSpec::time("t", 0, HHMMSS);
        assert_eq!(pack(&f, &time(14, 30, 45)).unwrap(), "143045");
    }

    #[test]
    fn encode_drops_unrendered_components() {
        // NACHA file_creation_time is hhmm; seconds are not written.
        let f = FieldSpec::time("t", 0, HHMM);
        assert_eq!(pack(&f, &time(14, 30, 45)).unwrap(), "1430");
    }

    #[test]
    fn roundtrip() {
        let f = FieldSpec::time("t", 0, HHMM);
        let t = time(9, 5, 0);
        assert_eq!(unpack::<NaiveTime>(&f, &pack(&f, &t).unwrap()), Ok(t));
    }

    #[test]
    fn invalid_times_rejected() {
        let f = FieldSpec::time("t", 0, HHMM);
        assert!(unpack::<NaiveTime>(&f, "2400").is_err());
        assert!(unpack::<NaiveTime>(&f, "1260").is_err());
    }
}

mod option {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn none_date_is_zero_filled() {
        let f = FieldSpec::date("d", 0, MMDDYYYY);
        assert_eq!(pack(&f, &None::<NaiveDate>).unwrap(), "00000000");
    }

    #[test]
    fn zero_date_decodes_as_none() {
        let f = FieldSpec::date("d", 0, MMDDYYYY);
        assert_eq!(unpack::<Option<NaiveDate>>(&f, "00000000"), Ok(None));
        assert_eq!(unpack::<Option<NaiveDate>>(&f, "        "), Ok(None));
    }

    #[test]
    fn some_date_roundtrips() {
        let f = FieldSpec::date("d", 0, MMDDYYYY);
        let d = Some(date(2020, 3, 15));
        assert_eq!(pack(&f, &d).unwrap(), "03152020");
        assert_eq!(unpack::<Option<NaiveDate>>(&f, "03152020"), Ok(d));
    }

    #[test]
    fn none_timestamp_is_zero_filled() {
        let pattern: &[Token] = &[
            Token::Month,
            Token::Day,
            Token::Year4,
            Token::Hour24,
            Token::Minute,
            Token::Second,
        ];
        let f = FieldSpec::datetime("ts", 0, pattern);
        assert_eq!(pack(&f, &None::<NaiveDateTime>).unwrap(), "0".repeat(14));
        assert_eq!(
            unpack::<Option<NaiveDateTime>>(&f, &"0".repeat(14)),
            Ok(None)
        );
    }

    #[test]
    fn none_alpha_is_blank() {
        let f = FieldSpec::alpha("a", 0, 2);
        assert_eq!(pack(&f, &None::<String>).unwrap(), "  ");
        assert_eq!(unpack::<Option<String>>(&f, "  "), Ok(None));
        // An alphanumeric "00" is a value, not empty.
        assert_eq!(unpack::<Option<String>>(&f, "00"), Ok(Some("00".into())));
    }

    #[test]
    fn none_numeric_uses_pad() {
        let zero = FieldSpec::numeric("n", 0, 3);
        let space = FieldSpec::numeric("n", 0, 3).with_pad(b' ');
        assert_eq!(pack(&zero, &None::<u32>).unwrap(), "000");
        assert_eq!(pack(&space, &None::<u32>).unwrap(), "   ");
        assert_eq!(unpack::<Option<u32>>(&zero, "000"), Ok(None));
        assert_eq!(unpack::<Option<u32>>(&space, "   "), Ok(None));
        assert_eq!(unpack::<Option<u32>>(&zero, "007"), Ok(Some(7)));
    }

    #[test]
    fn invalid_some_is_error() {
        let f = FieldSpec::date("d", 0, MMDDYYYY);
        assert!(unpack::<Option<NaiveDate>>(&f, "13132020").is_err());
    }
}
