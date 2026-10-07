use std::io::Write as _;

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

use super::{FieldValue, kind_mismatch, show};
use crate::field::{FieldKind, FieldSpec};
use crate::{FieldErrorKind, Sanitize};
use bryl_pattern::Token;

impl FieldValue for NaiveDate {
    fn encode(
        &self,
        spec: &FieldSpec,
        _sanitize: Sanitize,
        out: &mut Vec<u8>,
    ) -> Result<(), FieldErrorKind> {
        let FieldKind::Date(pattern) = spec.kind else {
            return Err(kind_mismatch::<Self>(spec));
        };
        render::<Self>(spec, pattern, Some(*self), None, out)
    }

    fn decode(spec: &FieldSpec, raw: &[u8]) -> Result<Self, FieldErrorKind> {
        let FieldKind::Date(pattern) = spec.kind else {
            return Err(kind_mismatch::<Self>(spec));
        };
        let parsed = Parsed::parse(spec, pattern, raw)?;
        parsed.date().ok_or_else(|| invalid(raw))
    }
}

impl FieldValue for NaiveTime {
    fn encode(
        &self,
        spec: &FieldSpec,
        _sanitize: Sanitize,
        out: &mut Vec<u8>,
    ) -> Result<(), FieldErrorKind> {
        let FieldKind::Time(pattern) = spec.kind else {
            return Err(kind_mismatch::<Self>(spec));
        };
        render::<Self>(spec, pattern, None, Some(*self), out)
    }

    fn decode(spec: &FieldSpec, raw: &[u8]) -> Result<Self, FieldErrorKind> {
        let FieldKind::Time(pattern) = spec.kind else {
            return Err(kind_mismatch::<Self>(spec));
        };
        let parsed = Parsed::parse(spec, pattern, raw)?;
        parsed.time().ok_or_else(|| invalid(raw))
    }
}

impl FieldValue for NaiveDateTime {
    fn encode(
        &self,
        spec: &FieldSpec,
        _sanitize: Sanitize,
        out: &mut Vec<u8>,
    ) -> Result<(), FieldErrorKind> {
        let FieldKind::DateTime(pattern) = spec.kind else {
            return Err(kind_mismatch::<Self>(spec));
        };
        render::<Self>(spec, pattern, Some(self.date()), Some(self.time()), out)
    }

    fn decode(spec: &FieldSpec, raw: &[u8]) -> Result<Self, FieldErrorKind> {
        let FieldKind::DateTime(pattern) = spec.kind else {
            return Err(kind_mismatch::<Self>(spec));
        };
        let parsed = Parsed::parse(spec, pattern, raw)?;
        Ok(NaiveDateTime::new(
            parsed.date().ok_or_else(|| invalid(raw))?,
            parsed.time().ok_or_else(|| invalid(raw))?,
        ))
    }
}

fn invalid(raw: &[u8]) -> FieldErrorKind {
    FieldErrorKind::InvalidDateTime { raw: show(raw) }
}

/// Renders the pattern. A token whose component is missing (e.g. an hour in a
/// date-only value) is a kind mismatch for `T`.
fn render<T>(
    spec: &FieldSpec,
    pattern: &[Token],
    date: Option<NaiveDate>,
    time: Option<NaiveTime>,
    out: &mut Vec<u8>,
) -> Result<(), FieldErrorKind> {
    let need_date = || date.ok_or_else(|| kind_mismatch::<T>(spec));
    let need_time = || time.ok_or_else(|| kind_mismatch::<T>(spec));
    let out_of_range = |value: &dyn std::fmt::Display| FieldErrorKind::InvalidDateTime {
        raw: value.to_string(),
    };
    let start = out.len();
    for token in pattern {
        // Writing to a Vec cannot fail.
        let _ = match *token {
            Token::Year4 => {
                let d = need_date()?;
                if !(0..=9999).contains(&d.year()) {
                    return Err(out_of_range(&d));
                }
                write!(out, "{:04}", d.year())
            }
            Token::Year2 => {
                let d = need_date()?;
                if !(2000..=2099).contains(&d.year()) {
                    return Err(out_of_range(&d));
                }
                write!(out, "{:02}", d.year() - 2000)
            }
            Token::Month => write!(out, "{:02}", need_date()?.month()),
            Token::Day => write!(out, "{:02}", need_date()?.day()),
            Token::DayOfYear => write!(out, "{:03}", need_date()?.ordinal()),
            Token::Hour24 => write!(out, "{:02}", need_time()?.hour()),
            Token::Hour12 => write!(out, "{:02}", need_time()?.hour12().1),
            Token::Minute => write!(out, "{:02}", need_time()?.minute()),
            Token::Second => write!(out, "{:02}", need_time()?.second()),
            Token::AmPm => {
                let pm = need_time()?.hour12().0;
                out.write_all(if pm { b"PM" } else { b"AM" })
            }
            Token::Literal(byte) => out.write_all(&[byte]),
        };
    }
    debug_assert_eq!(out.len() - start, spec.length);
    Ok(())
}

