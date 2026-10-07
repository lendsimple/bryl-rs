//! Parsing, validating and iterating arbitrary input must never panic.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(file) = nacha::File::read(data) {
        let _ = file.validate();
    }
    for record in nacha::Reader::new(data).take(64) {
        if record.is_err() {
            break;
        }
    }
});
