//! A base segment with its appended segments.

use bryl::{EncodeCx, Record};

use crate::error::Error;
use crate::records::{
    BaseSegment, HeaderRecord, J1Segment, J2Segment, K1Segment, K2Segment, K3Segment, K4Segment,
    L1Segment, N1Segment, TrailerRecord,
};

/// Largest value of the 4-digit record descriptor word.
pub const MAX_RECORD_LENGTH: usize = 9999;

/// One account: a base segment and its optional appended segments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataRecord {
    /// The base segment.
    pub base: BaseSegment,
    /// Associated consumers at the same address.
    pub j1: Vec<J1Segment>,
    /// Associated consumers at a different address.
    pub j2: Vec<J2Segment>,
    /// Original creditor.
    pub k1: Option<K1Segment>,
    /// Purchased from / sold to.
    pub k2: Option<K2Segment>,
    /// Mortgage information.
    pub k3: Option<K3Segment>,
    /// Specialized payment information.
    pub k4: Option<K4Segment>,
    /// Account number or identification change.
    pub l1: Option<L1Segment>,
    /// Employment information.
    pub n1: Option<N1Segment>,
}

/// Any Metro 2 record or segment, for flat iteration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Segment {
    Header(HeaderRecord),
    Base(BaseSegment),
    J1(J1Segment),
    J2(J2Segment),
    K1(K1Segment),
    K2(K2Segment),
    K3(K3Segment),
    K4(K4Segment),
    L1(L1Segment),
    N1(N1Segment),
    Trailer(TrailerRecord),
}

impl DataRecord {
    /// A data record with only a base segment.
    pub fn new(base: BaseSegment) -> Self {
        Self {
            base,
            j1: Vec::new(),
            j2: Vec::new(),
            k1: None,
            k2: None,
            k3: None,
            k4: None,
            l1: None,
            n1: None,
        }
    }

    /// Total length: the base segment plus every appended segment. This is
    /// the record descriptor word written for the record.
    pub fn len(&self) -> usize {
        BaseSegment::LENGTH
            + self.j1.len() * J1Segment::LENGTH
            + self.j2.len() * J2Segment::LENGTH
            + self.k1.as_ref().map_or(0, |_| K1Segment::LENGTH)
            + self.k2.as_ref().map_or(0, |_| K2Segment::LENGTH)
            + self.k3.as_ref().map_or(0, |_| K3Segment::LENGTH)
            + self.k4.as_ref().map_or(0, |_| K4Segment::LENGTH)
            + self.l1.as_ref().map_or(0, |_| L1Segment::LENGTH)
            + self.n1.as_ref().map_or(0, |_| N1Segment::LENGTH)
    }

    /// Always false: a data record has at least a base segment.
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// Encodes the record: the base segment (with its record descriptor word
    /// set to [`DataRecord::len`]), then J1s, J2s, K1, K2, K3, K4, L1 and N1.
    /// `self` is not modified.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RecordTooLong`] above 9999 characters, or the first
    /// field that cannot be encoded.
    pub fn encode(&self) -> Result<String, Error> {
        let mut out = Vec::with_capacity(self.len());
        self.encode_into(&EncodeCx::default(), &mut out)?;
        Ok(out.into_iter().map(char::from).collect())
    }

    /// Appends the encoded record to `out`; see [`DataRecord::encode`].
    ///
    /// # Errors
    ///
    /// As for [`DataRecord::encode`].
    pub fn encode_into(&self, cx: &EncodeCx, out: &mut Vec<u8>) -> Result<(), Error> {
        let length = self.len();
        let rdw = u16::try_from(length)
            .ok()
            .filter(|_| length <= MAX_RECORD_LENGTH)
            .ok_or(Error::RecordTooLong { length })?;
        let base = BaseSegment {
            record_descriptor_word: rdw,
            ..self.base.clone()
        };
        base.encode_into(cx, out)?;
        for j1 in &self.j1 {
            j1.encode_into(cx, out)?;
        }
        for j2 in &self.j2 {
            j2.encode_into(cx, out)?;
        }
        encode_opt(self.k1.as_ref(), *cx, out)?;
        encode_opt(self.k2.as_ref(), *cx, out)?;
        encode_opt(self.k3.as_ref(), *cx, out)?;
        encode_opt(self.k4.as_ref(), *cx, out)?;
        encode_opt(self.l1.as_ref(), *cx, out)?;
        encode_opt(self.n1.as_ref(), *cx, out)?;
        Ok(())
    }

    /// Decodes a base segment followed by appended segments, in any order.
    ///
    /// Decoding stops at the first unrecognized segment identifier, which is
    /// treated as padding (fixed-length files pad records to one size).
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid segment, a truncated segment, or a
    /// second K1, K2, K3, K4, L1 or N1.
    pub fn decode(raw: &[u8]) -> Result<Self, Error> {
        let mut record = Self::new(BaseSegment::decode(raw)?);
        let mut rest = &raw[BaseSegment::LENGTH..];
        while rest.len() >= 2 {
            let (segment, length): (&'static str, usize) = match &rest[..2] {
                b"J1" => ("J1", J1Segment::LENGTH),
                b"J2" => ("J2", J2Segment::LENGTH),
                b"K1" => ("K1", K1Segment::LENGTH),
                b"K2" => ("K2", K2Segment::LENGTH),
                b"K3" => ("K3", K3Segment::LENGTH),
                b"K4" => ("K4", K4Segment::LENGTH),
                b"L1" => ("L1", L1Segment::LENGTH),
                b"N1" => ("N1", N1Segment::LENGTH),
                _ => break,
            };
            if rest.len() < length {
                return Err(Error::TruncatedSegment { segment });
            }
            let bytes = &rest[..length];
            match segment {
                "J1" => record.j1.push(J1Segment::decode(bytes)?),
                "J2" => record.j2.push(J2Segment::decode(bytes)?),
                "K1" => set_once(&mut record.k1, K1Segment::decode(bytes)?, segment)?,
                "K2" => set_once(&mut record.k2, K2Segment::decode(bytes)?, segment)?,
                "K3" => set_once(&mut record.k3, K3Segment::decode(bytes)?, segment)?,
                "K4" => set_once(&mut record.k4, K4Segment::decode(bytes)?, segment)?,
                "L1" => set_once(&mut record.l1, L1Segment::decode(bytes)?, segment)?,
                _ => set_once(&mut record.n1, N1Segment::decode(bytes)?, segment)?,
            }
            rest = &rest[length..];
        }
        Ok(record)
    }

    /// The base segment and appended segments, in output order.
    pub fn into_segments(self) -> Vec<Segment> {
        let mut segments = vec![Segment::Base(self.base)];
        segments.extend(self.j1.into_iter().map(Segment::J1));
        segments.extend(self.j2.into_iter().map(Segment::J2));
        segments.extend(self.k1.map(Segment::K1));
        segments.extend(self.k2.map(Segment::K2));
        segments.extend(self.k3.map(Segment::K3));
        segments.extend(self.k4.map(Segment::K4));
        segments.extend(self.l1.map(Segment::L1));
        segments.extend(self.n1.map(Segment::N1));
        segments
    }
}

fn encode_opt<R: Record>(record: Option<&R>, cx: EncodeCx, out: &mut Vec<u8>) -> Result<(), Error> {
    if let Some(record) = record {
        record.encode_into(&cx, out)?;
    }
    Ok(())
}

fn set_once<T>(slot: &mut Option<T>, value: T, segment: &'static str) -> Result<(), Error> {
    if slot.is_some() {
        return Err(Error::DuplicateSegment { segment });
    }
    *slot = Some(value);
    Ok(())
}
