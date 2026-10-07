# bryl-rs

Fixed-width record files in Rust: a small declarative core, and NACHA (ACH)
and Metro 2 (credit reporting) built on it. Spec decisions and known
limitations are listed in [DEVIATIONS.md](DEVIATIONS.md).

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
- **Golden files** (`crates/*/tests/fixtures/golden`) are reference outputs
  the tests compare against byte for byte.
- **Fuzzing** (nightly and `cargo install cargo-fuzz`):
  ```sh
  cd fuzz
  cargo +nightly fuzz run nacha_file corpus/nacha_file seeds/nacha_file
  cargo +nightly fuzz run metro2_file corpus/metro2_file seeds/metro2_file
  ```
- The Metro 2 reference files in `crates/bryl-metro2/tests/fixtures/moov`
  come from [moov-io/metro2](https://github.com/moov-io/metro2) (Apache-2.0).
