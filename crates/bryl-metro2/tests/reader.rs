//! Reader, `File` and reference data.
//!
//! | `test_metro2.py`              | Here |
//! |-------------------------------|------|
//! | `TestMalformedError`          | `malformed::*` (errors are `bryl::read::ReadError`) |
//! | `TestReader`                  | `structured::*` |
//! | `TestReaderOptionalSegments`  | `segments::*` |
//! | `TestReaderNewlineMode`       | `newline::*` |
//! | `TestReaderMalformed`         | `malformed::*` |
//! | `TestRoundtrip`               | `roundtrip::*` |
//! | `TestGoTestData*`             | `moov::*` (fixtures copied into `tests/fixtures/moov`, so these always run) |

mod common;

use bryl::Record;
use bryl::read::{Location, ReadErrorKind};
use common::*;
use metro2::{
    AccountStatus, BaseSegment, DataRecord, File, Issue, J1Segment, J2Segment, K1Segment,
    K3Segment, L1Segment, N1Segment, Reader, Segment, TrailerRecord,
};

fn fixture(path: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/{path}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn read_data_records(output: &str) -> Vec<DataRecord> {
    let mut reader = Reader::new(output.as_bytes());
    reader.header().unwrap();
    reader.data_records().collect::<Result<_, _>>().unwrap()
}

mod structured {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn header() {
        let output = write_file(&[], false);
        let header = Reader::new(output.as_bytes()).header().unwrap();
        assert_eq!(header.reporter_name, "TEST REPORTER");
        assert_eq!(header.activity_date, date(2020, 8, 20));
    }

    #[test]
    fn single_data_record() {
        let records = read_data_records(&write_file(&[DataRecord::new(base())], false));
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].base.surname, "SMITH");
    }

    #[test]
    fn multiple_data_records() {
        let input: Vec<_> = ["ACCT-001", "ACCT-002", "ACCT-003"]
            .into_iter()
            .map(|account| {
                DataRecord::new(BaseSegment {
                    consumer_account_number: account.into(),
                    ..base()
                })
            })
            .collect();
        let records = read_data_records(&write_file(&input, false));
        let accounts: Vec<_> = records
            .iter()
            .map(|r| r.base.consumer_account_number.as_str())
            .collect();
        assert_eq!(accounts, ["ACCT-001", "ACCT-002", "ACCT-003"]);
    }

    #[test]
    fn trailer() {
        let output = write_file(&[DataRecord::new(base())], false);
        let mut reader = Reader::new(output.as_bytes());
        reader.header().unwrap();
        assert_eq!(reader.data_records().count(), 1);
        let trailer = reader.trailer().unwrap();
        assert_eq!(trailer.total_base_records, 1);
        assert_eq!(trailer.block_count, 3);
        reader.finish().unwrap();
    }

    #[test]
    fn full_structured_read() {
        let record = DataRecord {
            base: BaseSegment {
                social_security_number: 123_456_789,
                ..base()
            },
            j1: vec![J1Segment {
                social_security_number: 987_654_321,
                ..j1()
            }],
            ..DataRecord::new(base())
        };
        let file = File::read(write_file(&[record], false).as_bytes()).unwrap();
        assert_eq!(file.header.reporter_name, "TEST REPORTER");
        assert_eq!(file.data_records.len(), 1);
        assert_eq!(
            file.data_records[0].base.social_security_number,
            123_456_789
        );
        assert_eq!(file.data_records[0].j1.len(), 1);
        assert_eq!(file.trailer.total_ssns_all_segments, 2);
        assert_eq!(file.validate(), []);
    }

    #[test]
    fn flat_segments() {
        let record = DataRecord {
            j1: vec![j1()],
            ..DataRecord::new(base())
        };
        let output = write_file(&[record], false);
        let segments: Vec<_> = Reader::new(output.as_bytes())
            .segments()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(segments.len(), 4);
        assert!(matches!(segments[0], Segment::Header(_)));
        assert!(matches!(segments[1], Segment::Base(_)));
        assert!(matches!(segments[2], Segment::J1(_)));
        assert!(matches!(segments[3], Segment::Trailer(_)));
    }
}

