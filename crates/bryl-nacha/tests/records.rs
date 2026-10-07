//! Records (`records`), code tables and transaction code selection
//! (`codes`), routing numbers and file ID modifiers (`routing`), and entries
//! with addenda (`entry`). Routing numbers in these tests pass the ABA
//! checksum (091000019, 021000021, 011000015).

mod common;

use bryl::Record;
use chrono::NaiveDate;
use common::*;
use nacha::{
    AccountKind, Addendum, BatchControl, BatchHeader, Entry, EntryDetail, FileControl, FileHeader,
    FileIdModifier, NachaRecord, ReturnAddendum, ReturnReasonCode, RoutingNumber,
    RoutingNumberError, ServiceClassCode, StandardEntryClass, TransactionCode,
};

fn file_header() -> FileHeader {
    FileHeader::builder()
        .immediate_destination(routing())
        .immediate_origin("9876543210")
        .file_creation_date(sample_date())
        .file_creation_time(sample_time())
        .file_id_modifier(FileIdModifier::new('A').unwrap())
        .immediate_destination_name("DEST BANK")
        .immediate_origin_name("ORIGIN BANK")
        .build()
}

fn batch_header() -> BatchHeader {
    BatchHeader::builder()
        .service_class_code(ServiceClassCode::MixedDebitsAndCredits)
        .company_name("ACME CORP")
        .company_id("1234567890")
        .standard_entry_class(StandardEntryClass::Ppd)
        .company_entry_description("PAYROLL")
        .effective_entry_date(sample_date())
        .originating_dfi_id(12_345_678)
        .batch_number(1)
        .build()
}

fn batch_control() -> BatchControl {
    BatchControl::builder()
        .service_class_code(ServiceClassCode::MixedDebitsAndCredits)
        .entry_addenda_count(5)
        .entry_hash(1_234_567_890)
        .total_debit_amount(50_000)
        .total_credit_amount(50_000)
        .company_id("1234567890")
        .originating_dfi_id(12_345_678)
        .batch_number(1)
        .build()
}

fn file_control() -> FileControl {
    FileControl::builder()
        .batch_count(2)
        .block_count(3)
        .entry_addenda_count(10)
        .entry_hash(1_234_567_890)
        .total_debit_amount(5000)
        .total_credit_amount(15_000)
        .build()
}

fn addendum(info: &str, sequence: u16) -> Addendum {
    Addendum::builder()
        .payment_related_information(info)
        .addenda_sequence_number(sequence)
        .entry_detail_sequence_number(1_234_567)
        .build()
}

fn return_addendum(date_of_death: Option<NaiveDate>) -> ReturnAddendum {
    ReturnAddendum::builder()
        .return_reason_code(ReturnReasonCode::AccountHolderDeceased)
        .original_entry_trace_number(91_000_010_000_001)
        .maybe_date_of_death(date_of_death)
        .original_receiving_dfi_id(9_100_001)
        .addenda_information("DECEASED")
        .trace_number(123_456_789_012_345)
        .build()
}

mod records {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn record_type_constants() {
        assert_eq!(FileHeader::RECORD_TYPE, "1");
        assert_eq!(BatchHeader::RECORD_TYPE, "5");
        assert_eq!(EntryDetail::RECORD_TYPE, "6");
        assert_eq!(Addendum::RECORD_TYPE, "7");
        assert_eq!(BatchControl::RECORD_TYPE, "8");
        assert_eq!(FileControl::RECORD_TYPE, "9");
        assert_eq!(Addendum::ADDENDA_TYPE_CODE, 5);
    }

    #[test]
    fn file_header_constants() {
        assert_eq!(FileHeader::PRIORITY_CODE, 1);
        assert_eq!(FileHeader::RECORD_SIZE, 94);
        assert_eq!(FileHeader::BLOCKING_FACTOR, 10);
        assert_eq!(FileHeader::FORMAT_CODE, 1);
    }

