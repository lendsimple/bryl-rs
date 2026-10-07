use bryl::Record;

use crate::records::{Addendum, EntryDetail};

/// An entry detail record with its addenda.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The entry detail record.
    pub detail: EntryDetail,
    /// Its addenda, in order.
    pub addenda: Vec<Addendum>,
}

impl Entry {
    /// True for returns and notifications of change.
    pub const fn is_rejection(&self) -> bool {
        self.detail.transaction_code.is_return()
    }

    /// A copy with the account number masked; see [`EntryDetail::mask`].
    #[must_use]
    pub fn mask(&self) -> Self {
        Self {
            detail: self.detail.mask(),
            addenda: self.addenda.clone(),
        }
    }

    /// Encodes the detail and addenda as lines joined by `\n` (no trailing
    /// newline).
    ///
    /// # Errors
    ///
    /// Returns the first record that cannot be encoded.
    pub fn encode(&self) -> Result<String, bryl::Error> {
        let mut lines = vec![self.detail.encode()?];
        for addendum in &self.addenda {
            lines.push(addendum.encode()?);
        }
        Ok(lines.join("\n"))
    }

    /// Decodes lines produced by [`Entry::encode`] (`\r\n` is also accepted).
    ///
    /// # Errors
    ///
    /// Returns the first line that is not a valid record.
    pub fn decode(raw: &[u8]) -> Result<Self, bryl::Error> {
        let mut lines = raw
            .split(|&b| b == b'\n')
            .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
            .filter(|line| !line.is_empty());
        let detail = EntryDetail::decode_exact(lines.next().unwrap_or_default())?;
        let addenda = lines
            .map(Addendum::decode_exact)
            .collect::<Result<_, _>>()?;
        Ok(Self { detail, addenda })
    }
}
