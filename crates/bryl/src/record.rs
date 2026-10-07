use crate::{Error, FieldSpec, Sanitize};

/// Encode-time options that override record defaults.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EncodeCx {
    /// Overrides the record's [`Record::SANITIZE`] (field-level settings still win).
    pub sanitize: Option<Sanitize>,
}

/// A fixed-width record made of fixed-width fields.
///
/// Normally implemented by `#[derive(Record)]`. Hand-written implementations
/// encode and decode each field in order with [`codec::encode_field`] and
/// [`codec::decode_field`].
///
/// [`codec::encode_field`]: crate::codec::encode_field
/// [`codec::decode_field`]: crate::codec::decode_field
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a bryl record",
    label = "only records can be flattened; add `#[derive(bryl::Record)]` to `{Self}`"
)]
pub trait Record: Sized {
    /// Record type name, used in error messages.
    const NAME: &'static str;
    /// Encoded length in bytes.
    const LENGTH: usize;
    /// Field layout, in order.
    const FIELDS: &'static [FieldSpec];
    /// Default sanitize settings for alphanumeric fields.
    const SANITIZE: Sanitize = Sanitize::NONE;

    /// Appends the encoded record (exactly [`Self::LENGTH`] bytes) to `out`.
    ///
    /// # Errors
    ///
    /// Returns the first field that cannot be encoded.
    fn encode_into(&self, cx: &EncodeCx, out: &mut Vec<u8>) -> Result<(), Error>;

    /// Decodes the fields from `raw`, which is at least [`Self::LENGTH`] bytes.
    ///
    /// Callers should use [`Record::decode`], which checks the length first.
    ///
    /// # Errors
    ///
    /// Returns the first field that cannot be decoded.
    fn decode_fields(raw: &[u8]) -> Result<Self, Error>;

    /// Decodes a record from the first [`Self::LENGTH`] bytes of `raw`.
    /// Trailing bytes (such as a line terminator) are ignored.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Length`] if `raw` is too short, or the first invalid field.
    fn decode(raw: &[u8]) -> Result<Self, Error> {
        if raw.len() < Self::LENGTH {
            return Err(Error::Length {
                record: Self::NAME,
                expected: Self::LENGTH,
                actual: raw.len(),
            });
        }
        Self::decode_fields(&raw[..Self::LENGTH])
    }

    /// Decodes a record from `raw`, which must be exactly [`Self::LENGTH`] bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Length`] on a length mismatch, or the first invalid field.
    fn decode_exact(raw: &[u8]) -> Result<Self, Error> {
        if raw.len() != Self::LENGTH {
            return Err(Error::Length {
                record: Self::NAME,
                expected: Self::LENGTH,
                actual: raw.len(),
            });
        }
        Self::decode_fields(raw)
    }

    /// Looks up a field's layout by name.
    fn field(name: &str) -> Option<&'static FieldSpec> {
        Self::FIELDS.iter().find(|spec| spec.name == name)
    }

    /// Decodes a record if `raw` holds a valid one.
    fn probe(raw: &[u8]) -> Option<Self> {
        Self::decode(raw).ok()
    }

    /// Encodes the record with its default sanitize settings.
    ///
    /// # Errors
    ///
    /// Returns the first field that cannot be encoded.
    fn encode(&self) -> Result<String, Error> {
        self.encode_cx(&EncodeCx::default())
    }

    /// Encodes the record, overriding its default sanitize settings.
    ///
    /// # Errors
    ///
    /// Returns the first field that cannot be encoded.
    fn encode_with(&self, sanitize: Sanitize) -> Result<String, Error> {
        self.encode_cx(&EncodeCx {
            sanitize: Some(sanitize),
        })
    }

    /// Encodes the record with explicit options.
    ///
    /// # Errors
    ///
    /// Returns the first field that cannot be encoded.
    fn encode_cx(&self, cx: &EncodeCx) -> Result<String, Error> {
        let mut out = Vec::with_capacity(Self::LENGTH);
        self.encode_into(cx, &mut out)?;
        debug_assert_eq!(out.len(), Self::LENGTH, "{} encoded length", Self::NAME);
        Ok(out.into_iter().map(char::from).collect())
    }
}
