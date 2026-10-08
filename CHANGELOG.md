# Changelog

All notable changes to the bryl crates are recorded here. The crates share
one version. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the crates follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- The published packages include their tests, test fixtures and examples.

## [0.1.0] - 2026-10-07

First release.

### bryl

- `#[derive(Record)]` for structs of fixed-width fields: `alpha(N)`,
  `numeric(N)`, `date(..)`, `time(..)`, `datetime(..)`, constants, reserved
  filler and `flatten`. Lengths, offsets, date patterns and field types are
  checked at compile time.
- `#[derive(Code)]` for enums backed by fixed code tables.
- Field codecs for strings, unsigned integers, dates and times,
  constants and optional values; empty optional fields are filled with the
  field's pad byte.
- `Sanitize`: uppercasing, character filtering and truncation, set per
  record, per field or per encode call. Decoding never alters data.
- `bryl::read`: line- and block-framed readers with one record of lookahead.

### bryl-nacha

- `Writer` → `FileWriter` → `BatchWriter` for writing NACHA ACH files. The
  writer computes entry hashes, totals, counts and the block count, pads to
  whole blocks, and rejects batches and entries that break NACHA rules
  before writing anything. Lines end with `\n` or, optionally, `\r\n`.
- Standard entry classes ARC, BOC, CIE, PBR, POP, PPD, RCK, TEL, WEB, CBR,
  CCD, CTX, ACK, ATX, ADV, DNE, ENR, TRC, TRX and XCK, with their
  debit/credit and addenda rules.
- Return entries with a `ReturnAddendum` (addenda type 99), a
  `ReturnReasonCode` and the original entry's trace number.
- `Reader`, `File::read` and `File::validate` for reading files and
  reporting every problem.
- `RoutingNumber` with ABA checksum validation, and
  `TransactionCode::for_entry` to pick a transaction code from a signed
  amount.
- Not supported: type 02 and type 98 addenda (MTE, POS, SHR and COR
  entries), dishonored and contested dishonored returns (R61–R77), and
  international (IAT) entries.

### bryl-metro2

- Metro 2 character-format records: header, base segment, J1, J2, K1–K4,
  L1 and N1 segments, and trailer. Every code table is an enum, so an
  unknown code cannot be written.
- `Writer` validates each record before writing it (payment rating against
  account status, amount past due, payment history, allowed characters,
  the retired status 05) and computes the trailer totals.
- `Reader`, `File::read` and `File::validate` for RDW-framed,
  newline-delimited and variable-blocked files.
- Not supported: the packed (binary) format, and writing blocked files.

### bryl-derive, bryl-pattern

- Internal crates behind `bryl`'s derive macros and date/time patterns,
  re-exported through `bryl`.

[Unreleased]: https://github.com/lendsimple/bryl-rs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/lendsimple/bryl-rs/releases/tag/v0.1.0
