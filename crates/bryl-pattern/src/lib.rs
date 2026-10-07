//! Date/time patterns for `bryl` fields, such as `MMDDYYYY`, `YYMMDD` and `hhmm`.
//!
//! Shared by `bryl` (at runtime) and `bryl-derive` (to reject bad patterns at
//! compile time). Use it through `bryl`, which re-exports everything here.

use thiserror::Error;

/// One element of a date/time pattern such as `MMDDYYYY` or `hhmm`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Token {
    /// `YYYY`: four-digit year.
    Year4,
    /// `YY`: two-digit year, decoded as `20YY`.
    Year2,
    /// `MM`: month, `01`–`12`.
    Month,
    /// `DD`: day of month, `01`–`31`.
    Day,
    /// `DDD` or `JJJ`: day of year, `001`–`366`.
    DayOfYear,
    /// `hh`: hour, `00`–`23`.
    Hour24,
    /// `HH`: hour, `01`–`12`.
    Hour12,
    /// `mm`: minute.
    Minute,
    /// `ss`: second.
    Second,
    /// `pp`: `AM` or `PM`.
    AmPm,
    /// Any other printable, non-letter ASCII byte, copied verbatim.
    Literal(u8),
}

impl Token {
    /// Rendered width in bytes.
    pub const fn width(&self) -> usize {
        match self {
            Self::Year4 => 4,
            Self::DayOfYear => 3,
            Self::Literal(_) => 1,
            Self::Year2
            | Self::Month
            | Self::Day
            | Self::Hour24
            | Self::Hour12
            | Self::Minute
            | Self::Second
            | Self::AmPm => 2,
        }
    }

    /// True for tokens that need a date component.
    pub const fn is_date(&self) -> bool {
        matches!(
            self,
            Self::Year4 | Self::Year2 | Self::Month | Self::Day | Self::DayOfYear
        )
    }

    /// True for tokens that need a time component.
    pub const fn is_time(&self) -> bool {
        matches!(
            self,
            Self::Hour24 | Self::Hour12 | Self::Minute | Self::Second | Self::AmPm
        )
    }
}

/// Writes the token as it is spelled in a pattern, e.g. `YYYY` or `hh`.
impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let spelling = match self {
            Self::Year4 => "YYYY",
            Self::Year2 => "YY",
            Self::Month => "MM",
            Self::Day => "DD",
            Self::DayOfYear => "DDD",
            Self::Hour24 => "hh",
            Self::Hour12 => "HH",
            Self::Minute => "mm",
            Self::Second => "ss",
            Self::AmPm => "pp",
            Self::Literal(byte) => return write!(f, "{}", char::from(*byte)),
        };
        f.write_str(spelling)
    }
}

/// Total rendered width of a pattern.
pub const fn pattern_width(pattern: &[Token]) -> usize {
    let mut total = 0;
    let mut i = 0;
    while i < pattern.len() {
        total += pattern[i].width();
        i += 1;
    }
    total
}

/// Which tokens a pattern may contain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternKind {
    /// Date tokens only.
    Date,
    /// Time tokens only.
    Time,
    /// Any tokens.
    DateTime,
}

/// Writes `date`, `time` or `datetime`.
impl std::fmt::Display for PatternKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Date => "date",
            Self::Time => "time",
            Self::DateTime => "datetime",
        })
    }
}

/// A date/time pattern that could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum PatternError {
    /// Letters that do not form a known token.
    #[error(
        "unknown token at index {index} of pattern {pattern:?}; expected YYYY, YY, MM, DD, DDD, JJJ, hh, HH, mm, ss, pp or punctuation"
    )]
    UnknownToken {
        /// The pattern.
        pattern: String,
        /// Byte index of the unknown token.
        index: usize,
    },
    /// A token that is not allowed for the pattern kind.
    #[error("`{token}` is not allowed in a {kind} pattern")]
    WrongKind {
        /// The offending token.
        token: Token,
        /// The pattern kind.
        kind: PatternKind,
    },
    /// The pattern contains no tokens.
    #[error("pattern is empty")]
    Empty,
}

/// Tokens in match priority order: longer spellings before shorter ones.
const SPELLINGS: &[(&str, Token)] = &[
    ("YYYY", Token::Year4),
    ("YY", Token::Year2),
    ("DDD", Token::DayOfYear),
    ("JJJ", Token::DayOfYear),
    ("DD", Token::Day),
    ("MM", Token::Month),
    ("hh", Token::Hour24),
    ("HH", Token::Hour12),
    ("mm", Token::Minute),
    ("ss", Token::Second),
    ("pp", Token::AmPm),
];

/// Parses a pattern such as `MMDDYYYY`, `YYMMDD` or `MMDDYYYYhhmmss`.
///
/// Non-letter printable ASCII characters become [`Token::Literal`]s. Letters
/// that do not form a known token are rejected, so a typo like `YYY` fails
/// instead of being silently copied.
///
/// # Errors
///
/// Returns a [`PatternError`] for unknown tokens, tokens not allowed by `kind`,
/// or an empty pattern.
pub fn parse_pattern(pattern: &str, kind: PatternKind) -> Result<Vec<Token>, PatternError> {
    let mut tokens = Vec::new();
    let mut rest = pattern;
    while !rest.is_empty() {
        let index = pattern.len() - rest.len();
        let token = if let Some((spelling, token)) = SPELLINGS
            .iter()
            .find(|(spelling, _)| rest.starts_with(spelling))
        {
            rest = &rest[spelling.len()..];
            *token
        } else {
            let byte = rest.as_bytes()[0];
            if byte.is_ascii_alphabetic() || !(b' '..=b'~').contains(&byte) {
                return Err(PatternError::UnknownToken {
                    pattern: pattern.to_owned(),
                    index,
                });
            }
            rest = &rest[1..];
            Token::Literal(byte)
        };
        let allowed = match kind {
            PatternKind::Date => !token.is_time(),
            PatternKind::Time => !token.is_date(),
            PatternKind::DateTime => true,
        };
        if !allowed {
            return Err(PatternError::WrongKind { token, kind });
        }
        tokens.push(token);
    }
    if tokens.is_empty() {
        return Err(PatternError::Empty);
    }
    Ok(tokens)
}
