use crate::sanitize::Sanitize;
use bryl_pattern::{Token, pattern_width};

/// Which side of a field a value is aligned to; padding fills the other side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Align {
    /// Value first, padding after (alphanumeric default).
    Left,
    /// Padding first, value after (numeric default).
    Right,
}

/// What a field holds, and therefore how values are rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldKind {
    /// Printable ASCII text.
    Alpha,
    /// Unsigned decimal digits, with optional bounds.
    Numeric {
        /// Smallest allowed value.
        min: Option<u64>,
        /// Largest allowed value.
        max: Option<u64>,
    },
    /// A calendar date rendered with a date-only pattern.
    Date(&'static [Token]),
    /// A time of day rendered with a time-only pattern.
    Time(&'static [Token]),
    /// A date and time rendered with any pattern.
    DateTime(&'static [Token]),
}

impl FieldKind {
    /// Short lowercase name, used in error messages.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Alpha => "alpha",
            Self::Numeric { .. } => "numeric",
            Self::Date(_) => "date",
            Self::Time(_) => "time",
            Self::DateTime(_) => "datetime",
        }
    }
}

/// The fixed value of a constant or reserved field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Constant {
    /// Text constant for an alphanumeric field (`""` means blank-filled).
    Str(&'static str),
    /// Number constant for a numeric field (`0` means zero-filled).
    Num(u64),
}

/// Placeholder type for constant and reserved fields; it stores nothing.
///
/// The value written and expected on decode comes from the field's
/// [`FieldSpec::constant`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Const;

/// Layout and formatting of one field within a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldSpec {
    /// Field name.
    pub name: &'static str,
    /// Byte offset within the record.
    pub offset: usize,
    /// Width in bytes.
    pub length: usize,
    /// Padding byte.
    pub pad: u8,
    /// Alignment of the value within the field.
    pub align: Align,
    /// Kind of value held.
    pub kind: FieldKind,
    /// Fixed value, for constant and reserved fields.
    pub constant: Option<Constant>,
    /// Field-level sanitize override.
    pub sanitize: Option<Sanitize>,
}

impl FieldSpec {
    const fn new(name: &'static str, offset: usize, length: usize, kind: FieldKind) -> Self {
        Self {
            name,
            offset,
            length,
            pad: b' ',
            align: Align::Left,
            kind,
            constant: None,
            sanitize: None,
        }
    }

    /// Alphanumeric field: space padded, left aligned.
    pub const fn alpha(name: &'static str, offset: usize, length: usize) -> Self {
        Self::new(name, offset, length, FieldKind::Alpha)
    }

    /// Numeric field: zero padded, right aligned, no bounds.
    pub const fn numeric(name: &'static str, offset: usize, length: usize) -> Self {
        let mut spec = Self::new(
            name,
            offset,
            length,
            FieldKind::Numeric {
                min: None,
                max: None,
            },
        );
        spec.pad = b'0';
        spec.align = Align::Right;
        spec
    }

    /// Date field whose length is the pattern length.
    pub const fn date(name: &'static str, offset: usize, pattern: &'static [Token]) -> Self {
        Self::temporal(name, offset, FieldKind::Date(pattern), pattern)
    }

    /// Time field whose length is the pattern length.
    pub const fn time(name: &'static str, offset: usize, pattern: &'static [Token]) -> Self {
        Self::temporal(name, offset, FieldKind::Time(pattern), pattern)
    }

    /// Datetime field whose length is the pattern length.
    pub const fn datetime(name: &'static str, offset: usize, pattern: &'static [Token]) -> Self {
        Self::temporal(name, offset, FieldKind::DateTime(pattern), pattern)
    }

    const fn temporal(
        name: &'static str,
        offset: usize,
        kind: FieldKind,
        pattern: &'static [Token],
    ) -> Self {
        let mut spec = Self::new(name, offset, pattern_width(pattern), kind);
        spec.pad = b'0';
        spec.align = Align::Right;
        spec
    }

    /// Overrides the padding byte.
    #[must_use]
    pub const fn with_pad(mut self, pad: u8) -> Self {
        self.pad = pad;
        self
    }

    /// Overrides the alignment.
    #[must_use]
    pub const fn with_align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Sets the minimum of a numeric field (no effect on other kinds).
    #[must_use]
    pub const fn with_min(mut self, value: u64) -> Self {
        if let FieldKind::Numeric { max, .. } = self.kind {
            self.kind = FieldKind::Numeric {
                min: Some(value),
                max,
            };
        }
        self
    }

    /// Sets the maximum of a numeric field (no effect on other kinds).
    #[must_use]
    pub const fn with_max(mut self, value: u64) -> Self {
        if let FieldKind::Numeric { min, .. } = self.kind {
            self.kind = FieldKind::Numeric {
                min,
                max: Some(value),
            };
        }
        self
    }

    /// Makes this an alphanumeric constant field.
    #[must_use]
    pub const fn with_constant_str(mut self, value: &'static str) -> Self {
        self.constant = Some(Constant::Str(value));
        self
    }

    /// Makes this a numeric constant field.
    #[must_use]
    pub const fn with_constant_num(mut self, value: u64) -> Self {
        self.constant = Some(Constant::Num(value));
        self
    }

    /// Makes this a reserved filler: blanks for alphanumeric fields, zeros for
    /// numeric fields. Has no effect on date and time fields, which have no
    /// filler value.
    #[must_use]
    pub const fn reserved(mut self) -> Self {
        self.constant = match self.kind {
            FieldKind::Alpha => Some(Constant::Str("")),
            FieldKind::Numeric { .. } => Some(Constant::Num(0)),
            FieldKind::Date(_) | FieldKind::Time(_) | FieldKind::DateTime(_) => self.constant,
        };
        self
    }

    /// Sets a field-level sanitize override.
    #[must_use]
    pub const fn with_sanitize(mut self, sanitize: Sanitize) -> Self {
        self.sanitize = Some(sanitize);
        self
    }

    /// Returns a copy moved `by` bytes later, for records embedded in other
    /// records.
    #[must_use]
    pub const fn shifted(mut self, by: usize) -> Self {
        self.offset += by;
        self
    }

    /// Byte range of this field within a record.
    pub const fn range(&self) -> std::ops::Range<usize> {
        self.offset..self.offset + self.length
    }

    /// Checks that the spec is internally consistent: a constant matches the
    /// kind and fits the field.
    ///
    /// # Errors
    ///
    /// Returns the problem with the constant, if any.
    pub fn check(&self) -> Result<(), crate::FieldErrorKind> {
        let Some(constant) = self.constant else {
            return Ok(());
        };
        let mut out = Vec::with_capacity(self.length);
        crate::codec::encode_constant(self, constant, &mut out)
    }
}
