//! `#[derive(Record)]` and `#[derive(Code)]`.
//!
//! | `test_bryl.py`              | Here |
//! |-----------------------------|------|
//! | `TestRecordInheritance::*`  | `flatten::*` (composition instead of inheritance, B1) |
//! | `TestFieldEnum::*`          | `code::*` (enums instead of `enum=` lists/dicts, B12) |
//! | `TestField::test_value_property` | `record::constant_accessors` |
//!
//! Compile-time errors are covered by `ui.rs` (trybuild).

mod common;

use bryl::{Align, Const, Error, FieldErrorKind, FieldKind, FieldSpec, Record, Sanitize, Token};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use common::{SampleRecord, TaggedRecord};

mod record {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn derived_specs_match_hand_written() {
        assert_eq!(
            SampleRecord::FIELDS,
            &[
                FieldSpec::alpha("alpha", 0, 10),
                FieldSpec::numeric("num", 10, 5),
                FieldSpec::alpha("filler", 15, 5).reserved(),
            ]
        );
        assert_eq!(
            TaggedRecord::FIELDS,
            &[
                FieldSpec::alpha("tag", 0, 2).with_constant_str("K1"),
                FieldSpec::alpha("name", 2, 6),
                FieldSpec::alpha("raw_name", 8, 6).with_sanitize(Sanitize::NONE),
                FieldSpec::date("closed", 14, common::MMDDYYYY),
            ]
        );
    }

    #[test]
    fn name_length_and_sanitize() {
        assert_eq!(SampleRecord::NAME, "SampleRecord");
        assert_eq!(SampleRecord::LENGTH, 20);
        assert_eq!(SampleRecord::SANITIZE, Sanitize::NONE);
        assert_eq!(TaggedRecord::SANITIZE, Sanitize::UPPER);
    }

    #[test]
    fn constant_accessors() {
        assert_eq!(TaggedRecord::TAG, "K1");
        assert_eq!(Everything::RDW, 426);
    }

    #[test]
    fn field_lookup() {
        assert_eq!(SampleRecord::field("num").map(|f| f.offset), Some(10));
        assert_eq!(SampleRecord::field("nope"), None);
    }

    /// One field of every kind and option.
    #[derive(Record, Debug, Clone, PartialEq)]
    #[bryl(sanitize(upper, truncate))]
    struct Everything {
        #[bryl(numeric(4), constant = 426)]
        rdw: Const,
        #[bryl(alpha(5))]
        text: String,
        #[bryl(alpha(6), align = "right", pad = '*')]
        right: String,
        #[bryl(numeric(10), pad = ' ')]
        spaced: u32,
        #[bryl(numeric(3), min = 1, max = 500)]
        bounded: u16,
        #[bryl(numeric(1), reserved)]
        zero: Const,
        #[bryl(date("YYMMDD"))]
        date: NaiveDate,
        #[bryl(time("hhmm"))]
        time: NaiveTime,
        #[bryl(datetime("MMDDYYYYhhmmss"))]
        stamp: Option<NaiveDateTime>,
        #[bryl(alpha(3), sanitize(filter))]
        filtered: String,
        #[bryl(alpha(2))]
        r#type: Option<String>,
    }

    fn everything() -> Everything {
        Everything {
            rdw: Const,
            text: "abcdefg".into(),
            right: "x".into(),
            spaced: 123_456_789,
            bounded: 42,
            zero: Const,
            date: NaiveDate::from_ymd_opt(2023, 3, 5).unwrap(),
            time: NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
            stamp: None,
            filtered: "a\tb".into(),
            r#type: Some("ab".into()),
        }
    }

    #[test]
    fn every_option_encodes() {
        let encoded = everything().encode().unwrap();
        assert_eq!(
            encoded,
            [
                "0426",
                "ABCDE",
                "*****X",
                " 123456789",
                "042",
                "0",
                "230305",
                "1430",
                "00000000000000",
                "ab ",
                "AB",
            ]
            .concat()
        );
        assert_eq!(encoded.len(), Everything::LENGTH);
    }

    #[test]
    fn every_option_decodes() {
        let decoded = Everything::decode(everything().encode().unwrap().as_bytes()).unwrap();
        assert_eq!(
            decoded,
            Everything {
                text: "ABCDE".into(),
                right: "X".into(),
                filtered: "ab".into(),
                r#type: Some("AB".into()),
                ..everything()
            }
        );
    }

    #[test]
    fn raw_identifier_name() {
        assert_eq!(Everything::FIELDS.last().unwrap().name, "type");
    }