mod segments {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn j1() {
        let record = DataRecord {
            j1: vec![J1Segment {
                social_security_number: 111_222_333,
                ..common::j1()
            }],
            ..DataRecord::new(base())
        };
        let records = read_data_records(&write_file(&[record], false));
        assert_eq!(records[0].j1[0].social_security_number, 111_222_333);
    }

    #[test]
    fn j2() {
        let record = DataRecord {
            j2: vec![J2Segment {
                country_code: "US".into(),
                ..common::j2()
            }],
            ..DataRecord::new(base())
        };
        let records = read_data_records(&write_file(&[record], false));
        assert_eq!(records[0].j2[0].country_code, "US");
    }

    #[test]
    fn all_segments_roundtrip() {
        let records = read_data_records(&write_file(&[all_segments()], false));
        assert_eq!(
            records[0].k1.as_ref().unwrap().original_creditor_name,
            "BANK"
        );
        assert!(records[0].k2.is_some() && records[0].k3.is_some() && records[0].k4.is_some());
        assert_eq!(records[0].n1.as_ref().unwrap().employer_name, "ACME");
    }

    #[test]
    fn mixed_segments() {
        let record = DataRecord {
            j1: vec![
                common::j1(),
                J1Segment {
                    surname: "DOE".into(),
                    ..common::j1()
                },
            ],
            j2: vec![common::j2()],
            k1: Some(k1()),
            l1: Some(l1()),
            n1: Some(n1()),
            ..DataRecord::new(base())
        };
        let records = read_data_records(&write_file(&[record], false));
        assert_eq!(records[0].j1.len(), 2);
        assert_eq!(records[0].j2.len(), 1);
        assert!(records[0].k1.is_some() && records[0].l1.is_some() && records[0].n1.is_some());
    }

    #[test]
    fn rdw_matches_segment_sum() {
        let record = DataRecord {
            j1: vec![common::j1()],
            k1: Some(k1()),
            ..DataRecord::new(base())
        };
        let records = read_data_records(&write_file(&[record], false));
        assert_eq!(records[0].base.record_descriptor_word, 426 + 100 + 34);
    }
}

mod newline {
    use super::*;
    use pretty_assertions::assert_eq;

    fn read(output: &str) -> File {
        Reader::new(output.as_bytes())
            .newline(true)
            .read_file()
            .unwrap()
    }

    #[test]
    fn newline_delimited_file() {
        let file = read(&write_file(&[DataRecord::new(base())], true));
        assert_eq!(file.data_records.len(), 1);
        assert_eq!(file.validate(), []);
    }

    #[test]
    fn mixed_record_sizes() {
        let file = read(&write_file(
            &[
                DataRecord::new(base()),
                DataRecord {
                    j1: vec![j1()],
                    ..DataRecord::new(base())
                },
            ],
            true,
        ));
        assert!(file.data_records[0].j1.is_empty());
        assert_eq!(file.data_records[1].j1.len(), 1);
    }

    #[test]
    fn empty_lines_ignored() {
        let output = write_file(&[DataRecord::new(base())], true).replace('\n', "\n\n");
        assert_eq!(read(&output).data_records.len(), 1);
    }

    #[test]
    fn errors_are_located_by_line() {
        let output =
            write_file(&[DataRecord::new(base())], true).replace("0426TRAILER", "0426TRAILEX");
        let err = Reader::new(output.as_bytes())
            .newline(true)
            .read_file()
            .unwrap_err();
        assert_eq!(err.location, Location::Line(3));
    }
}

