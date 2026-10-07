# bryl-rs — Implementation Plan

## Context

`lms-python/backend/common/` contains three Python modules:

- `bryl.py`: a declarative framework for fixed-width records. Fields are typed and fixed-size (Alphanumeric, Numeric, Date, Time, Datetime) and are declared as class attributes. A metaclass orders them, works out offsets and lengths, and gives records `dump`/`load`. It also has line and block readers.
- `metro2.py`: Metro 2 credit reporting (header, base segment, J1/J2/K1–K4/L1/N1 segments, trailer). It includes a writer that accumulates the trailer, plus a reader for RDW mode and newline mode.
- `nacha.py`: NACHA ACH files (file header, batch header, entry detail, addendum, batch control, file control). It includes a nested-context writer that computes hashes, totals and block counts, plus a structured reader.

We are building an equivalent Rust library in the empty repo `/Users/kolanos/Web/bryl-rs`. Decisions already made:

| Decision | Choice |
|---|---|
| Packaging | Cargo workspace: `bryl`, `bryl-derive`, `bryl-metro2`, `bryl-nacha` |
| Record DSL | `#[derive(Record)]` (and `#[derive(Code)]` for code tables) on typed structs and enums |
| Output fidelity | **Spec-correct**: fix the quirks the Python code has, and document each deviation from Python |
| Dates | `chrono` (`NaiveDate`, `NaiveTime`, `NaiveDateTime`) |

Toolchain: rustc 1.97 is installed locally. Use edition 2024 with `rust-version = "1.85"`.

---

## Python behavior inventory (what we port and what we change)

These are the subtleties found while reading the code and tests. Each one is either **kept**, **changed** (a spec-correct fix or a Rust idiom), or **dropped**.

### bryl core

| # | Python behavior | Rust decision |
|---|---|---|
| B1 | Field order comes from a global `itertools.count()` at construction. `.constant()` and `.reserved()` copies keep the original order, and subclass overrides inherit the base field's order. | **Changed**: order is the struct declaration order. Composition goes through `#[bryl(flatten)]` instead of inheritance. |
| B2 | `required = required and default is None`. Required fields have no default and fail **lazily at `dump()`** with `LookupError`. Optional fields fall back to the class default (`0`, `""`, `None`). | **Changed**: required fields are checked at compile time through a `bon` typestate builder. Optional fields use `#[builder(default)]`. |
| B3 | `map()` coerces values. It sanitizes, validates, then tries `load(value)`, then `load(str(value))`. So `"123"` becomes `123` for Numeric, and `5` becomes `"5"` for Alpha. | **Dropped**: Rust types replace the coercion. |
| B4 | A thread-local global `ctx` stack holds `alpha_filter`, `alpha_truncate` and `alpha_upper`. `metro2` and `nacha` **both mutate the shared root frame** with `bryl.ctx(alpha_upper=True)`, so importing either one uppercases everything everywhere. | **Changed**: explicit `Sanitize` options. Defaults are set per record with `#[bryl(sanitize(upper))]`, and a writer can override them. No global state. |
| B5 | `Record.load` calls `cls(**values)`, which re-runs sanitize. **Reading uppercases data.** | **Changed**: decoding never mutates. Sanitize runs only on encode. |
| B6 | `Alphanumeric.alphabet = string.printable`, which allows `\t \n \r \x0b \x0c`. | **Changed**: printable ASCII only (`0x20..=0x7E`). |
| B7 | Numeric `validate`: the value must parse as a Decimal, satisfy `len(str(v)) <= length`, satisfy `min_value` (default 0) and `max_value`, be in the enum, and match the constant. | **Kept**, using unsigned integer types and a digit-count check. |
| B8 | Numeric `load`: an empty string becomes `0`, and `int(raw)` accepts `" 12"` and `"+12"`. | **Changed**: after stripping the pad, the value must be ASCII digits only. Empty becomes `0`. |
| B9 | `pack`: validate, `str(dump)`, then pad by alignment (LEFT pads right, RIGHT pads left). `unpack`: requires `len >= length`, slices, strips the pad on the aligned side, matches the optional regex `pattern`, loads, validates. | **Kept**. `pattern` is **dropped** because it is unused. |
| B10 | Constant handling: `__set__` raises if the value differs, `fill` silently ignores it, and Alpha `validate` **does not** check constants on load (Numeric does). | **Changed**: constants are always checked on decode and are never stored. |
| B11 | `.reserved()` needs a class default (Alpha becomes spaces, Numeric becomes zeros). `Datetime` has no default, so `.reserved()` raises `TypeError`. | **Kept**: `#[bryl(reserved)]` is only valid on alpha and numeric fields. This is a compile error otherwise. |
| B12 | `enum=` accepts a list, a list of tuples, or a dict. Values are validated and keys become attributes on the field. | **Changed**: `#[derive(Code)]` Rust enums. |
| B13 | Datetime format tokens `YYYY YY DDD JJJ DD MM hh HH mm ss pp ZZZ` (`X{2}` is in the regex but has no spec, so it would raise `KeyError`). `ZZZ` uses pytz `localize`. `Date`/`Time` use narrower token sets. | **Changed**: the derive macro parses tokens **at compile time** and our own codec formats and parses them. `ZZZ` and `XX` are **dropped** because neither format uses them. |
| B14 | `Field.probe(io)` and `Record.probe(io)` seek, read and restore the stream position, returning `None` on error. | **Changed**: `Record::probe(&[u8]) -> Option<Self>` works on byte slices, so no seeking is needed. |
| B15 | `Record` is a `dict`. Equality is dict equality, and `copy()` is `type(self)(**self)`. | **Changed**: plain structs with `#[derive(Clone, PartialEq, Debug)]`. |
| B16 | `LineReader`/`BlockReader` take an `as_record_type(reader, data, offset)` callback that returns a class or an instance. `next_record(expected_type, default="raise")` uses a one-slot `retry` buffer (a peek). `include_terminal`/`expected_terminal` check the EOL. | **Changed**: a `Dispatch` trait plus a `Reader<S, D>` with `peek()`, `next_if()`, and terminator checking. |
| B17 | `MalformedError(file_name, offset, reason)` is a `ValueError`. | **Changed**: `bryl::ReadError { source, location, kind }` built with `thiserror`. |