    #[test]
    fn options_land_in_specs() {
        let right = Everything::field("right").unwrap();
        assert_eq!((right.align, right.pad), (Align::Right, b'*'));
        assert_eq!(
            Everything::field("bounded").unwrap().kind,
            FieldKind::Numeric {
                min: Some(1),
                max: Some(500)
            }
        );
        assert_eq!(
            Everything::field("date").unwrap().kind,
            FieldKind::Date(&[Token::Year2, Token::Month, Token::Day])
        );
        assert_eq!(
            Everything::field("filtered").unwrap().sanitize,
            Some(Sanitize::NONE.filter(true))
        );
    }

    #[test]
    fn bounds_enforced() {
        let err = Everything {
            bounded: 0,
            ..everything()
        }
        .encode()
        .unwrap_err();
        assert_eq!(
            err,
            Error::Field {
                record: "Everything",
                field: "bounded",
                offset: 25,
                kind: FieldErrorKind::BelowMin { min: 1, value: 0 }
            }
        );
    }

    #[test]
    fn specs_are_valid() {
        for spec in Everything::FIELDS {
            assert_eq!(spec.check(), Ok(()), "{}", spec.name);
        }
    }
}

mod flatten {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Port of `ChildRecord(SampleRecord)` with `extra = Alphanumeric(4)`.
    #[derive(Record, Debug, Clone, PartialEq)]
    #[bryl(length = 24)]
    struct ChildRecord {
        #[bryl(flatten)]
        base: SampleRecord,
        #[bryl(alpha(4))]
        extra: String,
    }

    /// Flattened record in the middle, with fields on both sides.
    #[derive(Record, Debug, Clone, PartialEq)]
    #[bryl(length = 25)]
    struct Sandwich {
        #[bryl(alpha(2), constant = "SW")]
        tag: Const,
        #[bryl(flatten)]
        inner: SampleRecord,
        #[bryl(numeric(3))]
        count: u16,
    }

    fn child() -> ChildRecord {
        ChildRecord {
            base: SampleRecord::new("test", 1),
            extra: "ABCD".into(),
        }
    }

    #[test]
    fn child_inherits_fields() {
        let names: Vec<_> = ChildRecord::FIELDS.iter().map(|f| f.name).collect();
        assert_eq!(names, ["alpha", "num", "filler", "extra"]);
    }

    #[test]
    fn child_length_includes_all() {
        assert_eq!(ChildRecord::LENGTH, SampleRecord::LENGTH + 4);
    }

    #[test]
    fn child_roundtrip() {
        let loaded = ChildRecord::decode(child().encode().unwrap().as_bytes()).unwrap();
        assert_eq!(loaded, child());
        assert_eq!(loaded.base.alpha, "test");
        assert_eq!(loaded.extra, "ABCD");
    }

    #[test]
    fn offsets_are_shifted() {
        let offsets: Vec<_> = Sandwich::FIELDS
            .iter()
            .map(|f| (f.name, f.offset))
            .collect();
        assert_eq!(
            offsets,
            [
                ("tag", 0),
                ("alpha", 2),
                ("num", 12),
                ("filler", 17),
                ("count", 22)
            ]
        );
    }

    #[test]
    fn sandwich_roundtrip() {
        let value = Sandwich {
            tag: Const,
            inner: SampleRecord::new("mid", 7),
            count: 9,
        };
        let encoded = value.encode().unwrap();
        assert_eq!(encoded, "SWmid       00007     009");
        assert_eq!(Sandwich::decode(encoded.as_bytes()).unwrap(), value);
    }

    #[test]
    fn encode_errors_are_reported_against_the_outer_record() {
        let err = Sandwich {
            tag: Const,
            inner: SampleRecord::new("much too long", 7),
            count: 9,
        }
        .encode()
        .unwrap_err();
        assert!(matches!(
            err,
            Error::Field {
                record: "Sandwich",
                field: "alpha",
                offset: 2,
                kind: FieldErrorKind::TooLong { .. }
            }
        ));
    }

    #[test]
    fn decode_errors_are_reported_against_the_outer_record() {
        let err = Sandwich::decode(b"SWmid       0000x     009").unwrap_err();
        assert!(matches!(
            err,
            Error::Field {
                record: "Sandwich",
                field: "num",
                offset: 12,
                ..
            }
        ));
    }

    #[test]
    fn flattened_specs_are_valid() {
        for spec in Sandwich::FIELDS {
            assert_eq!(spec.check(), Ok(()), "{}", spec.name);
        }
    }
}

