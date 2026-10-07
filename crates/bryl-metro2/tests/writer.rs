//! Writer.
//!
//! | `test_metro2.py`            | Here |
//! |-----------------------------|------|
//! | `TestWriter`                | `layout::*` |
//! | `TestWriterTrailerTotals`   | `totals::*` |
//! | `TestWriterErrors`          | Compile errors now (typestate guards); see the doctests on `metro2::Writer`. Plus `validation::*` (M7). |
//! | `TestWriterNewlineMode`     | `newline::*` |

mod common;

use bryl::Record;
use common::*;
use metro2::{
    AccountStatus, BaseSegment, DataRecord, EcoaCode, Error, HeaderRecord, J1Segment, J2Segment,
    TrailerRecord, Violation, Writer,
};

fn trailer_of(records: &[DataRecord]) -> TrailerRecord {
    let output = write_file(records, false);
    TrailerRecord::decode(&output.as_bytes()[output.len() - 426..]).unwrap()
}

mod layout {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn header_first() {
        let output = write_file(&[], false);
        assert_eq!(&output[..10], "0426HEADER");
    }

    #[test]
    fn trailer_last() {
        let output = write_file(&[], false);
        assert_eq!(output.len(), 426 * 2);
        assert_eq!(&output[430..437], "TRAILER");
    }

    #[test]
    fn data_record_written() {
        assert_eq!(write_file(&[DataRecord::new(base())], false).len(), 426 * 3);
    }

    #[test]
    fn data_record_with_j1() {
        let record = DataRecord {
            j1: vec![j1()],
            ..DataRecord::new(base())
        };
        assert_eq!(write_file(&[record], false).len(), 426 + 526 + 426);
    }

    #[test]
    fn data_record_with_all_segments() {
        assert_eq!(
            write_file(&[all_segments()], false).len(),
            426 + ALL_SEGMENTS_LENGTH + 426
        );
    }

    #[test]
    fn multiple_data_records() {
        let records = [
            DataRecord::new(base()),
            DataRecord::new(BaseSegment {
                consumer_account_number: "ACCT-002".into(),
                ..base()
            }),
        ];
        assert_eq!(write_file(&records, false).len(), 426 * 4);
    }

    #[test]
    fn header_values() {
        let output = write_file(&[], false);
        let header = HeaderRecord::decode(&output.as_bytes()[..426]).unwrap();
        assert_eq!(header.activity_date, date(2020, 8, 20));
        assert_eq!(header.date_created, date(2020, 8, 20));
    }

    #[test]
    fn finish_returns_the_trailer() {
        let mut writer = Writer::new(Vec::new());
        let mut file = writer.begin_file(&header()).unwrap();
        file.write(&DataRecord::new(base())).unwrap();
        assert_eq!(file.trailer().total_base_records, 1);
        let trailer = file.finish().unwrap();
        assert_eq!(trailer.block_count, 3);
        let output = writer.into_inner();
        assert_eq!(
            output[output.len() - 426..],
            *trailer.encode().unwrap().as_bytes()
        );
    }

    #[test]
    fn dropped_file_writes_no_trailer() {
        let mut writer = Writer::new(Vec::new());
        let file = writer.begin_file(&header()).unwrap();
        drop(file);
        assert_eq!(writer.into_inner().len(), 426);
    }
}

mod totals {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn total_base_records() {
        let t = trailer_of(&vec![DataRecord::new(base()); 3]);
        assert_eq!(t.total_base_records, 3);
    }

    #[test]
    fn block_count() {
        let t = trailer_of(&vec![DataRecord::new(base()); 2]);
        assert_eq!(t.block_count, 4);
    }

    #[test]
    fn status_codes() {
        let records: Vec<_> = [
            AccountStatus::Current,
            AccountStatus::Current,
            AccountStatus::ChargeOff,
            AccountStatus::DeleteAccount,
        ]
        .into_iter()
        .map(|s| DataRecord::new(base_with_status(s)))
        .collect();
        let t = trailer_of(&records);
        assert_eq!(t.total_status_code_11, 2);
        assert_eq!(t.total_status_code_97, 1);
        assert_eq!(t.total_status_code_da, 1);
        assert_eq!(t.total_status_code_05, 0);
    }

