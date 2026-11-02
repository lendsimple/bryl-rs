//! Serde support (`serde` feature): files round-trip through JSON, codes are
//! code strings, validated values stay valid, and issues serialize.

#![cfg(feature = "serde")]

mod common;

use bryl::Record;
use common::*;
use nacha::{File, IssueKind};
use pretty_assertions::assert_eq;

fn snapshot(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/snapshots/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

/// Every record of `file`, encoded, in file order (no filler).
fn lines(file: &File) -> Vec<String> {
    let mut lines = vec![file.header.encode().unwrap()];
    for batch in &file.batches {
        lines.push(batch.header.encode().unwrap());
        for entry in &batch.entries {
            lines.extend(entry.encode().unwrap().lines().map(str::to_owned));
        }
        lines.push(batch.control.encode().unwrap());
    }
    lines.push(file.control.encode().unwrap());
    lines
}

#[test]
fn snapshot_files_round_trip_through_json() {
    for name in [
        "single_entry.ach",
        "entries_with_addenda.ach",
        "two_batches.ach",
        "returns.ach",
        "loan_and_gl_entries.ach",
        "notifications_of_change.ach",
        "dishonored_returns.ach",
    ] {
        let input = snapshot(name);
        let file = File::read(input.as_bytes()).unwrap();
        let json = serde_json::to_string(&file).unwrap();
        let loaded: File = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded, file, "{name}");
        // The deserialized file encodes to the same bytes.
        let records: Vec<&str> = input.lines().take(lines(&loaded).len()).collect();
        assert_eq!(lines(&loaded), records, "{name}");
    }
}

#[test]
fn codes_and_routing_numbers_are_strings() {
    let file = File::read(return_file().as_bytes()).unwrap();
    let json = serde_json::to_value(&file).unwrap();
    let entry = &json["batches"][0]["entries"][0];
    assert_eq!(entry["detail"]["transaction_code"], "26");
    assert_eq!(entry["detail"]["receiving_dfi"], "091000019");
    assert_eq!(
        entry["special_addendum"]["Return"]["return_reason_code"],
        "R01"
    );
    assert_eq!(json["batches"][0]["header"]["standard_entry_class"], "PPD");
    assert_eq!(json["header"]["file_id_modifier"], "A");
    // Fields fixed by the format are left out.
    assert_eq!(entry["detail"].get("record_type"), None);
}

#[test]
fn invalid_values_are_rejected() {
    let file = File::read(sample_file().as_bytes()).unwrap();
    let json = serde_json::to_string(&file).unwrap();
    // A bad check digit, an unknown code and a lowercase file ID modifier.
    for (from, to) in [
        ("\"091000019\"", "\"091000018\""),
        ("\"transaction_code\":\"22\"", "\"transaction_code\":\"25\""),
        ("\"file_id_modifier\":\"A\"", "\"file_id_modifier\":\"a\""),
    ] {
        assert!(json.contains(from), "{from}");
        let bad = json.replacen(from, to, 1);
        assert!(serde_json::from_str::<File>(&bad).is_err(), "{to}");
    }
}

#[test]
fn issues_serialize() {
    let mut file = File::read(sample_file().as_bytes()).unwrap();
    file.control.batch_count = 2;
    let issues = file.validate();
    assert!(matches!(issues[0].kind, IssueKind::ControlMismatch { .. }));
    let json = serde_json::to_value(&issues).unwrap();
    assert_eq!(
        json[0],
        serde_json::json!({
            "batch_number": null,
            "trace_number": null,
            "kind": {
                "ControlMismatch": { "field": "batch_count", "computed": 1, "recorded": 2 }
            }
        })
    );
}

#[test]
fn unread_entries_serialize_but_do_not_deserialize() {
    let output = sample_file();
    let mut lines: Vec<String> = output.lines().map(str::to_owned).collect();
    // A bad routing check digit in the entry detail.
    lines[2].replace_range(3..12, "091000018");
    let file = File::read_lenient(lines.join("\n").as_bytes()).unwrap();
    assert_eq!(file.batches[0].unread_entries.len(), 1);

    let json = serde_json::to_value(&file).unwrap();
    let unread = &json["batches"][0]["unread_entries"][0];
    assert_eq!(unread["location"], serde_json::json!({ "Line": 3 }));
    assert_eq!(unread["error"]["Field"]["field"], "receiving_dfi");

    let loaded: File = serde_json::from_value(json).unwrap();
    assert_eq!(loaded.batches[0].unread_entries, []);
    assert_eq!(loaded.batches[0].entries, file.batches[0].entries);
}
