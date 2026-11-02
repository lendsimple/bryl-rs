use bryl::Record;

use crate::records::{
    Addendum, EntryDetail, NocAddendum, RefusedNocAddendum, ReturnAddendum, is_refused_noc,
};

/// An entry detail record with its addenda.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The entry detail record.
    pub detail: EntryDetail,
    /// Its type-05 addenda, in order.
    pub addenda: Vec<Addendum>,
    /// The addendum of a return or notification of change entry, which
    /// carries no type-05 addenda.
    pub special_addendum: Option<SpecialAddendum>,
}

/// The one addendum a return or notification of change entry carries in
/// place of type-05 addenda.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SpecialAddendum {
    /// A return (addenda type 99).
    Return(ReturnAddendum),
    /// A notification of change (addenda type 98).
    Noc(NocAddendum),
    /// A refused notification of change (addenda type 98).
    RefusedNoc(RefusedNocAddendum),
}

impl SpecialAddendum {
    /// Encodes the addendum record.
    ///
    /// # Errors
    ///
    /// Returns an error if the record cannot be encoded.
    pub fn encode(&self) -> Result<String, bryl::Error> {
        match self {
            Self::Return(addendum) => addendum.encode(),
            Self::Noc(addendum) => addendum.encode(),
            Self::RefusedNoc(addendum) => addendum.encode(),
        }
    }

    /// The trace number of the entry carrying the addendum.
    pub const fn trace_number(&self) -> u64 {
        match self {
            Self::Return(addendum) => addendum.trace_number,
            Self::Noc(addendum) => addendum.trace_number,
            Self::RefusedNoc(addendum) => addendum.trace_number,
        }
    }

    /// True for both kinds of notification of change.
    pub const fn is_noc(&self) -> bool {
        matches!(self, Self::Noc(_) | Self::RefusedNoc(_))
    }
}

impl From<ReturnAddendum> for SpecialAddendum {
    fn from(addendum: ReturnAddendum) -> Self {
        Self::Return(addendum)
    }
}

impl From<NocAddendum> for SpecialAddendum {
    fn from(addendum: NocAddendum) -> Self {
        Self::Noc(addendum)
    }
}

impl From<RefusedNocAddendum> for SpecialAddendum {
    fn from(addendum: RefusedNocAddendum) -> Self {
        Self::RefusedNoc(addendum)
    }
}

impl Entry {
    /// True for returns and notifications of change.
    pub const fn is_rejection(&self) -> bool {
        self.detail.transaction_code.is_return()
    }

    /// The return addendum, if the entry is a return.
    pub const fn return_addendum(&self) -> Option<&ReturnAddendum> {
        match &self.special_addendum {
            Some(SpecialAddendum::Return(addendum)) => Some(addendum),
            _ => None,
        }
    }

    /// The notification of change addendum, if the entry is one.
    pub const fn noc_addendum(&self) -> Option<&NocAddendum> {
        match &self.special_addendum {
            Some(SpecialAddendum::Noc(addendum)) => Some(addendum),
            _ => None,
        }
    }

    /// The refused notification of change addendum, if the entry is one.
    pub const fn refused_noc_addendum(&self) -> Option<&RefusedNocAddendum> {
        match &self.special_addendum {
            Some(SpecialAddendum::RefusedNoc(addendum)) => Some(addendum),
            _ => None,
        }
    }

    /// Addenda records of every type.
    pub fn addenda_count(&self) -> usize {
        self.addenda.len() + usize::from(self.special_addendum.is_some())
    }

    /// A copy with the account number masked; see [`EntryDetail::mask`].
    #[must_use]
    pub fn mask(&self) -> Self {
        Self {
            detail: self.detail.mask(),
            addenda: self.addenda.clone(),
            special_addendum: self.special_addendum.clone(),
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
        if let Some(addendum) = &self.special_addendum {
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
            special_addendum: None,
        };
        for line in lines {
            if line.get(1..3) == Some(b"99") {
                entry.special_addendum = Some(ReturnAddendum::decode_exact(line)?.into());
            } else if line.get(1..3) == Some(b"98") && is_refused_noc(line) {
                entry.special_addendum = Some(RefusedNocAddendum::decode_exact(line)?.into());
            } else if line.get(1..3) == Some(b"98") {
                entry.special_addendum = Some(NocAddendum::decode_exact(line)?.into());
            } else {
                entry.addenda.push(Addendum::decode_exact(line)?);
            }
        }
        Ok(entry)
    }
}