### Metro 2

| # | Python behavior | Rust decision |
|---|---|---|
| M1 | Zero-fill dates and timestamps: an optional date defaults to the `_ZERO_FILL` sentinel, `__get__` maps it to `None`, `"00000000"` loads as `None`, and `None` dumps as zeros. | **Kept** as `Option<NaiveDate>` / `Option<NaiveDateTime>`, where `None` is zero-filled. |
| M2 | A **required** date explicitly set to `None` silently dumps zeros, because the overridden validate accepts `None`. | **Changed**: required dates are `NaiveDate`, so there is no `None`. |
| M3 | Code tables (`AccountStatuses` etc.) are plain dicts that are **never used to validate fields**. | **Changed**: fields use typed `Code` enums, so unknown codes are rejected when writing and reading. |
| M4 | The base segment's RDW is overwritten with the total data-record length in `DataRecord.dump()` on a copy, so the original is not mutated. | **Kept**: the RDW is computed during encoding and is not a stored field. |
| M5 | Segment output order: J1*, J2*, K1, K2, K3, K4, L1, N1. On load, an unknown 2-character segment id stops segment parsing (it is treated as padding). | **Kept**. |
| M6 | Trailer accumulation: status counters; SSN (`0 < ssn < 999999999`), DOB, phone (`> 0`) and ECOA `Z` counts across base/J1/J2; K/L/N counts; `block_count = total_base_records + 2`; the "all segments" totals are summed at the end. | **Kept**. `block_count` matches moov-io's test data (`block_count=3` for 1 base record). ⚠ Verify against the CRRG. |
| M7 | The validation helpers (`validate_payment_rating`, `validate_amount_past_due`, `validate_payment_history`) exist but the **writer never calls them**. | **Changed**: `BaseSegment::validate()` runs the cross-field rules, and the writer calls it by default (`Writer::validate(false)` turns that off). |
| M8 | `PAYMENT_RATING_FOR_STATUS` maps status 11→`0`, 71→`1`, and so on. | ⚠ **Verify against the CRRG** before encoding the rule. The CRRG states that Payment Rating is reported only for statuses 05/13/65/88/89/94/95 and is blank otherwise. Port whichever rule the guide confirms, and note it in `DEVIATIONS.md`. |
| M9 | Reader RDW mode: read 4 digits, then `rdw-4` more characters. A non-digit is "invalid RDW", `rdw < 4` is invalid, and a short read is "truncated record". Header and trailer RDWs are **normalized** (fixed-length files pad them, e.g. `0470`). Newline mode skips empty lines. | **Kept**. |
| M10 | `_detect_type` always reports **offset 0** in its error. | **Changed**: it reports the real offset. |
| M11 | `Reader.__iter__` flattens to header, base, segments…, trailer. | **Kept** as `Reader::records() -> impl Iterator<Item = Result<Metro2Record>>`. |
| M12 | Variable-blocked files with a BDW prefix (`unpacked_variable_file.dat` starts `0496 0426HEADER`) and packed format are not supported. | BDW support is a **stretch goal** (Stage 6). Packed format is **out of scope**. |

### NACHA

