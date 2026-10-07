//! Parsing (RDW and newline framing), validating and iterating arbitrary
//! input must never panic.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    for newline in [false, true] {
        if let Ok(file) = metro2::Reader::new(data).newline(newline).read_file() {
            let _ = file.validate();
        }
    }
    for segment in metro2::Reader::new(data).segments().take(64) {
        if segment.is_err() {
            break;
        }
    }
});