    #[test]
    fn all_records_are_94_characters() {
        assert_eq!(file_header().encode().unwrap().len(), 94);
        assert_eq!(batch_header().encode().unwrap().len(), 94);
        assert_eq!(
            entry_detail(TransactionCode::CheckingCredit)
                .encode()
                .unwrap()
                .len(),
            94
        );
        assert_eq!(addendum("PAYMENT INFO", 1).encode().unwrap().len(), 94);
        assert_eq!(batch_control().encode().unwrap().len(), 94);
        assert_eq!(file_control().encode().unwrap().len(), 94);
    }

    #[test]
    fn file_header_layout() {
        assert_eq!(
            file_header().encode().unwrap(),
            "101 09100001998765432102306151430A094101DEST BANK              ORIGIN BANK                    "
        );
    }

    #[test]
    fn file_header_roundtrip() {
        let loaded = FileHeader::decode(file_header().encode().unwrap().as_bytes()).unwrap();
        assert_eq!(loaded, file_header());
        assert_eq!(loaded.immediate_destination, routing());
        assert_eq!(loaded.file_creation_date, sample_date());
    }

    #[test]
    fn file_creation() {
        assert_eq!(file_header().file_creation(), sample_datetime());
    }

    #[test]
    fn immediate_origin_is_right_aligned() {
        // Right-aligned and space-padded, like the immediate destination.
        let header = FileHeader {
            immediate_origin: "091000019".into(),
            ..file_header()
        };
        assert_eq!(&header.encode().unwrap()[13..23], " 091000019");
    }

    #[test]
    fn batch_header_roundtrip() {
        let loaded = BatchHeader::decode(batch_header().encode().unwrap().as_bytes()).unwrap();
        assert_eq!(loaded, batch_header());
        assert_eq!(loaded.company_name, "ACME CORP");
        assert_eq!(
            loaded.service_class_code,
            ServiceClassCode::MixedDebitsAndCredits
        );
    }

    #[test]
    fn batch_header_codes() {
        let header = BatchHeader {
            service_class_code: ServiceClassCode::CreditsOnly,
            standard_entry_class: StandardEntryClass::Web,
            ..batch_header()
        };
        let encoded = header.encode().unwrap();
        assert_eq!(&encoded[1..4], "220");
        assert_eq!(&encoded[50..53], "WEB");
    }

    #[test]
    fn entry_detail_roundtrip() {
        let ed = entry_detail(TransactionCode::CheckingCredit);
        let loaded = EntryDetail::decode(ed.encode().unwrap().as_bytes()).unwrap();
        assert_eq!(loaded, ed);
        assert_eq!(loaded.amount, 10_000);
        assert_eq!(loaded.individual_name, "JOHN DOE");
    }

    #[test]
    fn entry_detail_transaction_code_field() {
        let ed = entry_detail(TransactionCode::SavingsDebit);
        assert_eq!(&ed.encode().unwrap()[1..3], "37");
    }

    #[test]
    fn entry_detail_account_number_is_left_aligned() {
        // Left-aligned and space-padded.
        let encoded = entry_detail(TransactionCode::CheckingCredit)
            .encode()
            .unwrap();
        assert_eq!(&encoded[12..29], "9876543210       ");
    }

    #[test]
    fn entry_detail_mask() {
        let masked = entry_detail(TransactionCode::CheckingCredit).mask();
        assert_eq!(masked.receiving_dfi_account_number, "X".repeat(17));
        // The original is untouched.
        assert_eq!(
            entry_detail(TransactionCode::CheckingCredit).receiving_dfi_account_number,
            "9876543210"
        );
    }

    #[test]
    fn entry_detail_sequence_number() {
        assert_eq!(
            entry_detail(TransactionCode::CheckingCredit).sequence_number(),
            9_012_345
        );
    }

    #[test]
    fn addenda_indicator_must_be_0_or_1() {
        let ed = EntryDetail {
            addenda_record_indicator: 2,
            ..entry_detail(TransactionCode::CheckingCredit)
        };
        assert!(ed.encode().is_err());
    }

    #[test]
    fn addendum_roundtrip() {
        let a = addendum("PAYMENT INFO", 1);
        let loaded = Addendum::decode(a.encode().unwrap().as_bytes()).unwrap();
        assert_eq!(loaded.payment_related_information, "PAYMENT INFO");
        assert_eq!(loaded, a);
    }