| # | Python behavior | Rust decision |
|---|---|---|
| N1 | `addenda_sequence_number = len(addenda)` starts at **0**. | **Changed**: starts at **1** (spec: "number consecutively beginning with 0001"). |
| N2 | `entry_count` is reset per batch, so trace sequences restart at 1 in each batch. Two batches with the same ODFI produce **duplicate trace numbers**. | **Changed**: the sequence is unique and ascending across the whole file. |
| N3 | Block count is `ceil(lines / 10)`, but the file is **not padded** with `9…9` filler records. | **Changed**: pad to a multiple of 10 lines with 94×`'9'` records. This is on by default and can be turned off with `.pad_blocks(false)`. |
| N4 | `begin_company_batch` never checks for a file context. Contexts are a runtime stack of bound methods. | **Changed**: typestate guards, so nesting is enforced by the borrow checker. |
| N5 | A batch's service class code (200/220/225) is not checked against its entries. | **Changed**: 220 (credits only) rejects debits and 225 (debits only) rejects credits. |
| N6 | Prenote amounts and routing check digits are not validated. | **Changed**: prenote codes (23/28/33/38) require `amount == 0`. Receiving routing numbers are validated with the ABA 3-7-1 checksum. |
| N7 | `immediate_destination` is `Numeric(10, pad=" ")`, which gives `" 123456789"`. `immediate_origin` is `Alphanumeric(10)` and **left**-aligned, so a 9-digit origin becomes `"123456789 "`. | **Changed**: `immediate_origin` is right-aligned and space-padded to match destination. ⚠ Confirm with the receiving bank's spec. |
| N8 | The number of addenda per entry is unlimited. | **Changed**: per-SEC limits (PPD/CCD/WEB/TEL… ≤ 1, CTX ≤ 9999). ⚠ Verify the per-SEC table against the NACHA Operating Rules. |
| N9 | `created_at` defaults to `datetime.utcnow()`. | **Changed**: `created_at` is an explicit `NaiveDateTime`. `FileParams::now()` is a convenience constructor. |
| N10 | Entry hash is the sum of 8-digit TRNs mod 10¹⁰. Debit/credit totals come from `code % 10`. `entry_addenda_count` is 1 + the number of addenda. `batch_number` increments from 1. | **Kept**. |
| N11 | `transaction_code_for(amount, receiving_type, is_return, is_prenote)`: return → 21; prenote or amount 0 → 23; otherwise 22. A negative amount adds 5, savings adds 10, and anything else is a `ValueError`. | **Kept** as `TransactionCode::for_entry(amount: i64, AccountKind, is_return, is_prenote)`. |
| N12 | Reader: `as_record_type` dispatches on the first character. The structured reader uses `file_header()`, `company_batches()`, `entries()`, `company_batch_control()` and `file_control()`, built on `next_record(expected_type, None)`, which peeks. | **Kept** as pull methods. A tree parser (`File::parse`) and `File::validate()` are **added**. Filler `9…9` lines are skipped. |
| N13 | `EntryDetail.mask()` mutates the account number to `X` repeated 17 times. | **Changed**: `mask(&self) -> Self` returns a masked copy. |

Every **Changed** row that alters bytes on the wire gets an entry in `DEVIATIONS.md` at the repo root, with a before/after example. Rows marked ⚠ need to be checked against the governing spec (CDIA CRRG for Metro 2, NACHA Operating Rules for NACHA) before that stage is marked complete.

---

## Workspace layout

```
bryl-rs/
├── Cargo.toml                 # [workspace] resolver = "3", shared [workspace.package]/[workspace.dependencies]/[workspace.lints]
├── rust-toolchain.toml        # channel = "stable"
├── implementation.md          # this plan (deleted when all stages complete)
├── DEVIATIONS.md              # every intentional output difference from lms-python
├── crates/
│   ├── bryl/                  # runtime: field codecs, Record/Code traits, Encoder/Decoder, readers, errors
│   │   ├── src/{lib.rs, field.rs, codec/{alpha,numeric,datetime,option}.rs, record.rs, code.rs,
│   │   │        sanitize.rs, error.rs, read/{mod,line,block,dispatch}.rs}
│   │   └── tests/{field.rs, record.rs, readers.rs, derive.rs, ui/*.rs (trybuild)}
│   ├── bryl-derive/           # proc-macro: #[derive(Record)], #[derive(Code)]
│   │   └── src/{lib.rs, record.rs, code.rs, attrs.rs, pattern.rs}
│   ├── bryl-metro2/           # package bryl-metro2, [lib] name = "metro2"
│   │   ├── src/{lib.rs, codes.rs, header.rs, base.rs, segments.rs, trailer.rs, data_record.rs,
│   │   │        validate.rs, writer.rs, reader.rs, error.rs}
│   │   └── tests/{records.rs, writer.rs, reader.rs, roundtrip.rs, moov_fixtures.rs, fixtures/…}
│   └── bryl-nacha/            # package bryl-nacha, [lib] name = "nacha"
│       ├── src/{lib.rs, codes.rs, records.rs, routing.rs, entry.rs, writer.rs, reader.rs, file.rs, error.rs}
│       └── tests/{records.rs, writer.rs, reader.rs, roundtrip.rs, fixtures/…}
└── tools/gen_golden.py        # regenerates golden files from lms-python
```

`bryl` re-exports the derives (`pub use bryl_derive::{Record, Code};`), so users depend only on `bryl`.

### Dependencies

| Crate | Deps | Dev-deps |
|---|---|---|
| `bryl` | `bryl-derive`, `chrono` (`default-features = false, features = ["alloc"]`), `thiserror` | `proptest`, `trybuild`, `pretty_assertions` |
| `bryl-derive` | `syn` 2 (`full`), `quote`, `proc-macro2` | |
| `bryl-metro2` / `bryl-nacha` | `bryl`, `chrono`, `thiserror`, `bon` | `proptest`, `pretty_assertions` |

