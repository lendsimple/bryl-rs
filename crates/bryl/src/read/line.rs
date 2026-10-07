use std::io::BufRead;

use super::{Location, Raw, ReadErrorKind, Source};

/// Newline-terminated records.
///
/// Lines end with `\n` or `\r\n`; the last line may have no terminator. The
/// terminator is removed from [`Raw::bytes`] and kept in [`Raw::terminator`].
#[derive(Debug)]
pub struct LineSource<R> {
    reader: R,
    /// Line number of the most recently read line (0 before any reads).
    line: usize,
    expected_terminator: Option<Vec<u8>>,
    skip_blank: bool,
}

impl<R: BufRead> LineSource<R> {
    /// Reads lines from `reader`, accepting `\n` and `\r\n` terminators.
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line: 0,
            expected_terminator: None,
            skip_blank: false,
        }
    }

    /// Requires every terminated line to end with exactly `terminator`, such
    /// as `b"\r\n"`. A final line without a terminator is still accepted.
    #[must_use]
    pub fn expect_terminator(mut self, terminator: impl Into<Vec<u8>>) -> Self {
        self.expected_terminator = Some(terminator.into());
        self
    }

    /// Skips empty lines instead of returning them as records.
    #[must_use]
    pub fn skip_blank(mut self, skip: bool) -> Self {
        self.skip_blank = skip;
        self
    }

    /// Returns the wrapped reader.
    pub fn into_inner(self) -> R {
        self.reader
    }
}

impl<R: BufRead> Source for LineSource<R> {
    fn next_raw(&mut self) -> Result<Option<Raw>, ReadErrorKind> {
        loop {
            let mut bytes = Vec::new();
            if self.reader.read_until(b'\n', &mut bytes)? == 0 {
                return Ok(None);
            }
            self.line += 1;
            let terminator_len = if bytes.ends_with(b"\r\n") {
                2
            } else {
                usize::from(bytes.ends_with(b"\n"))
            };
            let terminator = bytes.split_off(bytes.len() - terminator_len);
            if let Some(expected) = &self.expected_terminator {
                if !terminator.is_empty() && &terminator != expected {
                    return Err(ReadErrorKind::UnexpectedTerminator {
                        expected: String::from_utf8_lossy(expected).into_owned(),
                        found: String::from_utf8_lossy(&terminator).into_owned(),
                    });
                }
            }
            if self.skip_blank && bytes.is_empty() {
                continue;
            }
            return Ok(Some(Raw {
                bytes,
                terminator,
                location: Location::Line(self.line),
            }));
        }
    }

    fn location(&self) -> Location {
        Location::Line(self.line.max(1))
    }
}
