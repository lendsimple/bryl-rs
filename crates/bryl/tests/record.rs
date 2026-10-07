//! Record-level tests: encoding, decoding, layout and errors (`sample`),
//! probing (`probe`), and constants, sanitizing and optional dates
//! (`tagged`).

mod common;

use bryl::{Error, FieldErrorKind, Record, Sanitize};
use chrono::NaiveDate;
use common::{SampleRecord, TaggedRecord};

mod sample {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn encode_produces_fixed_width() {
        let encoded = SampleRecord::new("hello", 42).encode().unwrap();
        assert_eq!(encoded, "hello     00042     ");
        assert_eq!(encoded.len(), SampleRecord::LENGTH);
    }

    #[test]
    fn roundtrip() {
        let rec = SampleRecord::new("hello", 42);
        let loaded = SampleRecord::decode(rec.encode().unwrap().as_bytes()).unwrap();
        assert_eq!(loaded, rec);
    }

    #[test]
    fn field_names_in_order() {
        let names: Vec<_> = SampleRecord::FIELDS.iter().map(|f| f.name).collect();
        assert_eq!(names, ["alpha", "num", "filler"]);
    }

    #[test]
    fn offsets() {
        let offsets: Vec<_> = SampleRecord::FIELDS.iter().map(|f| f.offset).collect();
        assert_eq!(offsets, [0, 10, 15]);
    }

    #[test]
    fn length_is_sum_of_fields() {
        assert_eq!(SampleRecord::LENGTH, 20);
        assert_eq!(
            SampleRecord::FIELDS.iter().map(|f| f.length).sum::<usize>(),
            SampleRecord::LENGTH
        );
    }

    #[test]
    fn field_specs_are_valid() {
        for spec in SampleRecord::FIELDS.iter().chain(TaggedRecord::FIELDS) {
            assert_eq!(spec.check(), Ok(()), "{}", spec.name);
        }
    }

    #[test]
    fn decode_ignores_trailing_bytes() {
        let raw = format!("{}\n", SampleRecord::new("x", 1).encode().unwrap());
        assert_eq!(SampleRecord::decode(raw.as_bytes()).unwrap().num, 1);
    }

    #[test]
    fn decode_exact_rejects_trailing_bytes() {
        let raw = format!("{}\n", SampleRecord::new("x", 1).encode().unwrap());
        assert_eq!(
            SampleRecord::decode_exact(raw.as_bytes()),
            Err(Error::Length {
                record: "SampleRecord",
                expected: 20,
                actual: 21
            })
        );
    }

    #[test]
    fn short_input_is_length_error() {
        assert_eq!(
            SampleRecord::decode(b"hello"),
            Err(Error::Length {
                record: "SampleRecord",
                expected: 20,
                actual: 5
            })
        );
    }

    #[test]
    fn encode_error_has_context() {
        let err = SampleRecord::new("much too long", 1).encode().unwrap_err();
        assert_eq!(
            err,
            Error::Field {
                record: "SampleRecord",
                field: "alpha",
                offset: 0,
                kind: FieldErrorKind::TooLong {
                    max: 10,
                    actual: 13
                }
            }
        );
        assert_eq!(
            err.to_string(),
            "SampleRecord.alpha @ 0 - must have length <= 10, got 13"
        );
    }

    #[test]
    fn decode_error_has_context() {
        let err = SampleRecord::decode(b"hello     000x1     ").unwrap_err();
        assert!(matches!(
            err,
            Error::Field {
                field: "num",
                offset: 10,
                ..
            }
        ));
        assert!(matches!(
            err.field_kind(),
            Some(FieldErrorKind::NotNumeric { .. })
        ));
    }

    #[test]
    fn reserved_filler_must_be_blank_on_decode() {
        let err = SampleRecord::decode(b"hello     00042xxxxx").unwrap_err();
        assert!(matches!(
            err.field_kind(),
            Some(FieldErrorKind::ConstantMismatch { .. })
        ));
    }
}

mod probe {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn valid() {
        let encoded = SampleRecord::new("hello", 42).encode().unwrap();
        assert_eq!(
            SampleRecord::probe(encoded.as_bytes()),
            Some(SampleRecord::new("hello", 42))
        );
    }

    #[test]
    fn invalid_is_none() {
        assert_eq!(SampleRecord::probe(b"x"), None);
    }
}

mod tagged {
    use super::*;
    use pretty_assertions::assert_eq;

    fn rec(name: &str, closed: Option<NaiveDate>) -> TaggedRecord {
        TaggedRecord {
            tag: bryl::Const,
            name: name.into(),
            raw_name: name.into(),
            closed,
        }
    }

    #[test]
    fn constants_written() {
        assert_eq!(rec("ab", None).encode().unwrap(), "K1AB    ab    00000000");
    }

    #[test]
    fn constants_checked_on_decode() {
        let err = TaggedRecord::decode(b"J1AB    ab    00000000").unwrap_err();
        assert!(matches!(
            err,
            Error::Field {
                field: "tag",
                kind: FieldErrorKind::ConstantMismatch { .. },
                ..
            }
        ));
    }

    #[test]
    fn record_sanitize_uppercases_but_field_override_wins() {
        let encoded = rec("smith", None).encode().unwrap();
        assert_eq!(&encoded[2..8], "SMITH ");
        assert_eq!(&encoded[8..14], "smith ");
    }

    #[test]
    fn encode_with_overrides_record_default() {
        let encoded = rec("smith", None).encode_with(Sanitize::NONE).unwrap();
        assert_eq!(&encoded[2..8], "smith ");
    }

    #[test]
    fn decode_does_not_sanitize() {
        let loaded = TaggedRecord::decode(b"K1smith smith 00000000").unwrap();
        assert_eq!(loaded.name, "smith");
    }

    #[test]
    fn optional_date_roundtrip() {
        let closed = NaiveDate::from_ymd_opt(2020, 3, 15);
        let encoded = rec("A", closed).encode().unwrap();
        assert!(encoded.ends_with("03152020"));
        assert_eq!(
            TaggedRecord::decode(encoded.as_bytes()).unwrap().closed,
            closed
        );
    }

    #[test]
    fn zero_date_roundtrip() {
        let encoded = rec("A", None).encode().unwrap();
        assert_eq!(
            TaggedRecord::decode(encoded.as_bytes()).unwrap().closed,
            None
        );
    }
}