/// Components read from a raw value. Missing components default like Python's
/// `strptime`: 1900-01-01 00:00:00.
#[derive(Default)]
struct Parsed {
    year: Option<i32>,
    month: Option<u32>,
    day: Option<u32>,
    ordinal: Option<u32>,
    hour24: Option<u32>,
    hour12: Option<u32>,
    pm: Option<bool>,
    minute: Option<u32>,
    second: Option<u32>,
}

impl Parsed {
    fn parse(spec: &FieldSpec, pattern: &[Token], raw: &[u8]) -> Result<Self, FieldErrorKind> {
        if raw.len() != spec.length {
            return Err(invalid(raw));
        }
        let mut parsed = Self::default();
        let mut rest = raw;
        for token in pattern {
            let (chunk, tail) = rest.split_at(token.width());
            rest = tail;
            match *token {
                Token::Literal(byte) => {
                    if chunk != [byte] {
                        return Err(invalid(raw));
                    }
                }
                Token::AmPm => {
                    parsed.pm = Some(match chunk.to_ascii_uppercase().as_slice() {
                        b"AM" => false,
                        b"PM" => true,
                        _ => return Err(invalid(raw)),
                    });
                }
                _ => {
                    if !chunk.iter().all(u8::is_ascii_digit) {
                        return Err(invalid(raw));
                    }
                    let value: u32 = show(chunk).parse().map_err(|_| invalid(raw))?;
                    let slot = match token {
                        Token::Year4 | Token::Year2 => {
                            let year = if *token == Token::Year2 {
                                2000 + value
                            } else {
                                value
                            };
                            parsed.year = Some(i32::try_from(year).map_err(|_| invalid(raw))?);
                            continue;
                        }
                        Token::Month => &mut parsed.month,
                        Token::Day => &mut parsed.day,
                        Token::DayOfYear => &mut parsed.ordinal,
                        Token::Hour24 => &mut parsed.hour24,
                        Token::Hour12 => &mut parsed.hour12,
                        Token::Minute => &mut parsed.minute,
                        Token::Second => &mut parsed.second,
                        Token::AmPm | Token::Literal(_) => unreachable!("handled above"),
                    };
                    *slot = Some(value);
                }
            }
        }
        Ok(parsed)
    }

    fn date(&self) -> Option<NaiveDate> {
        let year = self.year.unwrap_or(1900);
        let Some(ordinal) = self.ordinal else {
            return NaiveDate::from_ymd_opt(year, self.month.unwrap_or(1), self.day.unwrap_or(1));
        };
        let date = NaiveDate::from_yo_opt(year, ordinal)?;
        let consistent = self.month.is_none_or(|m| m == date.month())
            && self.day.is_none_or(|d| d == date.day());
        consistent.then_some(date)
    }

    fn time(&self) -> Option<NaiveTime> {
        let hour = match (self.hour24, self.hour12) {
            (Some(hour), None) => hour,
            (None, Some(hour)) => {
                if !(1..=12).contains(&hour) {
                    return None;
                }
                // Without `pp`, treat as AM, as Python's strptime does.
                match (hour, self.pm.unwrap_or(false)) {
                    (12, false) => 0,
                    (12, true) => 12,
                    (hour, false) => hour,
                    (hour, true) => hour + 12,
                }
            }
            (None, None) => 0,
            (Some(_), Some(_)) => return None,
        };
        NaiveTime::from_hms_opt(hour, self.minute.unwrap_or(0), self.second.unwrap_or(0))
    }
}