    #[test]
    fn every_status_has_a_counter() {
        let records: Vec<_> = AccountStatus::ALL
            .iter()
            .map(|&s| DataRecord::new(base_with_status(s)))
            .collect();
        let t = trailer_of(&records);
        assert_eq!(t.total_base_records, 23);
        let raw = t.encode().unwrap();
        for spec in TrailerRecord::FIELDS
            .iter()
            .filter(|spec| spec.name.starts_with("total_status_code_"))
        {
            assert_eq!(&raw[spec.range()], "000000001", "{}", spec.name);
        }
    }

    #[test]
    fn j1_and_j2_counts() {
        let t = trailer_of(&[
            DataRecord {
                j1: vec![j1(), j1()],
                ..DataRecord::new(base())
            },
            DataRecord {
                j1: vec![j1()],
                j2: vec![j2()],
                ..DataRecord::new(base())
            },
        ]);
        assert_eq!(t.total_j1_segments, 3);
        assert_eq!(t.total_j2_segments, 1);
    }

    #[test]
    fn k_l_n_counts() {
        let t = trailer_of(&[all_segments()]);
        assert_eq!(
            [
                t.total_k1_segments,
                t.total_k2_segments,
                t.total_k3_segments,
                t.total_k4_segments,
                t.total_l1_segments,
                t.total_n1_segments
            ],
            [1; 6]
        );
    }

    #[test]
    fn ssn_counts_base() {
        let t = trailer_of(&[
            DataRecord::new(BaseSegment {
                social_security_number: 123_456_789,
                ..base()
            }),
            DataRecord::new(BaseSegment {
                social_security_number: 0,
                ..base()
            }),
            DataRecord::new(BaseSegment {
                social_security_number: 999_999_999,
                ..base()
            }),
        ]);
        assert_eq!(t.total_ssns_base_segments, 1);
        assert_eq!(t.total_ssns_all_segments, 1);
    }

    #[test]
    fn ssn_counts_j1_j2() {
        let t = trailer_of(&[DataRecord {
            base: BaseSegment {
                social_security_number: 123_456_789,
                ..base()
            },
            j1: vec![J1Segment {
                social_security_number: 987_654_321,
                ..j1()
            }],
            j2: vec![J2Segment {
                social_security_number: 111_223_333,
                ..j2()
            }],
            ..DataRecord::new(base())
        }]);
        assert_eq!(
            [
                t.total_ssns_base_segments,
                t.total_ssns_j1_segments,
                t.total_ssns_j2_segments,
                t.total_ssns_all_segments
            ],
            [1, 1, 1, 3]
        );
    }

    #[test]
    fn dob_counts() {
        let t = trailer_of(&[
            DataRecord {
                base: BaseSegment {
                    date_of_birth: Some(date(1990, 1, 15)),
                    ..base()
                },
                j1: vec![J1Segment {
                    date_of_birth: Some(date(1985, 3, 20)),
                    ..j1()
                }],
                j2: vec![J2Segment {
                    date_of_birth: Some(date(1988, 7, 10)),
                    ..j2()
                }],
                ..DataRecord::new(base())
            },
            DataRecord::new(base()),
        ]);
        assert_eq!(
            [
                t.total_dobs_base_segments,
                t.total_dobs_j1_segments,
                t.total_dobs_j2_segments,
                t.total_dobs_all_segments
            ],
            [1, 1, 1, 3]
        );
    }

    #[test]
    fn telephone_counts() {
        let t = trailer_of(&[
            DataRecord {
                base: BaseSegment {
                    telephone_number: 5_551_234_567,
                    ..base()
                },
                j1: vec![J1Segment {
                    telephone_number: 5_559_876_543,
                    ..j1()
                }],
                ..DataRecord::new(base())
            },
            DataRecord::new(base()),
        ]);
        assert_eq!(t.total_telephone_numbers, 2);
    }