    #[test]
    fn batch_control_roundtrip() {
        let loaded = BatchControl::decode(batch_control().encode().unwrap().as_bytes()).unwrap();
        assert_eq!(loaded.entry_addenda_count, 5);
        assert_eq!(loaded.entry_hash, 1_234_567_890);
        assert_eq!(loaded, batch_control());
    }

    #[test]
    fn file_control_roundtrip() {
        let loaded = FileControl::decode(file_control().encode().unwrap().as_bytes()).unwrap();
        assert_eq!(loaded.batch_count, 2);
        assert_eq!(loaded.entry_addenda_count, 10);
        assert_eq!(loaded, file_control());
    }

    #[test]
    fn text_is_uppercased() {
        let header = BatchHeader {
            company_name: "acme corp".into(),
            ..batch_header()
        };
        assert_eq!(&header.encode().unwrap()[4..13], "ACME CORP");
    }

    #[test]
    fn return_addendum_layout() {
        let line = return_addendum(None).encode().unwrap();
        assert_eq!(line.len(), 94);
        assert_eq!(&line[..6], "799R15");
        assert_eq!(&line[6..21], "091000010000001");
        assert_eq!(&line[21..27], "      "); // no date of death: blank
        assert_eq!(&line[27..35], "09100001");
        assert_eq!(line[35..79].trim_end(), "DECEASED");
        assert_eq!(&line[79..], "123456789012345");
    }

    #[test]
    fn return_addendum_roundtrip() {
        for date in [None, NaiveDate::from_ymd_opt(2024, 3, 9)] {
            let addendum = return_addendum(date);
            let line = addendum.encode().unwrap();
            if date.is_some() {
                assert_eq!(&line[21..27], "240309");
            }
            let loaded = ReturnAddendum::decode_exact(line.as_bytes()).unwrap();
            assert_eq!(loaded.addenda_information, "DECEASED");
            assert_eq!(loaded.date_of_death, date);
            assert_eq!(
                loaded.return_reason_code,
                ReturnReasonCode::AccountHolderDeceased
            );
        }
    }

    #[test]
    fn unsupported_return_reason_is_rejected() {
        // R61 (misrouted return) is a dishonored return, with another layout.
        let line = return_addendum(None)
            .encode()
            .unwrap()
            .replacen("R15", "R61", 1);
        assert!(ReturnAddendum::decode_exact(line.as_bytes()).is_err());
    }

    #[test]
    fn dispatch_by_addenda_type() {
        use bryl::read::Dispatch;
        let line = return_addendum(None).encode().unwrap();
        assert_eq!(
            NachaRecord::dispatch(line.as_bytes()).unwrap(),
            NachaRecord::ReturnAddendum(return_addendum(None))
        );
        let line = addendum("INFO", 1).encode().unwrap();
        assert_eq!(
            NachaRecord::dispatch(line.as_bytes()).unwrap(),
            NachaRecord::Addendum(addendum("INFO", 1))
        );
        // Notification of change addenda (type 98) are not supported.
        let line = line.replacen("705", "798", 1);
        assert!(NachaRecord::dispatch(line.as_bytes()).is_err());
    }

    #[test]
    fn dispatch_by_record_type() {
        use bryl::read::Dispatch;
        let line = batch_header().encode().unwrap();
        assert_eq!(
            NachaRecord::dispatch(line.as_bytes()).unwrap(),
            NachaRecord::BatchHeader(batch_header())
        );
        assert_eq!(
            NachaRecord::dispatch(&nacha::FILLER).unwrap(),
            NachaRecord::Filler
        );
        assert!(NachaRecord::dispatch(format!("X{}", " ".repeat(93)).as_bytes()).is_err());
        // Lines must be exactly 94 characters.
        assert!(NachaRecord::dispatch(format!("{line} ").as_bytes()).is_err());
        assert!(NachaRecord::dispatch(&line.as_bytes()[..93]).is_err());
    }
}

mod codes {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn code_values() {
        assert_eq!(ServiceClassCode::MixedDebitsAndCredits.as_code(), 200);
        assert_eq!(ServiceClassCode::CreditsOnly.as_code(), 220);
        assert_eq!(ServiceClassCode::DebitsOnly.as_code(), 225);
        assert_eq!(TransactionCode::SavingsDebit.as_code(), 37);
        assert_eq!(StandardEntryClass::Ppd.as_code(), "PPD");
    }

