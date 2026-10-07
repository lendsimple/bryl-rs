use bryl::Record;

use crate::records::{Addendum, EntryDetail, ReturnAddendum};

/// An entry detail record with its addenda.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The entry detail record.
    pub detail: EntryDetail,
    /// Its type-05 addenda, in order.
    pub addenda: Vec<Addendum>,
    /// The type-99 addendum of a return entry, which carries no type-05
    /// addenda.
    pub return_addendum: Option<ReturnAddendum>,
}

impl Entry {
    /// True for returns and notifications of change.
    pub const fn is_rejection(&self) -> bool {
        self.detail.transaction_code.is_return()
    }

    /// Addenda records of both types.
    pub fn addenda_count(&self) -> usize {
        self.addenda.len() + usize::from(self.return_addendum.is_some())
    }

    /// A copy with the account number masked; see [`EntryDetail::mask`].
    #[must_use]
    pub fn mask(&self) -> Self {
        Self {
            detail: self.detail.mask(),
            addenda: self.addenda.clone(),
            return_addendum: self.return_addendum.clone(),
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
        if let Some(addendum) = &self.return_addendum {
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
        let mut entry = Self {
            detail,
            addenda: Vec::new(),
            return_addendum: None,
        };
        for line in lines {
            if line.get(1..3) == Some(b"99") {
                entry.return_addendum = Some(ReturnAddendum::decode_exact(line)?);
            } else {
                entry.addenda.push(Addendum::decode_exact(line)?);
            }
        }
        Ok(entry)
    }
}