    #[test]
    fn ecoa_z_counts() {
        let t = trailer_of(&[
            DataRecord {
                base: BaseSegment {
                    ecoa_code: EcoaCode::Delete,
                    ..base()
                },
                j1: vec![J1Segment {
                    ecoa_code: EcoaCode::Delete,
                    ..j1()
                }],
                ..DataRecord::new(base())
            },
            DataRecord {
                j2: vec![J2Segment {
                    ecoa_code: EcoaCode::Delete,
                    ..j2()
                }],
                ..DataRecord::new(base())
            },
        ]);
        assert_eq!(t.total_ecoa_code_z, 3);
    }

    #[test]
    fn multiple_records_accumulate() {
        let records: Vec<_> = (0..5)
            .map(|i| {
                DataRecord::new(BaseSegment {
                    social_security_number: 100_000_000 + i,
                    date_of_birth: Some(date(1990, 1, 1)),
                    telephone_number: 5_550_000_000 + u64::from(i),
                    ..base()
                })
            })
            .collect();
        let t = trailer_of(&records);
        assert_eq!(t.total_base_records, 5);
        assert_eq!(t.total_status_code_11, 5);
        assert_eq!(t.total_ssns_all_segments, 5);
        assert_eq!(t.total_dobs_all_segments, 5);
        assert_eq!(t.total_telephone_numbers, 5);
        assert_eq!(t.block_count, 7);
    }

    #[test]
    fn from_records_matches_writer() {
        let records = vec![all_segments(), DataRecord::new(base())];
        assert_eq!(TrailerRecord::from_records(&records), trailer_of(&records));
    }
}

mod validation {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn invalid_base_rejected_and_not_written() {
        let mut writer = Writer::new(Vec::new());
        let mut file = writer.begin_file(&header()).unwrap();
        let bad = DataRecord::new(BaseSegment {
            payment_rating: None,
            ..base()
        });
        let err = file.write(&bad).unwrap_err();
        let Error::Invalid {
            account,
            violations,
        } = &err
        else {
            panic!("expected Invalid, got {err:?}");
        };
        assert_eq!(account, "ACCT-000001");
        assert!(matches!(violations[..], [Violation::PaymentRating { .. }]));
        assert!(
            err.to_string()
                .starts_with("account ACCT-000001: Account status '11'")
        );
        assert_eq!(file.trailer().total_base_records, 0);
        file.finish().unwrap();
        assert_eq!(writer.into_inner().len(), 426 * 2);
    }

    #[test]
    fn validation_can_be_disabled() {
        let mut writer = Writer::new(Vec::new()).validate(false);
        let mut file = writer.begin_file(&header()).unwrap();
        file.write(&DataRecord::new(BaseSegment {
            payment_rating: None,
            ..base()
        }))
        .unwrap();
        assert_eq!(file.finish().unwrap().total_base_records, 1);
    }

    #[test]
    fn encoding_errors_write_nothing() {
        let mut writer = Writer::new(Vec::new());
        let mut file = writer.begin_file(&header()).unwrap();
        let bad = DataRecord::new(BaseSegment {
            surname: "X".repeat(26),
            ..base()
        });
        assert!(matches!(file.write(&bad), Err(Error::Record(_))));
        file.finish().unwrap();
        assert_eq!(writer.into_inner().len(), 426 * 2);
    }
}

mod newline {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn newline_between_records() {
        let output = write_file(&[DataRecord::new(base())], true);
        let lines: Vec<_> = output.split('\n').collect();
        assert_eq!(lines.len(), 4);
        assert_eq!(&lines[0][..10], "0426HEADER");
        assert_eq!(&lines[2][4..11], "TRAILER");
        assert_eq!(lines[3], "");
    }

    #[test]
    fn data_record_is_one_line() {
        let record = DataRecord {
            j1: vec![j1()],
            ..DataRecord::new(base())
        };
        let output = write_file(&[record], true);
        assert_eq!(output.split('\n').nth(1).unwrap().len(), 526);
    }

    #[test]
    fn no_newline_by_default() {
        assert!(!write_file(&[DataRecord::new(base())], false).contains('\n'));
    }
}
