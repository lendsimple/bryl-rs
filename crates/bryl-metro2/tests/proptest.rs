//! Property tests: any valid data records the writer accepts read back
//! unchanged, the written trailer matches `TrailerRecord::from_records`, and
//! arbitrary input never panics the reader.

use chrono::NaiveDate;
use metro2::{
    AccountStatus, AccountType, BaseSegment, DataRecord, EcoaCode, File, HeaderRecord, J1Segment,
    K1Segment, N1Segment, PortfolioType, TrailerRecord, Writer,
};
use proptest::prelude::*;

/// Free text for fields without character rules (K1/N1 names).
fn text(max: usize) -> impl Strategy<Value = String> {
    pattern(&format!(
        "[A-Z0-9][A-Z0-9 ,.&-]{{0,{}}}[A-Z0-9]|[A-Z0-9]",
        max - 2
    ))
}

/// Letters, spaces and hyphens (CRRG name fields).
fn name(max: usize) -> impl Strategy<Value = String> {
    pattern(&format!("[A-Z][A-Z -]{{0,{}}}[A-Z]|[A-Z]", max - 2))
}

/// Letters, digits, spaces, slashes, dashes and periods (CRRG address fields).
fn address(max: usize) -> impl Strategy<Value = String> {
    pattern(&format!(
        "[A-Z0-9][A-Z0-9 ./-]{{0,{}}}[A-Z0-9]|[A-Z0-9]",
        max - 2
    ))
}

/// Letters and digits (CRRG account and identification numbers).
fn identifier(max: usize) -> impl Strategy<Value = String> {
    pattern(&format!("[A-Z0-9]{{1,{max}}}"))
}

fn pattern(regex: &str) -> impl Strategy<Value = String> + use<> {
    proptest::string::string_regex(regex).unwrap()
}

fn date() -> impl Strategy<Value = NaiveDate> {
    (1900i32..=2099, 1u32..=366)
        .prop_filter_map("valid date", |(y, o)| NaiveDate::from_yo_opt(y, o))
}

fn base() -> impl Strategy<Value = BaseSegment> {
    (
        // Status 05 is retired and rejected by the writer.
        proptest::sample::select(
            AccountStatus::ALL
                .iter()
                .copied()
                .filter(|&s| s != AccountStatus::Transferred)
                .collect::<Vec<_>>(),
        ),
        proptest::sample::select(AccountType::ALL),
        proptest::sample::select(PortfolioType::ALL),
        proptest::sample::select(EcoaCode::ALL),
        (
            date(),
            date(),
            proptest::option::of(date()),
            proptest::option::of(date()),
        ),
        (0u32..=999_999_999, 0u32..=999_999_999, 0u32..=999_999_999),
        (0u32..=999_999_999, 0u64..=9_999_999_999),
        (identifier(30), name(25), name(20), address(32)),
    )
        .prop_map(
            |(status, account_type, portfolio, ecoa, dates, amounts, ids, names)| {
                let (opened, as_of, closed, dob) = dates;
                let (balance, past_due, limit) = amounts;
                let (ssn, phone) = ids;
                let (account, surname, first, address) = names;
                let past_due =
                    if matches!(status, AccountStatus::Current | AccountStatus::PaidOrClosed) {
                        0
                    } else {
                        past_due
                    };
                BaseSegment::builder()
                    .identification_number("FURNISHER")
                    .consumer_account_number(account)
                    .portfolio_type(portfolio)
                    .account_type(account_type)
                    .date_opened(opened)
                    .credit_limit(limit)
                    .account_status(status)
                    .maybe_payment_rating(
                        status
                            .requires_payment_rating()
                            .then_some(metro2::PaymentRating::Current),
                    )
                    .current_balance(balance)
                    .amount_past_due(past_due)
                    .date_of_account_information(as_of)
                    .maybe_date_closed(closed)
                    .surname(surname)
                    .first_name(first)
                    .social_security_number(ssn)
                    .maybe_date_of_birth(dob)
                    .telephone_number(phone)
                    .ecoa_code(ecoa)
                    .first_line_of_address(address)
                    .city("CITY")
                    .state("CA")
                    .zip_code("90210")
                    .build()
            },
        )
}

fn data_record() -> impl Strategy<Value = DataRecord> {
    (
        base(),
        proptest::collection::vec(
            (name(25), proptest::option::of(date()), 0u32..=999_999_999),
            0..3,
        ),
        proptest::option::of(text(30)),
        proptest::option::of(text(30)),
    )
        .prop_map(|(base, j1s, k1, n1)| DataRecord {
            j1: j1s
                .into_iter()
                .map(|(surname, dob, ssn)| {
                    J1Segment::builder()
                        .surname(surname)
                        .first_name("J")
                        .ecoa_code(EcoaCode::Joint)
                        .maybe_date_of_birth(dob)
                        .social_security_number(ssn)
                        .build()
                })
                .collect(),
            k1: k1.map(|name| {
                K1Segment::builder()
                    .original_creditor_name(name)
                    .creditor_classification(metro2::CreditorClassification::Financial)
                    .build()
            }),
            n1: n1.map(|name| N1Segment::builder().employer_name(name).build()),
            ..DataRecord::new(base)
        })
}

fn write(records: &[DataRecord], newline: bool) -> (String, TrailerRecord) {
    let day = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
    let header = HeaderRecord::builder()
        .activity_date(day)
        .date_created(day)
        .reporter_name("REPORTER")
        .reporter_address("ADDRESS")
        .reporter_telephone_number(5_551_234_567)
        .build();
    let mut writer = Writer::new(Vec::new()).newline(newline);
    let mut file = writer.begin_file(&header).unwrap();
    for record in records {
        file.write(record).unwrap();
    }
    let trailer = file.finish().unwrap();
    (String::from_utf8(writer.into_inner()).unwrap(), trailer)
}

/// What reading gives back: the base segment carries the written RDW.
fn as_read(record: &DataRecord) -> DataRecord {
    DataRecord {
        base: BaseSegment {
            record_descriptor_word: u16::try_from(record.len()).unwrap(),
            ..record.base.clone()
        },
        ..record.clone()
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn written_files_read_back_and_validate(
        records in proptest::collection::vec(data_record(), 0..6),
        newline: bool,
    ) {
        let (output, trailer) = write(&records, newline);
        let file = metro2::Reader::new(output.as_bytes()).newline(newline).read_file().unwrap();
        prop_assert_eq!(file.validate(), vec![]);
        prop_assert_eq!(&file.trailer, &trailer);
        prop_assert_eq!(trailer, TrailerRecord::from_records(&records));
        let expected: Vec<_> = records.iter().map(as_read).collect();
        prop_assert_eq!(file.data_records, expected);
    }

    #[test]
    fn arbitrary_input_never_panics(input in proptest::collection::vec(any::<u8>(), 0..2000)) {
        let _ = File::read(input.as_slice());
        let _ = metro2::Reader::new(input.as_slice()).newline(true).read_file();
    }

    #[test]
    fn mutated_files_never_panic(
        records in proptest::collection::vec(data_record(), 1..3),
        position in any::<proptest::sample::Index>(),
        byte in any::<u8>(),
    ) {
        let (output, _) = write(&records, false);
        let mut bytes = output.into_bytes();
        let index = position.index(bytes.len());
        bytes[index] = byte;
        if let Ok(file) = File::read(bytes.as_slice()) {
            let _ = file.validate();
        }
    }
}
