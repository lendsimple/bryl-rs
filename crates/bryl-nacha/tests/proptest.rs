//! Property tests: any valid file the writer produces reads back to the same
//! entries and validates cleanly, and arbitrary input never panics the reader.

use chrono::NaiveDate;
use nacha::{
    BatchParams, EntryParams, File, FileParams, ServiceClassCode, StandardEntryClass,
    TransactionCode, Writer,
};
use proptest::prelude::*;

const ROUTING_NUMBERS: [&str; 4] = ["091000019", "021000021", "011000015", "026009593"];

#[derive(Debug, Clone)]
struct BatchSpec {
    service_class_code: ServiceClassCode,
    standard_entry_class: StandardEntryClass,
    entries: Vec<EntryParams>,
}

fn text(max: usize) -> impl Strategy<Value = String> {
    proptest::string::string_regex(&format!(
        "[A-Z0-9][A-Z0-9 ]{{0,{}}}[A-Z0-9]|[A-Z0-9]",
        max - 2
    ))
    .unwrap()
}

fn entry(
    service_class: ServiceClassCode,
    max_addenda: usize,
) -> impl Strategy<Value = EntryParams> {
    let codes: Vec<_> = TransactionCode::ALL
        .iter()
        .copied()
        .filter(|code| service_class.allows(*code))
        .collect();
    (
        proptest::sample::select(codes),
        proptest::sample::select(&ROUTING_NUMBERS[..]),
        "[0-9]{1,17}",
        0u64..=99_999_999,
        text(15),
        text(22),
        proptest::collection::vec(text(80), 0..=max_addenda),
    )
        .prop_map(|(code, routing, account, amount, id, name, addenda)| {
            EntryParams::builder()
                .transaction_code(code)
                .receiving_dfi(routing.parse().unwrap())
                .account_number(account)
                .amount(if code.is_prenote() { 0 } else { amount })
                .individual_id(id)
                .individual_name(name)
                .addenda(addenda)
                .build()
        })
}

fn batch() -> impl Strategy<Value = BatchSpec> {
    (
        proptest::sample::select(ServiceClassCode::ALL),
        proptest::sample::select(
            &[
                StandardEntryClass::Ppd,
                StandardEntryClass::Ccd,
                StandardEntryClass::Ctx,
                StandardEntryClass::Tel,
            ][..],
        ),
    )
        .prop_flat_map(|(service_class_code, standard_entry_class)| {
            let max = usize::from(standard_entry_class.max_addenda()).min(3);
            proptest::collection::vec(entry(service_class_code, max), 0..8).prop_map(
                move |entries| BatchSpec {
                    service_class_code,
                    standard_entry_class,
                    entries,
                },
            )
        })
}

fn write(batches: &[BatchSpec]) -> (String, Vec<Vec<nacha::Entry>>) {
    let created_at = NaiveDate::from_ymd_opt(2024, 2, 29)
        .unwrap()
        .and_hms_opt(23, 59, 0)
        .unwrap();
    let mut writer = Writer::new(Vec::new());
    let mut file = writer
        .begin_file(
            FileParams::builder()
                .immediate_destination("091000019".parse().unwrap())
                .immediate_destination_name("DEST")
                .immediate_origin("1234567890")
                .immediate_origin_name("ORIGIN")
                .created_at(created_at)
                .build(),
        )
        .unwrap();
    let mut written = Vec::new();
    for spec in batches {
        let mut batch = file
            .begin_batch(
                BatchParams::builder()
                    .service_class_code(spec.service_class_code)
                    .company_name("ACME")
                    .company_id("1234567890")
                    .standard_entry_class(spec.standard_entry_class)
                    .company_entry_description("TEST")
                    .originating_dfi_id(9_100_001)
                    .build(),
            )
            .unwrap();
        let entries = spec
            .entries
            .iter()
            .map(|params| batch.entry(params.clone()).unwrap())
            .collect();
        batch.finish().unwrap();
        written.push(entries);
    }
    file.finish().unwrap();
    (String::from_utf8(writer.into_inner()).unwrap(), written)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn written_files_read_back_and_validate(batches in proptest::collection::vec(batch(), 0..5)) {
        let (output, written) = write(&batches);
        prop_assert_eq!(output.lines().count() % 10, 0);
        let file = File::read(output.as_bytes()).unwrap();
        prop_assert_eq!(file.validate(), vec![]);
        let read: Vec<Vec<nacha::Entry>> = file.batches.into_iter().map(|b| b.entries).collect();
        prop_assert_eq!(read, written);
    }

    #[test]
    fn arbitrary_input_never_panics(lines in proptest::collection::vec(
        proptest::collection::vec(any::<u8>(), 0..120), 0..12)
    ) {
        let input: Vec<u8> = lines.join(&b'\n');
        let _ = File::read(input.as_slice());
        for record in nacha::Reader::new(input.as_slice()).take(20) {
            let _ = record;
        }
    }

    #[test]
    fn mutated_files_never_panic(
        batches in proptest::collection::vec(batch(), 1..3),
        position in any::<proptest::sample::Index>(),
        byte in any::<u8>(),
    ) {
        let (output, _) = write(&batches);
        let mut bytes = output.into_bytes();
        let index = position.index(bytes.len());
        bytes[index] = byte;
        if let Ok(file) = File::read(bytes.as_slice()) {
            let _ = file.validate();
        }
    }
}
