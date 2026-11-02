//! Serde support (`serde` feature): files round-trip through JSON, codes are
//! code strings, the payment history profile stays valid, and issues
//! serialize.

#![cfg(feature = "serde")]

use metro2::{File, Issue, Reader, Violation, Writer};
use pretty_assertions::assert_eq;

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn round_trip(file: &File) -> File {
    let json = serde_json::to_string(file).unwrap();
    serde_json::from_str(&json).unwrap()
}

/// Writes `file` back out, without validation, as RDW or newline records.
fn write(file: &File, newline: bool) -> String {
    let mut writer = Writer::new(Vec::new()).newline(newline).validate(false);
    let mut out = writer.begin_file(&file.header).unwrap();
    for record in &file.data_records {
        out.write(record).unwrap();
    }
    out.finish().unwrap();
    String::from_utf8(writer.into_inner()).unwrap()
}

#[test]
fn snapshot_files_round_trip_through_json() {
    for (name, newline) in [
        ("snapshots/base_only.dat", false),
        ("snapshots/all_segments.dat", false),
        ("snapshots/statuses.dat", false),
        ("snapshots/statuses_newline.dat", true),
    ] {
        let input = fixture(name);
        let file = Reader::new(input.as_bytes())
            .newline(newline)
            .read_file()
            .unwrap();
        let loaded = round_trip(&file);
        assert_eq!(loaded, file, "{name}");
        // The deserialized file writes the same bytes.
        assert_eq!(write(&loaded, newline), input, "{name}");
    }
}

#[test]
fn moov_reference_file_round_trips() {
    let file = File::read(fixture("moov/unpacked_fixed_file.dat").as_bytes()).unwrap();
    assert_eq!(round_trip(&file), file);
}

#[test]
fn codes_and_profiles_are_strings() {
    let file = Reader::new(fixture("snapshots/statuses.dat").as_bytes())
        .read_file()
        .unwrap();
    let json = serde_json::to_value(&file).unwrap();
    let records = &json["data_records"];
    assert_eq!(records[0]["base"]["account_status"], "11");
    assert_eq!(records[1]["base"]["account_status"], "97");
    assert_eq!(records[1]["base"]["special_comment"], "AU");
    assert_eq!(records[0]["base"]["portfolio_type"], "I");
    assert_eq!(records[0]["base"]["account_type"], "01");
    assert!(records[0]["base"]["payment_history_profile"].is_string());
    // Fields fixed by the format are left out; the descriptor word is data.
    assert_eq!(records[0]["base"].get("processing_indicator"), None);
    assert_eq!(records[0]["base"]["record_descriptor_word"], 426);

    // A profile with a character that is not a payment history code.
    let mut bad = json.clone();
    bad["data_records"][0]["base"]["payment_history_profile"] = "0!".into();
    assert!(serde_json::from_value::<File>(bad).is_err());
    let mut bad = json;
    bad["data_records"][0]["base"]["account_status"] = "12".into();
    assert!(serde_json::from_value::<File>(bad).is_err());
}

#[test]
fn issues_serialize() {
    let mut file = File::read(fixture("moov/unpacked_fixed_file.dat").as_bytes()).unwrap();
    file.header.reporter_name = String::new();
    let issues = file.validate();
    assert_eq!(
        issues[0],
        Issue::Header {
            violation: Violation::HeaderFieldRequired {
                field: "reporter_name"
            }
        }
    );
    let json = serde_json::to_value(&issues).unwrap();
    assert_eq!(
        json[0],
        serde_json::json!({ "Header": { "violation": { "HeaderFieldRequired": { "field": "reporter_name" } } } })
    );
    let k1 = json
        .as_array()
        .unwrap()
        .iter()
        .find(|issue| issue["Violation"]["violation"].get("K1Required").is_some())
        .unwrap();
    assert_eq!(
        k1["Violation"]["violation"]["K1Required"]["account_type"],
        "48"
    );
}