mod code {
    use super::*;
    use pretty_assertions::assert_eq;

    #[derive(bryl::Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum Status {
        #[code("05")]
        Transferred,
        #[code("11")]
        Current,
        #[code("DA")]
        DeleteAccount,
    }

    #[derive(bryl::Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum ServiceClass {
        #[code(200)]
        Mixed,
        #[code(220)]
        CreditsOnly,
        #[code(225)]
        DebitsOnly,
    }

    #[derive(bryl::Code, Debug, Clone, Copy, PartialEq, Eq)]
    enum Classification {
        #[code(1)]
        Retail,
        #[code(12)]
        Financial,
    }

    #[derive(Record, Debug, Clone, PartialEq)]
    struct Coded {
        #[bryl(alpha(2))]
        status: Status,
        #[bryl(numeric(3))]
        service_class: ServiceClass,
        #[bryl(numeric(2))]
        classification: Classification,
        #[bryl(alpha(2))]
        optional: Option<Status>,
    }

    #[test]
    fn all_variants() {
        assert_eq!(
            Status::ALL,
            &[Status::Transferred, Status::Current, Status::DeleteAccount]
        );
        assert_eq!(ServiceClass::ALL.len(), 3);
    }

    #[test]
    fn as_code_and_from_code() {
        assert_eq!(Status::Current.as_code(), "11");
        assert_eq!(Status::from_code("DA"), Some(Status::DeleteAccount));
        assert_eq!(Status::from_code("99"), None);
        assert_eq!(ServiceClass::CreditsOnly.as_code(), 220);
        assert_eq!(ServiceClass::from_code(225), Some(ServiceClass::DebitsOnly));
        assert_eq!(ServiceClass::from_code(1), None);
    }

    #[test]
    fn display_and_parse() {
        assert_eq!(Status::Transferred.to_string(), "05");
        assert_eq!(ServiceClass::Mixed.to_string(), "200");
        assert_eq!("11".parse::<Status>(), Ok(Status::Current));
        assert_eq!("220".parse::<ServiceClass>(), Ok(ServiceClass::CreditsOnly));
        assert_eq!(
            "XX".parse::<Status>(),
            Err(bryl::UnknownCode {
                type_name: "Status",
                code: "XX".into()
            })
        );
        assert!("abc".parse::<ServiceClass>().is_err());
    }

    #[test]
    fn try_from() {
        assert_eq!(Status::try_from("05"), Ok(Status::Transferred));
        assert_eq!(ServiceClass::try_from(200), Ok(ServiceClass::Mixed));
        assert_eq!(
            ServiceClass::try_from(201).unwrap_err().to_string(),
            "unknown ServiceClass code \"201\""
        );
    }

    fn coded() -> Coded {
        Coded {
            status: Status::DeleteAccount,
            service_class: ServiceClass::DebitsOnly,
            classification: Classification::Retail,
            optional: None,
        }
    }

    #[test]
    fn record_roundtrip() {
        let encoded = coded().encode().unwrap();
        assert_eq!(encoded, "DA22501  ");
        assert_eq!(Coded::decode(encoded.as_bytes()).unwrap(), coded());
    }

    #[test]
    fn optional_code() {
        let value = Coded {
            optional: Some(Status::Current),
            ..coded()
        };
        let encoded = value.encode().unwrap();
        assert!(encoded.ends_with("11"));
        assert_eq!(Coded::decode(encoded.as_bytes()).unwrap(), value);
    }

    #[test]
    fn unknown_code_rejected_on_decode() {
        // Python's Metro 2 code tables never validated fields (M3).
        let err = Coded::decode(b"9922501  ").unwrap_err();
        assert_eq!(
            err,
            Error::Field {
                record: "Coded",
                field: "status",
                offset: 0,
                kind: FieldErrorKind::UnknownCode("99".into())
            }
        );
        let err = Coded::decode(b"DA20101  ").unwrap_err();
        assert_eq!(
            err.field_kind(),
            Some(&FieldErrorKind::UnknownCode("201".into()))
        );
    }

    #[test]
    fn numeric_code_must_fit() {
        let spec = FieldSpec::numeric("n", 0, 2);
        let mut out = Vec::new();
        assert!(matches!(
            bryl::FieldValue::encode(&ServiceClass::Mixed, &spec, Sanitize::NONE, &mut out),
            Err(FieldErrorKind::TooLong { .. })
        ));
    }
}
