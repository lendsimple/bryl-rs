# Changelog

All notable changes to the bryl crates are recorded here. The crates share
one version. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the crates follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-11-01

This release has breaking changes; see [Changed](#changed). Code enums are
now `#[non_exhaustive]`, so later releases can add codes without another
breaking change.

### Added

#### bryl-nacha

- Notifications of change: COR entries with a type-98 `NocAddendum`
  (`ChangeCode` C01–C09, C13, C14) or `RefusedNocAddendum`
  (`RefusedChangeCode` C61–C69). The writer and `File::validate` check the
  COR rules: a zero amount, a return transaction code, exactly one type-98
  addendum with the entry's trace number, and no type-98 addenda outside
  COR batches.
- Dishonored and contested dishonored returns: type-99
  `DishonoredReturnAddendum` (`DishonoredReturnCode` R61, R62, R67–R70) and
  `ContestedReturnAddendum` (`ContestedReturnCode` R71–R77), written with
  `DishonoredReturnParams` and `ContestedReturnParams`. The reader picks the
  layout from the code. The return rules apply to them, and their two-digit
  reason codes must name known codes.
- Transaction codes for general ledger (41–44, 46–49) and loan (51–56)
  accounts, and the zero-dollar remittance codes (24, 29, 34, 39, 44, 49,
  54), following moov-io/ach:
  - `AccountKind::GeneralLedger` and `AccountKind::Loan`;
  - `TransactionCode::is_general_ledger`, `is_loan` and
    `is_zero_dollar_remittance`;
  - `StandardEntryClass::allows_zero_dollar_remittance`.

  The writer and `File::validate` require zero-dollar remittance entries to
  have a zero amount and an addendum and to appear only in CCD, CTX, ACK
  and ATX batches. Loan debits (55) may appear only in reversal batches.
- Lenient reading: `File::read_lenient`, `Reader::read_file_lenient` and
  `Reader::next_entry_lenient` keep an entry whose detail or addenda fail to
  decode (a bad routing check digit, an unknown code) as an `UnreadEntry`.
  It holds the entry's raw lines, location and error, and `detail_field`
  decodes its intact fields. `File::validate` reports an
  `IssueKind::UnreadEntry` for each. Strict reading is unchanged.
- A `clock` feature, on by default, for `FileParams::now()`. Without it the
  crates build for targets without a system clock, such as
  `wasm32-unknown-unknown`, which CI now checks.

#### bryl-metro2

- More validation, following the 2020 Credit Reporting Resource Guide
  (CRRG); each rule cites its section in the crate docs:
  - The date closed is required for paid accounts (statuses 13 and 61–65)
    and blank for charged-off installment and mortgage accounts
    (`AccountStatus::requires_date_closed`, `validate_date_closed`).
  - The date of first delinquency is reported exactly when the CRRG's Field
    25 hierarchy calls for it: delinquent and derogatory statuses, paid
    accounts that were late, and current accounts with a bankruptcy or
    receivership indicator
    (`BaseSegment::requires_date_of_first_delinquency`,
    `validate_date_of_first_delinquency`).
  - The header's reporter name and address are required
    (`HeaderRecord::validate`). `Writer::begin_file` rejects an invalid
    header with `Error::InvalidHeader`, and `File::validate` reports it as
    `Issue::Header`.
  - Special comment M is not combined with a compliance condition code for
    an account closed at the consumer's request (`validate_closed_codes`).
  - Collection agency, debt buyer and returned check accounts (types 48, 0C
    and 77) report a K1 segment (`AccountType::requires_k1`).

#### All crates

- An optional `serde` feature in `bryl`, `bryl-nacha` and `bryl-metro2`.
  - Files, batches, entries, data records, records and segments serialize
    and deserialize.
  - Code tables serialize as their codes (`"22"`, `"R01"`, `"11"`).
  - `RoutingNumber`, `FileIdModifier` and `PaymentHistoryProfile` serialize
    as their text and are checked again when deserialized.
  - Dates use ISO 8601, and fields fixed by the format are left out.
  - Issues, violations, `UnreadEntry` and `bryl::Error` can be serialized
    but not deserialized.

### Changed

- **Breaking:** the code enums are `#[non_exhaustive]`: in `bryl-nacha`,
  `TransactionCode`, `ReturnReasonCode`, `StandardEntryClass` and
  `ServiceClassCode`; in `bryl-metro2`, all of them. `AccountKind` and
  `NachaRecord` are too. A `match` on them needs a wildcard arm.
- **Breaking:** `Entry::return_addendum` and `EntryParams::return_addendum`
  are replaced by `special_addendum`. It holds the one addendum a return or
  notification of change entry carries in place of type-05 addenda: a
  `SpecialAddendum`, or `SpecialAddendumParams` when writing. Use
  `Entry::return_addendum()` to get a standard return addendum. The builder's
  `special_addendum(..)` accepts a `ReturnParams` directly.
- **Breaking:** `TransactionCode::for_entry` returns `Option`, since a loan
  debit prenote has no code.
- **Breaking:** `nacha::Batch` has a new `unread_entries` field, empty unless
  the file was read leniently.
- **Breaking:** `metro2::AccountType` variants are named after CRRG 2020
  Exhibit 1. Many names were wrong, although the codes were right. For
  example:
  - `0A` is `TimeShareLoan` (it was named `AutoLease`, which is `3A`);
  - `0C` is `DebtBuyer` (it was named `RealEstateJuniorLiens`, which is
    `5A`);
  - 70–75 are the government debts, `GovernmentOverpayment` through
    `GovernmentBenefit`.

  Every variant now documents its CRRG description.
- **Breaking:** `metro2::AccountType` no longer accepts 22 and 23, obsolete
  since September 2011, or 78, which the CRRG does not define.
- **Breaking:** `metro2` writers and `File::validate` enforce the new date,
  header, closed-code and K1 rules above, so records that passed 0.1 may now
  be rejected.
- `TransactionCode::is_credit` covers x1–x4 and `is_debit` x5–x9, so
  zero-dollar remittance codes and the loan debit (55) count toward service
  class checks and control totals.

## [0.1.1] - 2026-10-08

### Changed

- The published packages include their tests, test fixtures and examples,
  and the tests pass from the packages. The `bryl` compile-error snapshot
  test, which follows one rustc's wording, skips outside the repository.
- The `bryl-nacha` and `bryl-metro2` READMEs show crates.io and docs.rs
  badges.

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

[Unreleased]: https://github.com/lendsimple/bryl-rs/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/lendsimple/bryl-rs/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/lendsimple/bryl-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/lendsimple/bryl-rs/releases/tag/v0.1.0
