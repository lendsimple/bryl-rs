# Changelog

All notable changes to the bryl crates are recorded here. The crates share
one version. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the crates follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- An optional `serde` feature in `bryl`, `bryl-nacha` and `bryl-metro2`.
  Files, batches, entries, data records, records and segments serialize and
  deserialize; code tables as their codes (`"22"`, `"R01"`, `"11"`);
  `RoutingNumber`, `FileIdModifier` and `PaymentHistoryProfile` as their
  text, checked again when deserialized; dates as ISO 8601. Fields fixed
  by the format are left out. Issues, violations, `UnreadEntry` and
  `bryl::Error` can be serialized but not deserialized.

- `bryl-metro2` checks more CRRG 2020 rules: the header's reporter name
  and address are required (`HeaderRecord::validate`, checked by
  `Writer::begin_file` and reported by `File::validate` as `Issue::Header`);
  special comment M is not combined with a "closed at the consumer's
  request" compliance condition code (`validate_closed_codes`); and
  collection agency, debt buyer and returned check accounts (types 48, 0C,
  77) report a K1 segment (`AccountType::requires_k1`).
- `bryl-metro2` checks the date closed and the date of first delinquency
  (DOFD) against the 2020 CRRG. Paid accounts (statuses 13 and 61-65)
  report a date closed; charged-off installment and mortgage accounts don't.
  The DOFD is reported exactly when the CRRG's Field 25 hierarchy calls for
  it: delinquent and derogatory statuses, paid accounts that were late, and
  current accounts with a bankruptcy or receivership indicator
  (`AccountStatus::requires_date_closed`,
  `BaseSegment::requires_date_of_first_delinquency`, `validate_date_closed`,
  `validate_date_of_first_delinquency`).
- `bryl-nacha` lenient reading: `File::read_lenient`,
  `Reader::read_file_lenient` and `Reader::next_entry_lenient` keep an entry
  whose detail or addenda fail to decode (a bad routing check digit, an
  unknown code) as an `UnreadEntry`, with its raw lines, location and error,
  and `detail_field` to decode its intact fields. `File::validate` reports
  `IssueKind::UnreadEntry` for each. Strict reading is unchanged.
- `bryl-nacha` reads and writes dishonored and contested dishonored returns:
  type-99 `DishonoredReturnAddendum` (`DishonoredReturnCode` R61, R62,
  R67–R70) and `ContestedReturnAddendum` (`ContestedReturnCode` R71–R77),
  as `SpecialAddendum` variants written with `DishonoredReturnParams` and
  `ContestedReturnParams`. The reader picks the layout from the code. The
  writer and `File::validate` apply the return rules to them and check that
  their two-digit reason codes name known codes.
- `bryl-nacha` reads and writes notifications of change: COR entries with a
  type-98 `NocAddendum` (`ChangeCode` C01–C09, C13, C14) or
  `RefusedNocAddendum` (`RefusedChangeCode` C61–C69), held in
  `Entry::special_addendum` like return addenda. The writer and
  `File::validate` check the COR rules: a zero amount, a return transaction
  code, exactly one type-98 addendum with the entry's trace number, and no
  type-98 addenda outside COR batches.
- `bryl-nacha` transaction codes for general ledger (41–44, 46–49) and loan
  (51–56) accounts, and the zero-dollar remittance codes (24, 29, 34, 39, 44,
  49, 54), following moov-io/ach. `AccountKind` gains `GeneralLedger` and
  `Loan`; `TransactionCode` gains `is_general_ledger`, `is_loan` and
  `is_zero_dollar_remittance`; and
  `StandardEntryClass::allows_zero_dollar_remittance` says which classes may
  carry them.
- The writer and `File::validate` enforce the rules for the new codes:
  zero-dollar remittance entries have a zero amount and an addendum and
  appear only in CCD, CTX, ACK and ATX batches, and loan debits (55) appear
  only in reversal batches.
- `bryl-nacha` has a `clock` feature, on by default, for `FileParams::now()`.
  Without it the crates build for targets without a system clock, such as
  `wasm32-unknown-unknown`, which CI now checks.

### Changed

- **Breaking:** `metro2::AccountType` variants are named after CRRG 2020
  Exhibit 1. Many were wrong: for example `0A` is `TimeShareLoan` (it was
  `AutoLease`, which is `3A`), `0C` is `DebtBuyer` (it was
  `RealEstateJuniorLiens`, which is `5A`), and 70-75 are the government
  debts (`GovernmentOverpayment` through `GovernmentBenefit`). Every
  variant now documents its CRRG description.
- **Breaking:** account types 22 and 23, obsolete since September 2011, and
  78, which the CRRG does not define, are no longer accepted.
- **Breaking:** `nacha::Batch` has an `unread_entries` field, empty unless
  the file was read leniently.
- **Breaking:** `Entry::return_addendum` and `EntryParams::return_addendum`
  are replaced by `special_addendum`, holding a `SpecialAddendum` (or
  `SpecialAddendumParams`): the one addendum a return entry carries in place
  of type-05 addenda. `Entry::return_addendum()` returns the return addendum,
  and `EntryParams::builder().special_addendum(..)` accepts a `ReturnParams`.
- **Breaking:** `TransactionCode::for_entry` returns `Option`: there is no
  loan debit prenote.
- `TransactionCode::is_credit` covers x1–x4 and `is_debit` x5–x9, so
  zero-dollar remittance codes and the loan debit count toward service
  class checks and control totals.
- **Breaking:** the code enums of `bryl-nacha` (`TransactionCode`,
  `ReturnReasonCode`, `StandardEntryClass`, `ServiceClassCode`) and
  `bryl-metro2`, as well as `AccountKind` and `NachaRecord`, are
  `#[non_exhaustive]`, so later releases can add codes and record types
  without a breaking change. A `match` on them needs a wildcard arm.

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

[Unreleased]: https://github.com/lendsimple/bryl-rs/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/lendsimple/bryl-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/lendsimple/bryl-rs/releases/tag/v0.1.0