mod malformed {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn message_format() {
        let err = Reader::new(&b"042"[..])
            .with_name("report.dat")
            .header()
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "report.dat @ offset 0 - truncated record: expected 4 bytes, got 3"
        );
    }

    #[test]
    fn truncated_header() {
        let err = Reader::new(&b"042"[..]).header().unwrap_err();
        assert!(matches!(err.kind, ReadErrorKind::Truncated { .. }));
        assert!(err.to_string().contains("truncated record"));
    }

    #[test]
    fn invalid_rdw() {
        let input = format!("ABCD{}", " ".repeat(422));
        let err = Reader::new(input.as_bytes()).header().unwrap_err();
        assert!(err.to_string().contains("invalid RDW"), "{err}");
    }

    #[test]
    fn rdw_too_small() {
        let err = Reader::new(&b"0003"[..]).header().unwrap_err();
        assert!(err.to_string().contains("invalid RDW: 3"), "{err}");
    }

    #[test]
    fn rdw_longer_than_input() {
        let input = format!("1000{}", " ".repeat(100));
        let err = Reader::new(input.as_bytes()).header().unwrap_err();
        assert!(matches!(
            err.kind,
            ReadErrorKind::Truncated {
                expected: 1000,
                actual: 104
            }
        ));
    }

    fn file_with_raw_data_record(raw: &str) -> String {
        let trailer = TrailerRecord {
            total_base_records: 1,
            block_count: 3,
            ..TrailerRecord::default()
        };
        format!(
            "{}{raw}{}",
            header().encode().unwrap(),
            trailer.encode().unwrap()
        )
    }

    fn base_with_rdw(rdw: u16) -> String {
        BaseSegment {
            record_descriptor_word: rdw,
            ..base()
        }
        .encode()
        .unwrap()
    }

    #[test]
    fn unknown_segment_id_ignored() {
        let input =
            file_with_raw_data_record(&format!("{}ZZ{}", base_with_rdw(460), " ".repeat(32)));
        let records = read_data_records(&input);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].base.surname, "SMITH");
        assert!(records[0].j1.is_empty());
    }

    #[test]
    fn truncated_optional_segment() {
        let input =
            file_with_raw_data_record(&format!("{}J1{}", base_with_rdw(460), " ".repeat(32)));
        let mut reader = Reader::new(input.as_bytes());
        reader.header().unwrap();
        let err = reader.data_records().next().unwrap().unwrap_err();
        assert!(err.to_string().contains("truncated J1 segment"), "{err}");
        // M10: located at the data record, not offset 0.
        assert_eq!(err.location, Location::Offset(426));
    }

    #[test]
    fn unknown_record_type_is_located() {
        let mut raw = base_with_rdw(426);
        raw.replace_range(4..5, "9");
        let input = file_with_raw_data_record(&raw);
        let mut reader = Reader::new(input.as_bytes());
        reader.header().unwrap();
        let err = reader.data_records().next().unwrap().unwrap_err();
        assert!(matches!(err.kind, ReadErrorKind::UnknownRecord(_)));
        assert_eq!(err.location, Location::Offset(426));
    }

    #[test]
    fn missing_header() {
        let output = write_file(&[DataRecord::new(base())], false);
        let mut reader = Reader::new(&output.as_bytes()[426..]);
        let err = reader.header().unwrap_err();
        assert!(matches!(
            err.kind,
            ReadErrorKind::UnexpectedRecord {
                found: "DataRecord",
                ..
            }
        ));
    }

    #[test]
    fn missing_trailer() {
        let output = write_file(&[DataRecord::new(base())], false);
        let err = File::read(&output.as_bytes()[..852]).unwrap_err();
        assert!(matches!(err.kind, ReadErrorKind::UnexpectedEof { .. }));
    }

    #[test]
    fn record_after_trailer() {
        let mut output = write_file(&[], false);
        output.push_str(&DataRecord::new(base()).encode().unwrap());
        let err = File::read(output.as_bytes()).unwrap_err();
        assert!(matches!(
            err.kind,
            ReadErrorKind::UnexpectedRecord {
                found: "DataRecord",
                ..
            }
        ));
    }

    #[test]
    fn invalid_code_in_file() {
        let mut raw = base_with_rdw(426);
        raw.replace_range(123..125, "99");
        let err = File::read(file_with_raw_data_record(&raw).as_bytes()).unwrap_err();
        assert!(err.to_string().contains("account_status"), "{err}");
    }
}

mod roundtrip {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn header() {
        let header = Reader::new(write_file(&[], false).as_bytes())
            .header()
            .unwrap();
        assert_eq!(header.reporter_address, "123 MAIN ST ANYTOWN US 12345");
        assert_eq!(header.reporter_telephone_number, 5_551_234_567);
    }

