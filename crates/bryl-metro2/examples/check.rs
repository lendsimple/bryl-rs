//! Reads and validates a Metro 2 file: `cargo run -p bryl-metro2 --example check -- FILE [--newline]`.

use std::fs;
use std::io::BufReader;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: check FILE [--newline]");
        return ExitCode::FAILURE;
    };
    let newline = args.any(|arg| arg == "--newline");
    let input = match fs::File::open(&path) {
        Ok(file) => BufReader::new(file),
        Err(err) => {
            eprintln!("{path}: {err}");
            return ExitCode::FAILURE;
        }
    };
    let file = match metro2::Reader::new(input)
        .newline(newline)
        .with_name(path.clone())
        .read_file()
    {
        Ok(file) => file,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    };
    println!(
        "{path}: {} data records, reporter {:?}",
        file.data_records.len(),
        file.header.reporter_name
    );
    let issues = file.validate();
    for issue in &issues {
        println!("  {issue}");
    }
    if issues.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
