use std::borrow::Cow;

/// Normalization applied to alphanumeric values while encoding.
///
/// Set per record ([`crate::Record::SANITIZE`]), per field, or per encode
/// call; there is no global setting. Sanitizing only happens on encode;
/// decoding never alters data. The steps run in the order filter, truncate,
/// upper.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Sanitize {
    /// Uppercase ASCII letters.
    pub upper: bool,
    /// Drop characters outside printable ASCII.
    pub filter: bool,
    /// Truncate values longer than the field.
    pub truncate: bool,
}

impl Sanitize {
    /// No normalization.
    pub const NONE: Self = Self {
        upper: false,
        filter: false,
        truncate: false,
    };

    /// Uppercase only, the default for Metro 2 and NACHA records.
    pub const UPPER: Self = Self {
        upper: true,
        filter: false,
        truncate: false,
    };

    /// Returns a copy with `upper` set.
    #[must_use]
    pub const fn upper(mut self, on: bool) -> Self {
        self.upper = on;
        self
    }

    /// Returns a copy with `filter` set.
    #[must_use]
    pub const fn filter(mut self, on: bool) -> Self {
        self.filter = on;
        self
    }

    /// Returns a copy with `truncate` set.
    #[must_use]
    pub const fn truncate(mut self, on: bool) -> Self {
        self.truncate = on;
        self
    }

    /// Picks the effective settings: field attribute, then encode-time
    /// override, then the record default.
    pub fn resolve(field: Option<Self>, overridden: Option<Self>, record: Self) -> Self {
        field.or(overridden).unwrap_or(record)
    }

    /// Applies these settings to `value` for a field of `length` characters.
    pub fn apply<'a>(&self, value: &'a str, length: usize) -> Cow<'a, str> {
        let mut value = Cow::Borrowed(value);
        if self.filter && !value.chars().all(is_printable) {
            value = Cow::Owned(value.chars().filter(|&c| is_printable(c)).collect());
        }
        if self.truncate {
            if let Some((index, _)) = value.char_indices().nth(length) {
                value = Cow::Owned(value[..index].to_owned());
            }
        }
        if self.upper && value.bytes().any(|b| b.is_ascii_lowercase()) {
            value = Cow::Owned(value.to_ascii_uppercase());
        }
        value
    }
}

/// True for printable ASCII (`0x20..=0x7E`), the only characters allowed in a
/// record.
pub(crate) fn is_printable(c: char) -> bool {
    matches!(c, ' '..='~')
}