    #[test]
    fn multiple_records() {
        let input: Vec<_> = (0..5)
            .map(|i| {
                DataRecord::new(BaseSegment {
                    consumer_account_number: format!("ACCT-{i:03}"),
                    ..base()
                })
            })
            .collect();
        let file = File::read(write_file(&input, false).as_bytes()).unwrap();
        for (i, record) in file.data_records.iter().enumerate() {
            assert_eq!(record.base.consumer_account_number, format!("ACCT-{i:03}"));
        }
    }

    #[test]
    fn trailer_totals() {
        let input: Vec<_> = (0..3)
            .map(|i| DataRecord {
                base: BaseSegment {
                    social_security_number: 100_000_000 + i,
                    date_of_birth: Some(date(1990, 1, 1)),
                    telephone_number: 5_550_000_000 + u64::from(i),
                    ..base()
                },
                j1: vec![J1Segment {
                    social_security_number: 200_000_000 + i,
                    ..j1()
                }],
                ..DataRecord::new(base())
            })
            .collect();
        let trailer = File::read(write_file(&input, false).as_bytes())
            .unwrap()
            .trailer;
        assert_eq!(trailer.total_base_records, 3);
        assert_eq!(trailer.total_j1_segments, 3);
        assert_eq!(trailer.total_status_code_11, 3);
        assert_eq!(trailer.total_ssns_all_segments, 6);
        assert_eq!(trailer.total_dobs_all_segments, 3);
        assert_eq!(trailer.total_telephone_numbers, 3);
        assert_eq!(trailer.block_count, 5);
    }

    #[test]
    fn records_equal_after_roundtrip() {
        let input = vec![
            all_segments(),
            DataRecord::new(base_with_status(AccountStatus::Dpd60)),
        ];
        let file = File::read(write_file(&input, false).as_bytes()).unwrap();
        for (written, read) in input.iter().zip(&file.data_records) {
            assert_eq!(
                read,
                &DataRecord {
                    base: BaseSegment {
                        record_descriptor_word: u16::try_from(written.len()).unwrap(),
                        ..written.base.clone()
                    },
                    ..written.clone()
                }
            );
        }
    }
}

mod validate {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn trailer_mismatch() {
        let mut file =
            File::read(write_file(&[DataRecord::new(base())], false).as_bytes()).unwrap();
        file.trailer.total_status_code_11 = 2;
        file.trailer.block_count = 9;
        assert_eq!(
            file.validate(),
            [
                Issue::TrailerMismatch {
                    field: "block_count",
                    computed: 3,
                    recorded: 9
                },
                Issue::TrailerMismatch {
                    field: "total_status_code_11",
                    computed: 1,
                    recorded: 2
                },
            ]
        );
    }

    #[test]
    fn base_violations() {
        let mut file =
            File::read(write_file(&[DataRecord::new(base())], false).as_bytes()).unwrap();
        file.data_records[0].base.amount_past_due = 5;
        let issues = file.validate();
        assert_eq!(issues.len(), 1);
        assert_eq!(
            issues[0].to_string(),
            "data record 0 (account ACCT-000001): Amount past due must be 0 for status '11', got 5"
        );
    }
}

mod moov {
    //! Reference files from moov-io/metro2 (Apache-2.0; see
    //! `tests/fixtures/moov/LICENSE`).
    use super::*;
    use pretty_assertions::assert_eq;

    fn fixed_file() -> File {
        File::read(fixture("moov/unpacked_fixed_file.dat").as_bytes()).unwrap()
    }

    #[test]
    fn fixed_file_validates() {
        assert_eq!(fixed_file().validate(), []);
    }

    #[test]
    fn header_fields() {
        let header = fixed_file().header;
        assert_eq!(header.reporter_name, "YOUR BUSINESS NAME HERE");
        assert_eq!(header.reporter_telephone_number, 1_234_567_890);
        assert_eq!(header.activity_date, date(2002, 8, 20));
        assert_eq!(header.date_created, date(1999, 5, 10));
    }

