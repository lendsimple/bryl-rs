# bryl-rs

[![CI](https://github.com/kolanos/bryl-rs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/kolanos/bryl-rs/actions/workflows/ci.yml)

Fixed-width record files in Rust: a small declarative core, and NACHA (ACH)
and Metro 2 (credit reporting) built on it. Each format crate's
documentation lists the sources for its rules and what it does not
support.

The core is a Rust take on [bryl](https://github.com/balanced/bryl), Balanced's
library for declaratively defining, constructing and serializing fixed-width
records made of typed fields.

| Crate | Import as | What it is |
|-------|-----------|------------|
| [`bryl`](crates/bryl) | `bryl` | Field codecs, `#[derive(Record)]`, `#[derive(Code)]`, line/block readers |
| [`bryl-nacha`](crates/bryl-nacha) | `nacha` | NACHA ACH files: writer, reader, validation |
| [`bryl-metro2`](crates/bryl-metro2) | `metro2` | Metro 2 character-format files: writer, reader, validation |
| `bryl-derive`, `bryl-pattern` | (through `bryl`) | The derive macros and the date-pattern tokenizer they share |

```rust
use bryl::{Code, Const, Record};

#[derive(Code, Debug, Clone, Copy, PartialEq)]
enum Status {
    #[code("11")]
    Current,
    #[code("97")]
    ChargeOff,
}

#[derive(Record, Debug, Clone, PartialEq)]
#[bryl(sanitize(upper), length = 23)]
struct Account {
    #[bryl(alpha(2), constant = "AC")]
    tag: Const,
    #[bryl(alpha(10))]
    surname: String,
    #[bryl(numeric(9))]
    balance: u64,
    #[bryl(alpha(2))]
    status: Status,
}
```

Lengths, offsets, date patterns and field types are checked at compile time;
see the crate docs (`cargo doc --open`) for the attributes.

## Try it

```sh
cargo run -p bryl-nacha --example write_sample > sample.ach
cargo run -p bryl-nacha --example check -- sample.ach

cargo run -p bryl-metro2 --example write_sample > sample.dat
cargo run -p bryl-metro2 --example check -- sample.dat
```

## Development

Requires Rust 1.85 or later.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

- **Compile-error tests** (`crates/bryl/tests/ui`) match current stable rustc
  wording; regenerate with `TRYBUILD=overwrite cargo test -p bryl --test ui`.
- **Snapshots** (`crates/*/tests/fixtures/snapshots`) hold the writers'
  output for fixed scenarios; `tests/snapshots.rs` compares against them
  byte for byte. After an intended change to written bytes, regenerate with
  `UPDATE_SNAPSHOTS=1 cargo test --test snapshots` and review the diff.
- **Fuzzing** (nightly and `cargo install cargo-fuzz`):
  ```sh
  cd fuzz
  cargo +nightly fuzz run nacha_file corpus/nacha_file seeds/nacha_file
  cargo +nightly fuzz run metro2_file corpus/metro2_file seeds/metro2_file
  ```
- The Metro 2 reference files in `crates/bryl-metro2/tests/fixtures/moov`
  come from [moov-io/metro2](https://github.com/moov-io/metro2) (Apache-2.0).

## License

Licensed under the [MIT License](LICENSE).

The moov-io reference files in `crates/bryl-metro2/tests/fixtures/moov` and
`fuzz/seeds/metro2_file` remain under moov-io's Apache License 2.0 (see the
`LICENSE` and `MOOV-LICENSE` files beside them).