    #[test]
    fn code_table_sizes() {
        assert_eq!(ServiceClassCode::ALL.len(), 3);
        assert_eq!(StandardEntryClass::ALL.len(), 24);
        assert_eq!(TransactionCode::ALL.len(), 12);
    }

    #[test]
    fn parse_codes() {
        assert_eq!("WEB".parse(), Ok(StandardEntryClass::Web));
        assert_eq!(
            ServiceClassCode::try_from(220),
            Ok(ServiceClassCode::CreditsOnly)
        );
        assert!(TransactionCode::try_from(24).is_err());
    }

    fn for_entry(amount: i64, account: AccountKind, ret: bool, prenote: bool) -> TransactionCode {
        TransactionCode::for_entry(amount, account, ret, prenote)
    }

    #[test]
    fn for_entry_credit_and_debit() {
        use AccountKind::{Checking, Savings};
        assert_eq!(
            for_entry(100, Checking, false, false),
            TransactionCode::CheckingCredit
        );
        assert_eq!(
            for_entry(-100, Checking, false, false),
            TransactionCode::CheckingDebit
        );
        assert_eq!(
            for_entry(100, Savings, false, false),
            TransactionCode::SavingsCredit
        );
        assert_eq!(
            for_entry(-100, Savings, false, false),
            TransactionCode::SavingsDebit
        );
    }

    #[test]
    fn for_entry_prenote() {
        use AccountKind::{Checking, Savings};
        assert_eq!(
            for_entry(100, Checking, false, true),
            TransactionCode::CheckingPrenoteCredit
        );
        assert_eq!(
            for_entry(100, Savings, false, true),
            TransactionCode::SavingsPrenoteCredit
        );
        assert_eq!(
            for_entry(-100, Savings, false, true),
            TransactionCode::SavingsPrenoteDebit
        );
    }

    #[test]
    fn for_entry_zero_amount_is_prenote() {
        assert_eq!(
            for_entry(0, AccountKind::Checking, false, false),
            TransactionCode::CheckingPrenoteCredit
        );
    }

    #[test]
    fn for_entry_return() {
        use AccountKind::{Checking, Savings};
        assert_eq!(
            for_entry(100, Checking, true, false),
            TransactionCode::CheckingReturnedCredit
        );
        assert_eq!(
            for_entry(100, Savings, true, false),
            TransactionCode::SavingsReturnedCredit
        );
        assert_eq!(
            for_entry(-100, Checking, true, true),
            TransactionCode::CheckingReturnedDebit
        );
        // A return takes precedence over a zero amount.
        assert_eq!(
            for_entry(0, Checking, true, false),
            TransactionCode::CheckingReturnedCredit
        );
    }