    #[test]
    fn data_record_fields() {
        let file = fixed_file();
        assert_eq!(file.data_records.len(), 1);
        let base = &file.data_records[0].base;
        assert_eq!(base.consumer_account_number, "553723456");
        assert_eq!(base.social_security_number, 159_328_759);
        assert_eq!(base.surname, "SMITH-JONES");
        assert_eq!(base.first_name, "JUNIOR");
        assert_eq!(base.account_status, AccountStatus::MfrCollected);
        assert_eq!(base.ecoa_code, metro2::EcoaCode::Individual);
        // Fixed-length format: the record is padded to its RDW of 1000.
        assert_eq!(base.record_descriptor_word, 1000);
    }

    #[test]
    fn optional_segments() {
        let record = &fixed_file().data_records[0];
        assert_eq!(record.j1.len(), 1);
        assert_eq!(record.j1[0].surname, "BEAUCHAMP");
        assert_eq!(record.j2.len(), 1);
        assert_eq!(record.j2[0].surname, "BEAUCHAMP");
        assert!(record.k3.is_some() && record.l1.is_some() && record.n1.is_some());
    }

    #[test]
    fn trailer_totals() {
        let t = fixed_file().trailer;
        assert_eq!(t.total_base_records, 1);
        assert_eq!(t.total_j1_segments, 1);
        assert_eq!(t.total_j2_segments, 1);
        assert_eq!(t.block_count, 3);
        assert_eq!(t.total_status_code_62, 1);
        assert_eq!(t.total_ssns_all_segments, 3);
        assert_eq!(t.total_ssns_base_segments, 1);
        assert_eq!(t.total_ssns_j1_segments, 1);
        assert_eq!(t.total_ssns_j2_segments, 1);
        assert_eq!(t.total_dobs_all_segments, 3);
        assert_eq!(t.total_telephone_numbers, 3);
        assert_eq!(t.total_n1_segments, 1);
        assert_eq!(t.total_k3_segments, 1);
        assert_eq!(t.total_l1_segments, 1);
    }

    #[test]
    fn newline_file() {
        let file = Reader::new(fixture("moov/unpacked_fixed_request.dat").as_bytes())
            .newline(true)
            .read_file()
            .unwrap();
        assert_eq!(file.header.reporter_name, "YOUR BUSINESS NAME HERE");
        assert_eq!(file.header.activity_date, date(2002, 8, 20));
        assert_eq!(file.data_records.len(), 1);
        assert_eq!(file.trailer.total_base_records, 1);
        assert_eq!(file.trailer.block_count, 3);
        assert_eq!(file.validate(), []);
    }

    #[test]
    fn individual_segments() {
        let j1 = J1Segment::decode(fixture("moov/j1_segment.dat").as_bytes()).unwrap();
        assert_eq!(j1.surname, "BEAUCHAMP");
        K1Segment::decode(fixture("moov/k1_segment.dat").as_bytes()).unwrap();
        K3Segment::decode(fixture("moov/k3_segment.dat").as_bytes()).unwrap();
        L1Segment::decode(fixture("moov/l1_segment.dat").as_bytes()).unwrap();
        let n1 = N1Segment::decode(fixture("moov/n1_segment.dat").as_bytes()).unwrap();
        // B5: Python's test expected "EMPLOYER NAME" only because loading
        // uppercased; the file says "Employer Name".
        assert_eq!(n1.employer_name, "Employer Name");
    }

    /// moov-io hard-wraps some fixtures with newlines for readability; the
    /// content is valid once unwrapped. Embedded newlines themselves are
    /// rejected (B6).
    #[test]
    fn wrapped_fixtures() {
        let unwrap = |path| fixture(path).replace('\n', "");
        let raw = fixture("moov/j2_segment.dat");
        assert!(J2Segment::decode(raw.as_bytes()).is_err());
        let j2 = J2Segment::decode(unwrap("moov/j2_segment.dat").as_bytes()).unwrap();
        assert_eq!(j2.surname, "BEAUCHAMP");
        let trailer = TrailerRecord::decode(unwrap("moov/trailer_record.dat").as_bytes()).unwrap();
        assert_eq!(trailer.total_base_records, 1);
        // header_record.dat and base_segment.dat start with a block
        // descriptor word (`0430` before `0426HEADER`): the variable-blocked
        // format, not supported yet (M12).
    }
}
