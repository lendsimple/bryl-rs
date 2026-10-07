//! Reads and validates a NACHA file: `cargo run -p bryl-nacha --example check -- FILE`.

use std::fs;
use std::io::BufReader;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: check FILE");
        return ExitCode::FAILURE;
    };
    let input = match fs::File::open(&path) {
        Ok(file) => BufReader::new(file),
        Err(err) => {
            eprintln!("{path}: {err}");
            return ExitCode::FAILURE;
        }
    };
    let file = match nacha::Reader::new(input)
        .with_name(path.clone())
        .read_file()
    {
        Ok(file) => file,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    };
    let entries: usize = file.batches.iter().map(|batch| batch.entries.len()).sum();
    println!(
        "{path}: {} batches, {entries} entries, debits {}, credits {}",
        file.batches.len(),
        file.control.total_debit_amount,
        file.control.total_credit_amount
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
