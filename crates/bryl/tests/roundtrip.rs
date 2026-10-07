//! Property tests: `decode(encode(x)) == x` for every built-in `FieldValue`.

use bryl::{Const, FieldSpec, FieldValue, Sanitize, Token};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use proptest::prelude::*;

fn roundtrip<T: FieldValue + PartialEq + std::fmt::Debug>(spec: &FieldSpec, value: &T) {
    let mut out = Vec::new();
    value.encode(spec, Sanitize::NONE, &mut out).unwrap();
    assert_eq!(out.len(), spec.length);
    assert_eq!(&T::decode(spec, &out).unwrap(), value);
}

const MMDDYYYY: &[Token] = &[Token::Month, Token::Day, Token::Year4];
const YYMMDD: &[Token] = &[Token::Year2, Token::Month, Token::Day];
const YYYYDDD: &[Token] = &[Token::Year4, Token::DayOfYear];
const HHMMSS: &[Token] = &[Token::Hour24, Token::Minute, Token::Second];
const HHMMPP: &[Token] = &[Token::Hour12, Token::Minute, Token::Second, Token::AmPm];
const TIMESTAMP: &[Token] = &[
    Token::Month,
    Token::Day,
    Token::Year4,
    Token::Hour24,
    Token::Minute,
    Token::Second,
];

fn date_in(years: std::ops::RangeInclusive<i32>) -> impl Strategy<Value = NaiveDate> {
    (years, 1u32..=366).prop_filter_map("valid ordinal", |(y, o)| NaiveDate::from_yo_opt(y, o))
}

fn any_time() -> impl Strategy<Value = NaiveTime> {
    (0u32..24, 0u32..60, 0u32..60).prop_map(|(h, m, s)| NaiveTime::from_hms_opt(h, m, s).unwrap())
}

proptest! {
    // Values without trailing spaces; left-aligned alpha strips them.
    #[test]
    fn alpha_left(value in "[ -~]{0,19}[!-~]|") {
        roundtrip(&FieldSpec::alpha("a", 0, 20), &value);
    }

    // Values without leading spaces; right-aligned alpha strips them.
    #[test]
    fn alpha_right(value in "[!-~][ -~]{0,19}|") {
        roundtrip(&FieldSpec::alpha("a", 0, 20).with_align(bryl::Align::Right), &value);
    }

    #[test]
    fn numeric_u64(value in 0u64..=9_999_999_999) {
        roundtrip(&FieldSpec::numeric("n", 0, 10), &value);
    }

    #[test]
    fn numeric_full_width_u64(value: u64) {
        roundtrip(&FieldSpec::numeric("n", 0, 20), &value);
    }

    #[test]
    fn numeric_u8(value: u8) {
        roundtrip(&FieldSpec::numeric("n", 0, 3), &value);
    }

    #[test]
    fn numeric_space_padded(value in 0u32..=999_999_999) {
        roundtrip(&FieldSpec::numeric("n", 0, 10).with_pad(b' '), &value);
    }

    #[test]
    fn date_four_digit_year(value in date_in(0..=9999)) {
        roundtrip(&FieldSpec::date("d", 0, MMDDYYYY), &value);
    }

    #[test]
    fn date_two_digit_year(value in date_in(2000..=2099)) {
        roundtrip(&FieldSpec::date("d", 0, YYMMDD), &value);
    }

    #[test]
    fn date_day_of_year(value in date_in(0..=9999)) {
        roundtrip(&FieldSpec::date("d", 0, YYYYDDD), &value);
    }

    #[test]
    fn time_24h(value in any_time()) {
        roundtrip(&FieldSpec::time("t", 0, HHMMSS), &value);
    }

    #[test]
    fn time_12h(value in any_time()) {
        roundtrip(&FieldSpec::time("t", 0, HHMMPP), &value);
    }

    #[test]
    fn datetime(date in date_in(1..=9999), time in any_time()) {
        roundtrip(&FieldSpec::datetime("ts", 0, TIMESTAMP), &NaiveDateTime::new(date, time));
    }

    #[test]
    fn optional_date(value in proptest::option::of(date_in(1..=9999))) {
        roundtrip(&FieldSpec::date("d", 0, MMDDYYYY), &value);
    }

    #[test]
    fn optional_alpha(value in proptest::option::of("[!-~][ -~]{0,4}[!-~]|[!-~]")) {
        roundtrip(&FieldSpec::alpha("a", 0, 6), &value);
    }

    #[test]
    fn optional_numeric(value in proptest::option::of(1u32..=99_999)) {
        roundtrip(&FieldSpec::numeric("n", 0, 5), &value);
    }

    #[test]
    fn alpha_constant(value in "[!-~]{1,10}") {
        let spec = FieldSpec::alpha("c", 0, 10).with_constant_str(Box::leak(value.into_boxed_str()));
        roundtrip(&spec, &Const);
    }

    #[test]
    fn numeric_constant(value in 0u64..=99_999) {
        roundtrip(&FieldSpec::numeric("c", 0, 5).with_constant_num(value), &Const);
    }

    // Decoding arbitrary bytes never panics.
    #[test]
    fn decode_never_panics(raw in proptest::collection::vec(any::<u8>(), 8)) {
        let _ = String::decode(&FieldSpec::alpha("a", 0, 8), &raw);
        let _ = u64::decode(&FieldSpec::numeric("n", 0, 8), &raw);
        let _ = NaiveDate::decode(&FieldSpec::date("d", 0, MMDDYYYY), &raw);
        let _ = Option::<NaiveDate>::decode(&FieldSpec::date("d", 0, MMDDYYYY), &raw);
    }
}