    #[test]
    fn for_entry_matches_code_arithmetic() {
        // Codes are 21/23/22 (return, prenote, credit), +5 for debits and
        // +10 for savings accounts.
        for account in [AccountKind::Checking, AccountKind::Savings] {
            for amount in [-5i64, 0, 5] {
                for ret in [false, true] {
                    for prenote in [false, true] {
                        let mut expected = if ret {
                            21
                        } else if prenote || amount == 0 {
                            23
                        } else {
                            22
                        };
                        if amount < 0 {
                            expected += 5;
                        }
                        if account == AccountKind::Savings {
                            expected += 10;
                        }
                        assert_eq!(
                            for_entry(amount, account, ret, prenote).as_code(),
                            expected,
                            "{amount} {account:?} {ret} {prenote}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn predicates_account() {
        assert!(TransactionCode::CheckingCredit.is_checking());
        assert!(!TransactionCode::CheckingCredit.is_savings());
        assert!(TransactionCode::SavingsCredit.is_savings());
        assert!(!TransactionCode::SavingsCredit.is_checking());
    }

    #[test]
    fn predicates_direction() {
        assert!(TransactionCode::CheckingCredit.is_credit());
        assert!(!TransactionCode::CheckingCredit.is_debit());
        assert!(TransactionCode::CheckingDebit.is_debit());
        assert!(!TransactionCode::CheckingDebit.is_credit());
    }

    #[test]
    fn predicates_prenote_and_return() {
        assert!(TransactionCode::CheckingPrenoteCredit.is_prenote());
        assert!(TransactionCode::CheckingPrenoteDebit.is_prenote());
        assert!(TransactionCode::CheckingReturnedCredit.is_return());
        assert!(!TransactionCode::CheckingCredit.is_return());
    }

    #[test]
    fn service_class_allows() {
        use ServiceClassCode::{CreditsOnly, DebitsOnly, MixedDebitsAndCredits};
        for code in TransactionCode::ALL.iter().copied() {
            assert!(MixedDebitsAndCredits.allows(code));
            assert_eq!(CreditsOnly.allows(code), code.is_credit(), "{code:?}");
            assert_eq!(DebitsOnly.allows(code), code.is_debit(), "{code:?}");
        }
    }

    #[test]
    fn direction_and_description_rules() {
        use StandardEntryClass as Sec;
        let debits_only: Vec<_> = Sec::ALL
            .iter()
            .filter(|c| c.debits_only())
            .copied()
            .collect();
        assert_eq!(
            debits_only,
            [Sec::Arc, Sec::Boc, Sec::Pop, Sec::Rck, Sec::Tel, Sec::Trc]
        );
        let credits_only: Vec<_> = Sec::ALL
            .iter()
            .filter(|c| c.credits_only())
            .copied()
            .collect();
        assert_eq!(credits_only, [Sec::Cie]);
        assert_eq!(Sec::Enr.required_entry_description(), Some("AUTOENROLL"));
        assert_eq!(Sec::Rck.required_entry_description(), Some("REDEPCHECK"));
        assert_eq!(Sec::Ppd.required_entry_description(), None);
        assert_eq!("BOC".parse(), Ok(Sec::Boc));
    }

    #[test]
    fn addenda_limits() {
        use StandardEntryClass as Sec;
        let limits = |classes: &[Sec]| -> Vec<(u16, u16)> {
            classes
                .iter()
                .map(|c| (c.min_addenda(), c.max_addenda()))
                .collect()
        };
        let none = [
            Sec::Arc,
            Sec::Boc,
            Sec::Pop,
            Sec::Rck,
            Sec::Trc,
            Sec::Xck,
            Sec::Tel,
            Sec::Adv,
            Sec::Cor,
            Sec::Mte,
            Sec::Pos,
            Sec::Shr,
        ];
        assert_eq!(limits(&none), vec![(0, 0); none.len()]);
        let many = [Sec::Ctx, Sec::Atx, Sec::Trx];
        assert_eq!(limits(&many), vec![(0, 9999); many.len()]);
        let exactly_one = [Sec::Dne, Sec::Enr];
        assert_eq!(limits(&exactly_one), vec![(1, 1); exactly_one.len()]);
        let up_to_one = [
            Sec::Ppd,
            Sec::Ccd,
            Sec::Web,
            Sec::Cie,
            Sec::Ack,
            Sec::Pbr,
            Sec::Cbr,
        ];
        assert_eq!(limits(&up_to_one), vec![(0, 1); up_to_one.len()]);
        assert_eq!(
            none.len() + many.len() + exactly_one.len() + up_to_one.len(),
            Sec::ALL.len()
        );
    }

    #[test]
    fn date_of_death_only_for_r14_and_r15() {
        let allowed: Vec<_> = ReturnReasonCode::ALL
            .iter()
            .filter(|code| code.allows_date_of_death())
            .map(ToString::to_string)
            .collect();
        assert_eq!(allowed, ["R14", "R15"]);
    }

    #[test]
    fn return_reason_codes() {
        assert_eq!(ReturnReasonCode::InsufficientFunds.as_code(), "R01");
        assert_eq!(
            "R90".parse::<ReturnReasonCode>(),
            Ok(ReturnReasonCode::SanctionsCompliance)
        );
        assert!("R48".parse::<ReturnReasonCode>().is_err());
    }
}

mod routing {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn valid_numbers() {
        for value in ["091000019", "021000021", "011000015"] {
            let routing: RoutingNumber = value.parse().unwrap();
            assert_eq!(routing.to_string(), value);
        }
    }

    #[test]
    fn parts() {
        let routing = routing();
        assert_eq!(routing.get(), 91_000_019);
        assert_eq!(routing.trn(), 9_100_001);
        assert_eq!(routing.check_digit(), 9);
        assert_eq!(RoutingNumber::from_parts(9_100_001, 9), Ok(routing));
    }

    #[test]
    fn bad_check_digit() {
        assert_eq!(
            "123456789".parse::<RoutingNumber>(),
            Err(RoutingNumberError::Checksum(123_456_789))
        );
        assert!(RoutingNumber::from_parts(9_100_001, 8).is_err());
    }

    #[test]
    fn bad_format() {
        for value in ["1234", "0910000190", "09100001X", ""] {
            assert!(
                matches!(
                    value.parse::<RoutingNumber>(),
                    Err(RoutingNumberError::Format(_))
                ),
                "{value}"
            );
        }
        assert!(RoutingNumber::new(1_000_000_000).is_err());
        assert!(RoutingNumber::from_parts(9_100_001, 10).is_err());
    }

    #[test]
    fn invalid_routing_number_rejected_on_decode() {
        let mut line = entry_detail(TransactionCode::CheckingCredit)
            .encode()
            .unwrap();
        line.replace_range(3..12, "123456789");
        let err = EntryDetail::decode(line.as_bytes()).unwrap_err();
        assert!(err.to_string().contains("invalid check digit"), "{err}");
    }

    #[test]
    fn file_id_modifier() {
        assert_eq!(FileIdModifier::default().get(), 'A');
        assert!(FileIdModifier::new('7').is_ok());
        assert!(FileIdModifier::new('a').is_err());
        assert!(FileIdModifier::new('-').is_err());
    }
}

mod entry {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn is_rejection() {
        let entry = Entry {
            detail: entry_detail(TransactionCode::CheckingReturnedCredit),
            addenda: vec![],
            return_addendum: None,
        };
        assert!(entry.is_rejection());
    }

    #[test]
    fn not_rejection() {
        let entry = Entry {
            detail: entry_detail(TransactionCode::CheckingCredit),
            addenda: vec![],
            return_addendum: None,
        };
        assert!(!entry.is_rejection());
    }

    #[test]
    fn mask_delegates() {
        let entry = Entry {
            detail: entry_detail(TransactionCode::CheckingCredit),
            addenda: vec![],
            return_addendum: None,
        };
        assert_eq!(
            entry.mask().detail.receiving_dfi_account_number,
            "X".repeat(17)
        );
    }

    #[test]
    fn encode_decode_with_addenda() {
        let entry = Entry {
            detail: EntryDetail {
                addenda_record_indicator: 1,
                ..entry_detail(TransactionCode::CheckingCredit)
            },
            addenda: vec![addendum("ADD INFO", 1)],
            return_addendum: None,
        };
        let encoded = entry.encode().unwrap();
        assert_eq!(encoded.len(), 94 * 2 + 1);
        let loaded = Entry::decode(encoded.as_bytes()).unwrap();
        assert_eq!(loaded.addenda.len(), 1);
        assert_eq!(loaded.addenda[0].payment_related_information, "ADD INFO");
        assert_eq!(loaded, entry);
    }

    #[test]
    fn encode_decode_return() {
        let entry = Entry {
            detail: EntryDetail {
                addenda_record_indicator: 1,
                ..entry_detail(TransactionCode::CheckingReturnedDebit)
            },
            addenda: vec![],
            return_addendum: Some(return_addendum(None)),
        };
        assert_eq!(entry.addenda_count(), 1);
        let encoded = entry.encode().unwrap();
        assert_eq!(encoded.len(), 94 * 2 + 1);
        assert_eq!(Entry::decode(encoded.as_bytes()).unwrap(), entry);
    }

    #[test]
    fn encode_without_addenda() {
        let entry = Entry {
            detail: entry_detail(TransactionCode::CheckingCredit),
            addenda: vec![],
            return_addendum: None,
        };
        assert_eq!(entry.encode().unwrap().len(), 94);
    }
}
