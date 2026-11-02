//! Serde support (`serde` feature): code tables as their codes, and
//! serializable errors.

#![cfg(feature = "serde")]

use bryl::{Code, Record};
use pretty_assertions::assert_eq;

#[derive(Code, Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    #[code("11")]
    Current,
    #[code("DA")]
    Delete,
}

#[derive(Code, Debug, Clone, Copy, PartialEq, Eq)]
enum Transaction {
    #[code(22)]
    Credit,
    #[code(27)]
    Debit,
}

#[test]
fn string_codes_serialize_as_codes() {
    assert_eq!(serde_json::to_string(&Status::Current).unwrap(), r#""11""#);
    assert_eq!(serde_json::to_string(&Status::Delete).unwrap(), r#""DA""#);
    assert_eq!(
        serde_json::from_str::<Status>(r#""DA""#).unwrap(),
        Status::Delete
    );
}

#[test]
fn numeric_codes_serialize_as_strings() {
    assert_eq!(
        serde_json::to_string(&Transaction::Debit).unwrap(),
        r#""27""#
    );
    assert_eq!(
        serde_json::from_str::<Transaction>(r#""22""#).unwrap(),
        Transaction::Credit
    );
}

#[test]
fn unknown_codes_are_rejected() {
    let err = serde_json::from_str::<Status>(r#""99""#).unwrap_err();
    assert!(err.to_string().contains("unknown Status code"), "{err}");
    assert!(serde_json::from_str::<Transaction>("22").is_err());
}

#[derive(Record, Debug, PartialEq)]
#[bryl(length = 2)]
struct Pair {
    #[bryl(numeric(2))]
    value: u8,
}

#[test]
fn errors_serialize() {
    let err = Pair::decode(b"x1").unwrap_err();
    let json = serde_json::to_value(&err).unwrap();
    assert_eq!(json["Field"]["record"], "Pair");
    assert_eq!(json["Field"]["field"], "value");
    assert_eq!(
        serde_json::to_value(bryl::read::Location::Line(4)).unwrap(),
        serde_json::json!({ "Line": 4 })
    );
}
