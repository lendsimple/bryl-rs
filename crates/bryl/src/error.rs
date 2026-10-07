use thiserror::Error;

/// Why a single field value could not be encoded or decoded.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum FieldErrorKind {
    /// The value does not fit in the field.
    #[error("must have length <= {max}, got {actual}")]
    TooLong {
        /// Field length.
        max: usize,
        /// Length of the offending value.
        actual: usize,
    },
    /// The value contains a character outside printable ASCII (`0x20..=0x7E`).
    #[error("has invalid character {ch:?} @ {index}")]
    InvalidChar {
        /// The offending character.
        ch: char,
        /// Character index within the value.
        index: usize,
    },
    /// A numeric field contains something other than ASCII digits.
    #[error("must be a number, got {raw:?}")]
    NotNumeric {
        /// The raw field content after stripping padding.
        raw: String,
    },
    /// A numeric value is below the field's minimum.
    #[error("must be >= {min}, got {value}")]
    BelowMin {
        /// Minimum allowed value.
        min: u64,
        /// The offending value.
        value: u64,
    },
    /// A numeric value is above the field's maximum.
    #[error("must be <= {max}, got {value}")]
    AboveMax {
        /// Maximum allowed value.
        max: u64,
        /// The offending value.
        value: u64,
    },
    /// A decoded number does not fit the Rust integer type.
    #[error("number {raw} does not fit in {target}")]
    Overflow {
        /// The decoded digits.
        raw: String,
        /// The target Rust type.
        target: &'static str,
    },
    /// A code value is not part of its code table.
    #[error("unknown code {0:?}")]
    UnknownCode(String),
    /// A constant (or reserved) field holds an unexpected value.
    #[error("must be constant {expected:?}, got {found:?}")]
    ConstantMismatch {
        /// The constant value.
        expected: String,
        /// The value found.
        found: String,
    },
    /// A date, time or datetime could not be rendered or parsed.
    #[error("invalid date/time {raw:?}")]
    InvalidDateTime {
        /// The offending raw content or value.
        raw: String,
    },
    /// The Rust type cannot be stored in a field of this kind.
    #[error("{type_name} cannot be stored in a {kind} field")]
    KindMismatch {
        /// The Rust type being encoded or decoded.
        type_name: &'static str,
        /// The field kind.
        kind: &'static str,
    },
    /// A format-specific rule rejected the value (e.g. a bad check digit).
    #[error("{0}")]
    Invalid(String),
    /// A `Const` field has no constant in its spec.
    #[error("field has no constant value")]
    MissingConstant,
}

/// An error encoding or decoding a record.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum Error {
    /// A field could not be encoded or decoded.
    #[error("{record}.{field} @ {offset} - {kind}")]
    Field {
        /// Record type name.
        record: &'static str,
        /// Field name.
        field: &'static str,
        /// Field byte offset within the record.
        offset: usize,
        /// What went wrong.
        kind: FieldErrorKind,
    },
    /// The raw input is too short (or, for exact decoding, not the right length).
    #[error("{record} length must be {expected}, got {actual}")]
    Length {
        /// Record type name.
        record: &'static str,
        /// Required length.
        expected: usize,
        /// Actual input length.
        actual: usize,
    },
}

impl Error {
    /// The field-level cause, if this is a field error.
    pub fn field_kind(&self) -> Option<&FieldErrorKind> {
        match self {
            Self::Field { kind, .. } => Some(kind),
            Self::Length { .. } => None,
        }
    }
}
