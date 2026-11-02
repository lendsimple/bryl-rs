use bryl::read::Location;
use bryl::{FieldValue, Record};

use crate::records::{
    Addendum, ContestedReturnAddendum, DishonoredReturnAddendum, EntryDetail, NocAddendum,
    RefusedNocAddendum, ReturnAddendum, ReturnLayout, is_refused_noc, return_layout,
};

/// An entry detail record with its addenda.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum SpecialAddendum {
    /// A return (addenda type 99).
    Return(ReturnAddendum),
    /// A dishonored return (addenda type 99).
    Dishonored(DishonoredReturnAddendum),
    /// A contested dishonored return (addenda type 99).
    Contested(ContestedReturnAddendum),
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
            Self::Dishonored(addendum) => addendum.encode(),
            Self::Contested(addendum) => addendum.encode(),
            Self::Noc(addendum) => addendum.encode(),
            Self::RefusedNoc(addendum) => addendum.encode(),
        }
    }

    /// The trace number of the entry carrying the addendum.
    pub const fn trace_number(&self) -> u64 {
        match self {
            Self::Return(addendum) => addendum.trace_number,
            Self::Dishonored(addendum) => addendum.trace_number,
            Self::Contested(addendum) => addendum.trace_number,
            Self::Noc(addendum) => addendum.trace_number,
            Self::RefusedNoc(addendum) => addendum.trace_number,
        }
    }

    /// True for every kind of return: standard, dishonored and contested.
    pub const fn is_return(&self) -> bool {
        matches!(
            self,
            Self::Return(_) | Self::Dishonored(_) | Self::Contested(_)
        )
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

impl From<DishonoredReturnAddendum> for SpecialAddendum {
    fn from(addendum: DishonoredReturnAddendum) -> Self {
        Self::Dishonored(addendum)
    }
}

impl From<ContestedReturnAddendum> for SpecialAddendum {
    fn from(addendum: ContestedReturnAddendum) -> Self {
        Self::Contested(addendum)
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

    /// The standard return addendum, if the entry is a return; see also
    /// [`Entry::dishonored_return_addendum`] and
    /// [`Entry::contested_return_addendum`].
    pub const fn return_addendum(&self) -> Option<&ReturnAddendum> {
        match &self.special_addendum {
            Some(SpecialAddendum::Return(addendum)) => Some(addendum),
            _ => None,
        }
    }

    /// The dishonored return addendum, if the entry is one.
    pub const fn dishonored_return_addendum(&self) -> Option<&DishonoredReturnAddendum> {
        match &self.special_addendum {
            Some(SpecialAddendum::Dishonored(addendum)) => Some(addendum),
            _ => None,
        }
    }

    /// The contested dishonored return addendum, if the entry is one.
    pub const fn contested_return_addendum(&self) -> Option<&ContestedReturnAddendum> {
        match &self.special_addendum {
            Some(SpecialAddendum::Contested(addendum)) => Some(addendum),
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
                entry.special_addendum = Some(match return_layout(line) {
                    ReturnLayout::Standard => ReturnAddendum::decode_exact(line)?.into(),
                    ReturnLayout::Dishonored => {
                        DishonoredReturnAddendum::decode_exact(line)?.into()
                    }
                    ReturnLayout::Contested => ContestedReturnAddendum::decode_exact(line)?.into(),
                });
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

/// An entry that lenient reading could not decode: its entry detail or one of
/// its addenda failed, for instance on a routing number check digit or an
/// unknown code. See [`File::read_lenient`](crate::File::read_lenient).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct UnreadEntry {
    /// Where its entry detail record starts.
    pub location: Location,
    /// The entry detail record and its addenda, exactly as read.
    pub lines: Vec<Vec<u8>>,
    /// Where the first record that failed to decode starts.
    pub error_location: Location,
    /// Why that record failed to decode.
    pub error: bryl::Error,
}

impl UnreadEntry {
    /// Decodes one field of the entry detail record, such as
    /// `trace_number` or `amount`, so the entry can still be routed.
    ///
    /// # Errors
    ///
    /// Returns the field's decode error if its own content is invalid.
    ///
    /// # Panics
    ///
    /// Panics if [`EntryDetail`] has no field `name`.
    pub fn detail_field<T: FieldValue>(&self, name: &str) -> Result<T, bryl::Error> {
        let spec =
            EntryDetail::field(name).unwrap_or_else(|| panic!("EntryDetail has no field {name:?}"));
        bryl::codec::decode_field::<EntryDetail, T>(spec, &self.lines[0])
    }
}
