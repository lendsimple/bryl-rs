//! `DataRecord`.
//!
//! | `test_metro2.py`  | Here |
//! |-------------------|------|
//! | `TestDataRecord`  | all tests in this file |

mod common;

use bryl::Record;
use common::*;
use metro2::{BaseSegment, DataRecord, Error, Segment};
use pretty_assertions::assert_eq;

#[test]
fn base_only_construction() {
    let record = DataRecord::new(base());
    assert_eq!(record.base, base());
    assert!(record.j1.is_empty() && record.j2.is_empty());
    assert_eq!(record.k1, None);
    assert_eq!(record.len(), 426);
}

#[test]
fn encode_base_only() {
    let raw = DataRecord::new(base()).encode().unwrap();
    assert_eq!(raw.len(), 426);
    assert_eq!(&raw[..4], "0426");
}

#[test]
fn encode_with_j1_updates_rdw() {
    let record = DataRecord {
        j1: vec![j1()],
        ..DataRecord::new(base())
    };
    let raw = record.encode().unwrap();
    assert_eq!(raw.len(), 526);
    assert_eq!(&raw[..4], "0526");
}

#[test]
fn encode_with_j2_updates_rdw() {
    let record = DataRecord {
        j2: vec![j2()],
        ..DataRecord::new(base())
    };
    assert_eq!(&record.encode().unwrap()[..4], "0626");
}

#[test]
fn encode_with_all_segments() {
    let raw = all_segments().encode().unwrap();
    assert_eq!(raw.len(), ALL_SEGMENTS_LENGTH);
    assert_eq!(raw[..4], format!("{ALL_SEGMENTS_LENGTH:04}"));
}

#[test]
fn encode_with_multiple_j1() {
    let record = DataRecord {
        j1: vec![
            j1(),
            metro2::J1Segment {
                surname: "DOE".into(),
                ..j1()
            },
        ],
        ..DataRecord::new(base())
    };
    let raw = record.encode().unwrap();
    assert_eq!(raw.len(), 626);
    assert_eq!(&raw[..4], "0626");
}

#[test]
fn segment_order() {
    let raw = all_segments().encode().unwrap();
    let mut offset = 426;
    for (id, length) in [
        ("J1", 100),
        ("J2", 200),
        ("K1", 34),
        ("K2", 34),
        ("K3", 40),
        ("K4", 30),
        ("L1", 54),
        ("N1", 146),
    ] {
        assert_eq!(&raw[offset..offset + 2], id);
        offset += length;
    }
}

#[test]
fn decode_base_only() {
    let record = DataRecord::decode(DataRecord::new(base()).encode().unwrap().as_bytes()).unwrap();
    assert_eq!(record.base.surname, "SMITH");
    assert!(record.j1.is_empty());
    assert_eq!(record.k1, None);
}

#[test]
fn decode_with_j1() {
    let record = DataRecord {
        j1: vec![metro2::J1Segment {
            social_security_number: 987_654_321,
            ..j1()
        }],
        ..DataRecord::new(base())
    };
    let decoded = DataRecord::decode(record.encode().unwrap().as_bytes()).unwrap();
    assert_eq!(decoded.j1.len(), 1);
    assert_eq!(decoded.j1[0].social_security_number, 987_654_321);
}

#[test]
fn roundtrip_all_segments() {
    let record = all_segments();
    let decoded = DataRecord::decode(record.encode().unwrap().as_bytes()).unwrap();
    // The decoded base carries the record descriptor word that was written.
    assert_eq!(
        decoded.base.record_descriptor_word as usize,
        ALL_SEGMENTS_LENGTH
    );
    assert_eq!(
        decoded,
        DataRecord {
            base: BaseSegment {
                record_descriptor_word: 1064,
                ..record.base.clone()
            },
            ..record
        }
    );
}

#[test]
fn roundtrip_preserves_base_fields() {
    let record = DataRecord {
        base: BaseSegment {
            credit_limit: 50_000,
            current_balance: 12_345,
            social_security_number: 123_456_789,
            ..base()
        },
        j1: vec![j1()],
        ..DataRecord::new(base())
    };
    let decoded = DataRecord::decode(record.encode().unwrap().as_bytes()).unwrap();
    assert_eq!(decoded.base.credit_limit, 50_000);
    assert_eq!(decoded.base.current_balance, 12_345);
    assert_eq!(decoded.base.social_security_number, 123_456_789);
}

#[test]
fn encode_does_not_mutate_base() {
    let record = DataRecord {
        j1: vec![j1()],
        ..DataRecord::new(base())
    };
    record.encode().unwrap();
    assert_eq!(record.base.record_descriptor_word, 426);
}

#[test]
fn unknown_segment_id_stops_decoding() {
    // Fixed-length files pad records; padding is not a segment.
    let raw = format!(
        "{}ZZ{}",
        DataRecord::new(base()).encode().unwrap(),
        " ".repeat(32)
    );
    let record = DataRecord::decode(raw.as_bytes()).unwrap();
    assert!(record.j1.is_empty());
}

#[test]
fn truncated_segment() {
    let raw = format!(
        "{}J1{}",
        DataRecord::new(base()).encode().unwrap(),
        " ".repeat(30)
    );
    let err = DataRecord::decode(raw.as_bytes()).unwrap_err();
    assert!(matches!(err, Error::TruncatedSegment { segment: "J1" }));
    assert_eq!(err.to_string(), "truncated J1 segment");
}

#[test]
fn duplicate_segment() {
    let raw = format!(
        "{}{}{}",
        DataRecord::new(base()).encode().unwrap(),
        k1().encode().unwrap(),
        k1().encode().unwrap()
    );
    assert!(matches!(
        DataRecord::decode(raw.as_bytes()),
        Err(Error::DuplicateSegment { segment: "K1" })
    ));
}

#[test]
fn segments_in_any_order_are_read() {
    let raw = format!(
        "{}{}{}",
        DataRecord::new(base()).encode().unwrap(),
        n1().encode().unwrap(),
        j1().encode().unwrap()
    );
    let record = DataRecord::decode(raw.as_bytes()).unwrap();
    assert_eq!(record.j1.len(), 1);
    assert!(record.n1.is_some());
}

#[test]
fn record_too_long() {
    let record = DataRecord {
        j2: vec![j2(); 48],
        ..DataRecord::new(base())
    };
    assert_eq!(record.len(), 10_026);
    assert!(matches!(
        record.encode(),
        Err(Error::RecordTooLong { length: 10_026 })
    ));
}

#[test]
fn into_segments() {
    let kinds: Vec<_> = all_segments()
        .into_segments()
        .into_iter()
        .map(|segment| match segment {
            Segment::Base(_) => "base",
            Segment::J1(_) => "J1",
            Segment::J2(_) => "J2",
            Segment::K1(_) => "K1",
            Segment::K2(_) => "K2",
            Segment::K3(_) => "K3",
            Segment::K4(_) => "K4",
            Segment::L1(_) => "L1",
            Segment::N1(_) => "N1",
            Segment::Header(_) | Segment::Trailer(_) => "other",
        })
        .collect();
    assert_eq!(
        kinds,
        ["base", "J1", "J2", "K1", "K2", "K3", "K4", "L1", "N1"]
    );
}
