//! Date/time pattern parsing.

use bryl_pattern::{PatternError, PatternKind, Token, parse_pattern, pattern_width};
use pretty_assertions::assert_eq;

#[test]
fn metro2_date() {
    assert_eq!(
        parse_pattern("MMDDYYYY", PatternKind::Date),
        Ok(vec![Token::Month, Token::Day, Token::Year4])
    );
}

#[test]
fn nacha_date_and_time() {
    assert_eq!(
        parse_pattern("YYMMDD", PatternKind::Date),
        Ok(vec![Token::Year2, Token::Month, Token::Day])
    );
    assert_eq!(
        parse_pattern("hhmm", PatternKind::Time),
        Ok(vec![Token::Hour24, Token::Minute])
    );
}

#[test]
fn metro2_timestamp() {
    let tokens = parse_pattern("MMDDYYYYhhmmss", PatternKind::DateTime).unwrap();
    assert_eq!(pattern_width(&tokens), 14);
}

#[test]
fn day_of_year_spellings() {
    assert_eq!(
        parse_pattern("YYYYDDD", PatternKind::Date),
        Ok(vec![Token::Year4, Token::DayOfYear])
    );
    assert_eq!(
        parse_pattern("YYJJJ", PatternKind::Date),
        Ok(vec![Token::Year2, Token::DayOfYear])
    );
}

#[test]
fn twelve_hour_tokens() {
    assert_eq!(
        parse_pattern("HHmmpp", PatternKind::Time),
        Ok(vec![Token::Hour12, Token::Minute, Token::AmPm])
    );
}

#[test]
fn literals() {
    assert_eq!(
        parse_pattern("YYYY-MM-DD hh:mm", PatternKind::DateTime),
        Ok(vec![
            Token::Year4,
            Token::Literal(b'-'),
            Token::Month,
            Token::Literal(b'-'),
            Token::Day,
            Token::Literal(b' '),
            Token::Hour24,
            Token::Literal(b':'),
            Token::Minute,
        ])
    );
}

#[test]
fn widths() {
    assert_eq!(
        pattern_width(&parse_pattern("MMDDYYYY", PatternKind::Date).unwrap()),
        8
    );
    assert_eq!(
        pattern_width(&parse_pattern("YYYYDDD", PatternKind::Date).unwrap()),
        7
    );
    assert_eq!(pattern_width(&[]), 0);
}

#[test]
fn unknown_letters_rejected() {
    assert_eq!(
        parse_pattern("YYY", PatternKind::Date),
        Err(PatternError::UnknownToken {
            pattern: "YYY".into(),
            index: 2
        })
    );
    assert!(parse_pattern("XX", PatternKind::DateTime).is_err());
    assert!(parse_pattern("ZZZ", PatternKind::DateTime).is_err());
}

#[test]
fn non_printable_rejected() {
    assert!(parse_pattern("YYYY\tMM", PatternKind::Date).is_err());
    assert!(parse_pattern("YYYYé", PatternKind::Date).is_err());
}

#[test]
fn wrong_kind_rejected() {
    assert_eq!(
        parse_pattern("YYYYhh", PatternKind::Date),
        Err(PatternError::WrongKind {
            token: Token::Hour24,
            kind: PatternKind::Date
        })
    );
    assert!(parse_pattern("hhMM", PatternKind::Time).is_err());
}

#[test]
fn empty_rejected() {
    assert_eq!(
        parse_pattern("", PatternKind::Date),
        Err(PatternError::Empty)
    );
}