Workspace lints: `unsafe_code = "forbid"`, `missing_docs = "warn"`, and clippy `pedantic` as warn, with the noisy lints allowed explicitly. CI runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`, `cargo doc --no-deps` (with `RUSTDOCFLAGS=-D warnings`), and an MSRV build on 1.85.

---

## Core design (`bryl`)

### Data model: bytes, ASCII only

Records encode to and decode from `&[u8]`. Every valid value is printable ASCII, so byte offsets equal character offsets, and `String::from_utf8` on encoded output cannot fail. (The Python code works on `str`, where offsets are character counts; any non-ASCII input was already invalid there.) Encoders write into a reusable `Vec<u8>`, which avoids per-field allocations.

### Field metadata

```rust
pub enum Align { Left, Right }

pub enum FieldKind {
    Alpha,
    Numeric { min: Option<u64>, max: Option<u64> },
    Date(&'static [Token]),            // tokens produced at compile time by the derive
    Time(&'static [Token]),
    DateTime(&'static [Token]),
}

pub struct FieldSpec {
    pub name: &'static str,
    pub offset: usize,                 // computed by the derive (const)
    pub length: usize,
    pub pad: u8,
    pub align: Align,
    pub kind: FieldKind,
    pub constant: Option<Constant>,    // Str(&'static str) | Num(u64)
    pub sanitize: Option<Sanitize>,    // field-level override
}
```

`FieldSpec` replaces Python's `Field` instances for introspection, for example `K1Segment::FIELDS` (lengths and offsets, as in `test_field_offsets`).

### Value codecs: the `FieldValue` trait

```rust
pub trait FieldValue: Sized {
    fn encode(&self, spec: &FieldSpec, cx: &EncodeCx, out: &mut Vec<u8>) -> Result<(), FieldErrorKind>;
    fn decode(spec: &FieldSpec, raw: &[u8]) -> Result<Self, FieldErrorKind>;  // raw is already sliced to spec.length
}
```

Built-in implementations:

| Rust type | Valid kinds | Encode | Decode |
|---|---|---|---|
| `String` | Alpha | sanitize (upper/filter/truncate), check printable ASCII, check `len <= length`, pad | strip the pad on the aligned side, check printable |
| `u8 u16 u32 u64` | Numeric (Alpha also accepted, e.g. a numeric value padded with spaces) | check digit count `<= length` and min/max, pad (default `'0'`, right) | strip the pad; empty → 0; ASCII digits only; overflow → error |
| `NaiveDate` / `NaiveTime` / `NaiveDateTime` | Date / Time / DateTime | render the tokens | parse the tokens, then `from_ymd_opt`/`from_yo_opt`/`from_hms_opt` |
| `Option<T>` | any | `None` → all `'0'` for Numeric/Date kinds, all spaces for Alpha | all pad / all zeros / all spaces → `None` |
| `#[derive(Code)]` enums | Alpha or Numeric | `as_code()` then the Alpha/Numeric rules | `from_code()`; an unknown code → `FieldErrorKind::UnknownCode` |
| `#[derive(Record)]` structs (flatten) | n/a | delegate | delegate |

**Date tokens:** `YYYY`, `YY`, `MM`, `DD`, `DDD` (and its alias `JJJ`, day of year), `hh` (24h), `HH` (12h), `mm`, `ss`, `pp` (AM/PM), and literal characters. These are parsed in `bryl-derive/src/pattern.rs`, and an unknown or ambiguous token is a **compile error** that points at the pattern. We use our own tokenizer rather than chrono's strftime to avoid the two-digit-year pivot ambiguity: `YY` decodes as `2000 + yy`. Python's pivot gives 69–99 → 19xx, but no field in either format carries pre-2000 two-digit years; this is documented in `DEVIATIONS.md`. The field length is the pattern length, as in Python.

### Sanitize (replaces the global `ctx`)

```rust
#[derive(Clone, Copy, Default)]
pub struct Sanitize { pub upper: bool, pub filter: bool, pub truncate: bool }
```

Settings resolve in this order: the field attribute, then the writer or `encode_with` override, then the record attribute (`#[bryl(sanitize(upper))]`), then `Sanitize::default()` (all off). Sanitizing happens **only during encoding** (see B5). Filter drops characters that aren't printable ASCII, truncate cuts to the field length, and upper uppercases ASCII. This is the same order as Python's `sanitize()`: filter, then truncate, then upper.

### `Record` trait (implemented by the derive)

```rust
pub trait Record: Sized {
    const NAME: &'static str;
    const LENGTH: usize;
    const FIELDS: &'static [FieldSpec];
    const SANITIZE: Sanitize;

    fn encode_into(&self, cx: &EncodeCx, out: &mut Vec<u8>) -> Result<(), Error>;
    fn decode(raw: &[u8]) -> Result<Self, Error>;           // Err if raw.len() < LENGTH; ignores trailing bytes (as Python)

    // provided
    fn encode(&self) -> Result<String, Error>;
    fn encode_with(&self, sanitize: Sanitize) -> Result<String, Error>;
    fn decode_exact(raw: &[u8]) -> Result<Self, Error>;     // Err if raw.len() != LENGTH
    fn probe(raw: &[u8]) -> Option<Self> { Self::decode(raw).ok() }
}
```

Errors carry context: `Error::Field { record: "BaseSegment", field: "surname", offset: 231, kind }` and `Error::Length { record, expected, actual }`. `FieldErrorKind` covers `TooLong`, `InvalidChar { ch, index }`, `NotNumeric`, `BelowMin`, `AboveMax`, `UnknownCode`, `ConstantMismatch { expected, found }`, and `InvalidDate`.

### `#[derive(Record)]` syntax

```rust
use bryl::Record;
use chrono::NaiveDate;

#[derive(Record, bon::Builder, Debug, Clone, PartialEq)]
#[bryl(sanitize(upper), length = 34)]           // `length` = compile-time assert on the summed field lengths
pub struct K1Segment {
    #[bryl(alpha(2), constant = "K1")]
    #[builder(skip)]
    segment_identifier: bryl::Const,            // zero-sized marker; nothing is stored
    #[bryl(alpha(30))]
    pub original_creditor_name: String,
    #[bryl(numeric(2))]
    pub creditor_classification: CreditorClassification,   // #[derive(Code)] enum
}
```

Field attributes:

| Attribute | Meaning |
|---|---|
| `alpha(N)` / `numeric(N)` | kind and length |
| `date("MMDDYYYY")`, `time("hhmm")`, `datetime("MMDDYYYYhhmmss")` | kind and pattern (length = pattern length) |
| `pad = ' '`, `align = "left" \| "right"` | override the kind defaults (Alpha: `' '`/left; Numeric: `'0'`/right) |
| `min = N`, `max = N` | numeric bounds (`min` defaults to 0 because the types are unsigned) |
| `constant = "K1"` or `constant = 426` | checked on decode; the field type must be `bryl::Const` |
| `reserved` | filler (Alpha = spaces, Numeric = zeros); type `bryl::Const`; compile error on date kinds (B11) |
| `sanitize(upper, filter, truncate)` / `no_sanitize` | field-level override |
| `flatten` | the field type is itself a `Record`; its fields are spliced in at this position |

Generated items:
- `impl Record`
- `const _: () = assert!(Self::LENGTH == 34);` when `length` is given
- an inherent `pub const SEGMENT_IDENTIFIER: &str = "K1";` for each constant (replaces Python's `Field.value`)
- `pub const <FIELD>_OFFSET: usize` for each field

Compile-time errors (tested with trybuild):
- an unknown attribute
- a missing kind
- a bad date pattern
- `constant`/`reserved` on a field that isn't `Const`
- `reserved` on a date
- a `length` mismatch
- a numeric constant longer than the field (replaces `test_constant_invalid_raises`)

### `#[derive(Code)]` for code tables

```rust
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountStatus {
    #[code("05")] Transferred,
    #[code("11")] Current,
    // …
}
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceClassCode { #[code(200)] Mixed, #[code(220)] CreditsOnly, #[code(225)] DebitsOnly }
```

This generates `as_code()`, `from_code()`, `pub const ALL: &[Self]` (so the Python count tests become `AccountType::ALL.len() == 66`), `Display`, `FromStr`, `TryFrom<&str>` / `TryFrom<u64>`, and `FieldValue`. The derive rejects duplicate codes at compile time.

### Readers

```rust
pub trait Source {                           // yields raw records with their location
    fn next_raw(&mut self) -> Result<Option<Raw>, ReadError>;
}
pub struct Raw { pub bytes: Vec<u8>, pub terminator: Vec<u8>, pub location: Location }
pub enum Location { Line(usize), Offset(u64) }

pub struct LineSource<R: BufRead>   { /* line_no, optional expected_terminator ("\n" | "\r\n") */ }
pub struct BlockSource<R: Read>     { /* record_size, byte offset */ }

pub trait Dispatch: Sized {                  // replaces as_record_type
    fn dispatch(raw: &[u8]) -> Result<Self, bryl::Error>;
}

pub struct Reader<S: Source, D: Dispatch> { source: S, peeked: Option<(Raw, D)>, name: Cow<'static, str> }
impl<S, D> Reader<S, D> {
    pub fn peek(&mut self) -> Result<Option<&D>, ReadError>;
    pub fn next_if<T>(&mut self, f: impl FnOnce(D) -> Result<T, D>) -> Result<Option<T>, ReadError>; // like next_record(expected_type, None)
    pub fn expect<T>(&mut self, what: &str, f: impl FnOnce(D) -> Result<T, D>) -> Result<T, ReadError>; // like next_record(expected_type) with "raise"
}
impl<S, D> Iterator for Reader<S, D> { type Item = Result<D, ReadError>; }
```

`ReadError { source: name, location, kind: ReadErrorKind }`. The kinds are `UnexpectedEof`, `UnexpectedRecord { expected, found }`, `UnexpectedTerminator`, `Record(bryl::Error)`, `Io(io::Error)`, and `Other(String)`. Its `Display` matches Python's `"{file} @ {offset} - {reason}"`. The one-slot peek buffer reproduces Python's `retry` semantics without the side-effecting `retry` field.

---

## Format crates

### `bryl-nacha` (lib `nacha`)

**Records** (all 94 characters long, `#[bryl(sanitize(upper), length = 94)]`):
- `FileHeader`, `BatchHeader`, `EntryDetail`, `Addendum`, `BatchControl`, `FileControl`, with field layouts as in `nacha.py`.
- `EntryDetail` stores `receiving_dfi: RoutingNumber` as `numeric(9)`. This is a newtype with ABA checksum validation and the methods `trn() -> u32` and `check_digit() -> u8`. It is still 8+1 digits on the wire, and replaces `receiving_dfi_trn` + `receiving_dfi_trn_check_digit` + `receiving_dfi_routing_number`.
- The account number is `alpha(17)` with `align = "right"`.

**Codes:**
- `ServiceClassCode`, `StandardEntryClass` (23 variants)
- `TransactionCode` (12 variants), with methods `is_checking`, `is_savings`, `is_credit`, `is_debit`, `is_prenote`, `is_return`, and `for_entry(amount: i64, AccountKind, is_return, is_prenote)`
- `AccountKind { Checking, Savings }`, which replaces the `"checking"`/`"savings"` strings

**Entry:** `pub struct Entry { pub detail: EntryDetail, pub addenda: Vec<Addendum> }` with `is_rejection()`, `mask()`, `encode()` (joined with `\n`), and `decode()`.

**Dispatch:** `enum NachaRecord { FileHeader(..), BatchHeader(..), Entry(..), Addendum(..), BatchControl(..), FileControl(..), Filler }`, keyed on the first byte. A line of 94 × `'9'` is `Filler`.

**Writer (typestate guards):**

```rust
let mut out = Vec::new();
let mut file = nacha::Writer::new(&mut out)          // W: io::Write
    .pad_blocks(true)                                  // default
    .begin_file(FileParams { immediate_destination, immediate_destination_name,
                             immediate_origin, immediate_origin_name,
                             created_at, file_id_modifier: 'A', reference_code: None })?;
{
    let mut batch = file.begin_batch(BatchParams { service_class_code, company_name, company_id,
                                                   standard_entry_class, company_entry_description,
                                                   originating_dfi_id, effective_entry_date: None,
                                                   company_descriptive_date: None,
                                                   company_discretionary_data: None })?;  // &mut borrow of `file`
    let entry: Entry = batch.entry(EntryParams { transaction_code, receiving_dfi, account_number,
                                                 amount, individual_id, individual_name,
                                                 trace_number: None, discretionary_data: None,
                                                 addenda: vec!["MEMO".into()] })?;
    batch.finish()?;                                    // writes BatchControl
}
let summary: FileControl = file.finish()?;             // writes FileControl + 9-filler padding
```

- Guards are `#[must_use]`. Dropping one without calling `finish()` is the Python "exception inside the `with`" case: the control record is not written, and the docs say so. The borrow checker makes it impossible to open a batch outside a file or two files at once, which replaces the Python `"Cannot be in context"` / `"Not in company batch context"` runtime errors.
- `entry()` is atomic: it validates everything (routing checksum, prenote amount, service-class/debit-credit match, the SEC addenda limit) **before** writing, so a rejected entry leaves the output untouched.
- Totals live in a pure, unit-testable `Totals` accumulator (hash mod 10¹⁰, debit/credit sums, entry+addenda counts). It is shared by the batch and the file and reused by `File::validate()`.
- Trace numbers are ODFI (8) followed by a file-wide sequence (7), per N2. `entry_detail_sequence_number` is the last 7 digits of the trace, and addenda sequence numbers start at 1 (N1).

**Reader:**
- Pull API that mirrors Python: `file_header()`, `next_batch()` → `Option<BatchHeader>`, `entries()` (an iterator borrowing `&mut self`, which stops at a non-entry record), `batch_control()`, `file_control()`.
- Flat iteration: `records()`; `filter` is just `.filter_map`.
- Tree parse: `nacha::File::parse(reader) -> Result<File>`, then `File::validate() -> Vec<Issue>`. This recomputes the hashes, totals, counts and block count and compares them with the control records.

### `bryl-metro2` (lib `metro2`)

**Records:**
- `HeaderRecord` (426), `BaseSegment` (426), `J1Segment` (100), `J2Segment` (200), `K1Segment` (34), `K2Segment` (34), `K3Segment` (40), `K4Segment` (30), `L1Segment` (54), `N1Segment` (146), `TrailerRecord` (426).
- Every one is `#[bryl(sanitize(upper), length = N)]`, so every offset in `test_field_offsets` is asserted at compile time.
- `BaseSegment` does **not** store the RDW. It is written by `DataRecord::encode` (M4) and exposed on decode as `DataRecord::rdw`.

**Codes** (each a `#[derive(Code)]` enum):
- Alpha: `PortfolioType`, `AccountType` (66), `AccountStatus` (23), `PaymentRating`, `EcoaCode`, `ConsumerInformationIndicator` (25), `ComplianceConditionCode`, `SpecialComment` (51), `TermsFrequency`, `AddressIndicator`, `ResidenceCode`, `GenerationCode`, `InterestTypeIndicator`
- Numeric: `CreditorClassification`, `ChangeIndicator`, `PurchasedIndicator`, `AgencyIdentifier`, `SpecializedPaymentIndicator`
- `PaymentHistoryCode`, used by `PaymentHistoryProfile`, a validated `alpha(24)` newtype
- Optional codes are `Option<Code>` (blank ⇄ `None`)

**DataRecord:**

```rust
pub struct DataRecord {
    pub base: BaseSegment,
    pub j1: Vec<J1Segment>, pub j2: Vec<J2Segment>,
    pub k1: Option<K1Segment>, pub k2: Option<K2Segment>, pub k3: Option<K3Segment>,
    pub k4: Option<K4Segment>, pub l1: Option<L1Segment>, pub n1: Option<N1Segment>,
}
```

- `encode` writes the RDW as the sum of the segment lengths and errors if it exceeds 9999.
- `decode` follows M5, and a truncated segment is an error (`"truncated J1 segment"`).
- `segments()` iterates in output order.

**Validation** (`validate.rs`):
- `is_valid_ssn`, `is_valid_phone`, `is_valid_dob`
- `validate_payment_rating` (⚠ M8), `validate_amount_past_due`, `validate_payment_history` (the B-embedding rule)
- `BaseSegment::validate() -> Result<(), Vec<Violation>>`

**Writer:**

```rust
let mut file = metro2::Writer::new(&mut out).newline(false).validate(true)
    .begin_file(header /* HeaderRecord built via bon; date_created required */)?;   // writes header
file.write(&data_record)?;                         // validates, writes, accumulates
let trailer: TrailerRecord = file.finish()?;       // finalizes the "all segments" totals, writes the trailer
```

Accumulation lives in `TrailerRecord::accumulate(&mut self, &DataRecord)` and `TrailerRecord::finalize(&mut self)`. These are pure functions and are tested directly. The status → counter mapping is a `match` on `AccountStatus`, replacing the `STATUS_COUNTER_MAP` strings.

**Reader:**
- `Reader::new(r)` for RDW mode, or `.newline(true)`.
- `header()`, `data_records()` (an iterator borrowing `&mut self`; it peeks the trailer and stops), `trailer()`, and `records()` (flat, `Metro2Record` enum, per M11).
- Header and trailer RDW normalization follow M9. `MalformedError` becomes `ReadError` with a real `Location::Offset` (M10).

---

## Stages

Each stage leaves the workspace compiling, with `fmt`, `clippy -D warnings` and the tests green, and is committed on its own.

### Stage 1: Workspace scaffold and `bryl` runtime (no macros)
**Goal**: workspace, CI config, and `bryl` with `FieldSpec`, the `FieldValue` implementations, the date token codec, `Sanitize`, the `Record` trait, `Const`, and the errors. One hand-written `impl Record` is used as a test fixture.
**Success criteria**: field pack/unpack, sanitize and date codec tests pass, and proptest round-trips `decode(encode(x)) == x` for every built-in `FieldValue`.
**Tests**: ports of `TestField`, `TestFieldPackUnpack`, `TestNumeric`, `TestAlphanumeric`, `TestDatetime`, `TestDate`, `TestTime`, and `TestContext` (re-expressed as `Sanitize` precedence tests), plus:
- the B6 printable-only rule
- B8 strict digits
- `YY` → 20yy
- `DDD` day-of-year
- `hh`/`HH`/`pp`
- `Option<NaiveDate>` zero-fill
**Status**: Complete. 142 tests (field, pattern, record, sanitize, proptest round-trips, doctest); clippy pedantic clean; builds and tests on 1.85.
**Notes** (small departures from the design above):
- `FieldValue::encode` receives the already-resolved `Sanitize` instead of an `EncodeCx`; `EncodeCx` is only passed to `Record::encode_into`.
- `Record` implementations provide `decode_fields` (the input is already length-checked); `decode`, `decode_exact`, `probe`, `encode`, `encode_with` and `encode_cx` are provided methods.
- `Token::len`/`pattern_len` are named `Token::width`/`pattern_width` (clippy `len_without_is_empty`).
- `parse_pattern(&str, PatternKind)` lives in `bryl` for runtime use. Stage 2 must decide how `bryl-derive` shares it (a proc-macro crate cannot depend on `bryl`).
- Encoding a year outside 2000–2099 with `YY` is an error, so a `YY` value always round-trips.

### Stage 2: `bryl-derive` (`Record` and `Code`)
**Goal**: both derives with every attribute in the table above, compile-time offsets and length assertions, constant accessors, and `flatten`.
**Success criteria**: the Stage 1 hand-written fixture is replaced by a derived one with identical behavior, and trybuild UI tests cover every compile error listed.
**Tests**: ports of `TestRecord`, `TestRecordInheritance` (via `flatten`), `TestRecordProbe`, and `TestFieldEnum` (via `Code`), plus constant mismatch on decode (B10) and decode not mutating (B5).
**Status**: Not Started

### Stage 3: `bryl` readers
**Goal**: `Source`, `LineSource`, `BlockSource`, `Dispatch`, `Reader` (peek, next_if, expect, Iterator), and `ReadError`.
**Success criteria**: the Tagged A/B line reader and the block reader from `test_bryl.py` behave as in Python, including the EOF default and raise semantics and the peek/retry behavior.
**Tests**: ports of `TestLineReader`, `TestBlockReader`, and `TestMalformedError`, plus terminator checks (`\n` vs `\r\n`), an unexpected record type leaving the record peeked, and an I/O error surfacing as a `ReadError`.
**Status**: Not Started

### Stage 4: `bryl-nacha`
**Goal**: records, codes, `RoutingNumber`, `Entry`, the typestate writer, the reader, `File::parse`, and `File::validate`, with N1–N13 applied and documented in `DEVIATIONS.md`.
**Success criteria**: every `test_nacha.py` behavior is ported. Golden files (generated by `tools/gen_golden.py` from lms-python, then hand-patched only at the documented deviation columns) match byte for byte, and `File::validate()` on our own output reports no issues.
**Tests**: ports of `TestEnum` (→ `Code`), `TestFileHeader` through `TestFileControl`, `TestTransactionCodeFor`, `TestEntryDetailProperties`, `TestEntry`, `TestWriter*`, `TestReader`, and `TestMalformedError`, plus:
- addenda sequence starting at 1
- trace numbers unique across batches with the same ODFI
- 9-filler padding, with the line count a multiple of 10
- service class 220 rejecting debits
- prenote with nonzero amount rejected
- bad ABA checksum rejected
- SEC addenda limit
- an entry error leaving the output unchanged
- the reader skipping filler lines
**Status**: Not Started

### Stage 5: `bryl-metro2`
**Goal**: records, the 18 code enums, `DataRecord`, validation, the writer, and the reader, with M1–M12 applied (BDW excluded) and documented.
**Success criteria**: every `test_metro2.py` behavior is ported, and the moov-io fixtures (copied into `tests/fixtures/` with their Apache-2.0 NOTICE) parse with the expected values. Writer output for the shared scenarios matches the Python golden files except where `DEVIATIONS.md` says otherwise.
**Tests**: ports of every class in `test_metro2.py`, including the field offsets (also asserted at compile time), the code-table counts (`ALL.len()`), the trailer totals, newline mode, malformed input, and `TestGoTestData*`, plus:
- unknown status code rejected on read and write
- `BaseSegment::validate()` called by the writer
- RDW above 9999 is an error
- correct offsets in reader errors
**Status**: Not Started

### Stage 6: Hardening, docs, stretch goals
**Goal**: production readiness.
**Success criteria**:
- crate-level rustdoc with runnable examples (the module docstrings from `bryl.py`, `metro2.py` and `nacha.py`, rewritten)
- a README per crate
- `cargo-fuzz` targets for `metro2::Reader` and `nacha::Reader`, run for 10 minutes with no crashes; moov-io's `crashers/` corpus is used as seeds
- proptest file-level round-trips (write a random valid file, read it back, compare equal)
- ⚠ items verified against the specs and the decisions recorded
- stretch: Metro 2 BDW (variable-blocked) reading, checked against `unpacked_variable_file.dat`
**Status**: Not Started

When all stages are complete, `implementation.md` is deleted (per the global CLAUDE.md) and `DEVIATIONS.md` stays.

---

## Verification

1. **Per-stage gates**: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && RUSTDOCFLAGS=-D\ warnings cargo doc --workspace --no-deps`.
2. **Behavioral parity**: every Python test has a Rust counterpart. A mapping table (Python test → Rust test, and "adapted because …" where a deviation applies) is kept at the top of each crate's `tests/` directory and reviewed at the end of each stage.
3. **Byte-level parity**: `tools/gen_golden.py` runs inside `lms-python` (`cd backend && python ../../bryl-rs/tools/gen_golden.py`) and writes the NACHA and Metro 2 scenarios used in the tests to `crates/*/tests/fixtures/golden/`. The Rust tests diff their own output against these files, with an explicit allow-list of `(line, column range)` deviations taken from `DEVIATIONS.md`.
4. **External reference data**: the moov-io Metro 2 test data at `~/Web/go-metro2/test/testdata` (header, segments, fixed file, newline request file).
5. **End to end**: `cargo run -p bryl-nacha --example write_file > /tmp/x.ach`, then `cargo run -p bryl-nacha --example validate /tmp/x.ach` should report no issues. Do the same for `metro2` with `examples/{write,read}_file.rs`.
