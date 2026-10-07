# Deviations from lms-python

Intentional behavior differences between these crates and
`lms-python/backend/common/{bryl,metro2,nacha}.py`. IDs refer to the behavior
inventory in `implementation.md`.

## bryl

| ID | Python | Rust | Effect on output |
|----|--------|------|------------------|
| B1 | Fields are ordered by a global creation counter; records extend other records by subclassing. | Fields are ordered by declaration; records embed other records with `#[bryl(flatten)]`. | None (API only). |
| B3 | Values are coerced: `"123"` → `123` for Numeric, `5` → `"5"` for Alphanumeric. | No coercion; field types are fixed at compile time. | None (API only). |
| B4 | A global, thread-local `ctx` holds `alpha_upper`/`alpha_filter`/`alpha_truncate`; importing `metro2` or `nacha` turns on uppercasing for every record in the process. | `Sanitize` is set per record (`Record::SANITIZE`), per field, or per encode call. No global state. | Records that don't opt into uppercasing are no longer uppercased just because another module was imported. |
| B5 | `Record.load` re-runs sanitize, so reading uppercases text. | Decoding never changes data. | Decoded text keeps its original case: `"smith"` reads back as `"smith"`, not `"SMITH"`. |
| B6 | Alphanumeric allows `string.printable`, including `\t \n \r \x0b \x0c`. | Printable ASCII only (`0x20..=0x7E`). | Values containing tabs or other control characters are rejected on encode and decode. |
| B8 | Numeric decode uses `int()`, which accepts `" 12"`, `"+12"` and `"-12"`. | After stripping padding, only ASCII digits are accepted. | Such fields are rejected on decode. |
| B10 | Alphanumeric constants are not checked when loading. | All constants and reserved fields are checked on decode. | A record with the wrong constant (e.g. `J2` where `J1` is expected) or non-blank reserved filler fails to decode. |
| B12 | `enum=` takes a list, list of tuples, or dict; values are checked but keys are only attributes. | `#[derive(Code)]` enums. | None for valid data; unknown codes are rejected on encode and decode. |
| B13 | `YY` is parsed by `strptime`: `00`–`68` → 20xx, `69`–`99` → 19xx. `ZZZ` (time zone) is supported. | `YY` always decodes as 20xx, and encoding a year outside 2000–2099 with `YY` is an error. `ZZZ` is not supported. | A `YY` value of `69`–`99` decodes as 2069–2099 instead of 1969–1999. |

### Not deviations, but worth knowing

- `Option<T>` fields treat a blank field (all padding, all spaces, or all zeros for non-alphanumeric kinds) as `None`. For a zero-padded numeric field, `Some(0)` therefore reads back as `None`; use a plain integer when zero is meaningful.
- A left-aligned, zero-padded numeric field strips trailing zeros on decode, exactly as in Python: `420` is written as `42000` and read back as `42`. Avoid `align = left` on zero-padded numerics.

## nacha

Sources: [NACHA ACH developer guide](https://achdevguide.nacha.org/ach-file-details),
[moov-io/ach](https://github.com/moov-io/ach) (`fileHeader.go`, `batch*.go`), and
the published NACHA file specifications of Chase, Regions, First Citizens and
Banc of California. These sources also confirm N1, N2, N3, N5, N6 and N14.

Output differences are verified byte for byte by
`crates/bryl-nacha/tests/golden.rs`, which applies each one (N1, N2, N3,
N14) to files written by `nacha.py`.

| ID | Python | Rust | Effect on output |
|----|--------|------|------------------|
| N1 | Addenda sequence numbers start at `0000`. | Start at `0001`, as the spec requires. | Addenda columns 84–87 are one higher. |
| N2 | The trace sequence restarts at 1 in each batch, so two batches with the same ODFI duplicate trace numbers. | The sequence runs across the whole file. | Trace numbers (and addenda entry sequence numbers) in the second and later batches differ. |
| N3 | No filler records; the block count still counts 10-record blocks. | The file is padded to a multiple of 10 lines with 94 `9`s (`Writer::pad_blocks(false)` turns this off). | Up to 9 extra lines at the end. |
| N4 | Nesting (`begin_file` / `begin_company_batch` / `entry`) is checked at runtime; `begin_company_batch` never checked for an open file. | Typestate guards; misuse does not compile. | None (API only). |
| N5 | A batch's service class code is not checked against its entries. | `220` (credits only) rejects debits; `225` (debits only) rejects credits. | Such entries are rejected instead of written. |
| N6 | Routing numbers are only checked for length; prenote amounts are not checked. | Routing numbers (receiving DFI and immediate destination) must pass the ABA 3-7-1 checksum; prenotes must have a zero amount. | Such values are rejected on write and on read. |
| N7 | `immediate_origin` is left-aligned, so a 9-digit origin is written `123456789 `. | Right-aligned and space-padded, like `immediate_destination`: ` 123456789`. NACHA's developer guide: "the nine-digit routing transit number … preceded by a blank"; moov-io/ach does the same. A 10-character value (e.g. `1` + tax ID) is written unchanged; some banks ask for that. | Differs only for origins shorter than 10 characters. |
| N8 | Any number of addenda per entry. | Type-05 addenda per SEC code. At most one for PPD, CCD and WEB, none for TEL, up to 9,999 for CTX: NACHA's developer guide and bank specifications (First Citizens). From moov-io/ach only: none for ARC, BOC, POP, RCK, TRC, XCK and ADV; none for COR (needs type 98) and MTE, POS, SHR (need type 02), which this library cannot write; exactly one for DNE and ENR (ENR is the weakest: a moov-io comment); up to 9,999 for ATX and TRX; at most one for CIE, ACK, PBR, CBR. | Entries outside the limits are rejected. |
| N9 | `created_at` defaults to `utcnow()`. | Required; `FileParams::now()` is a convenience. | None (API only). |
| N12 | Lines may be longer than 94 characters; filler lines are not recognized. | Lines must be exactly 94 characters; `9…9` filler lines are recognized and skipped. | Malformed lines are rejected on read. |
| N13 | `EntryDetail.mask()` changes the record in place. | `mask()` returns a masked copy. | None (API only). |
| N14 | `receiving_dfi_account_number` is right-aligned. | Left-aligned and space-padded, as the spec requires for alphanumeric fields. | Account number columns 13–29 differ for numbers shorter than 17 characters. Reading a Python-written file keeps the leading spaces (`"        123456789"`); trim them when comparing. |
| N15 | The file ID modifier is any one character. | Must be `A`–`Z` or `0`–`9`. | Other values are rejected. |
| N16 | `StandardEntryClasses` has no BOC (back office conversion). | `StandardEntryClass::Boc` exists, with no addenda. | BOC batches can be written and read. |
| N17 | Any transaction code in any SEC batch. | TEL and the check conversion classes (ARC, BOC, POP, RCK, TRC) are debit-only and CIE is credit-only, except in reversal batches (company entry description `REVERSAL`). From NACHA's developer guide and moov-io/ach; WEB is not restricted (person-to-person WEB credits exist). | Such entries are rejected on write and reported by `File::validate`. |
| N18 | Any company entry description. | ENR batches must be described `AUTOENROLL` and RCK batches `REDEPCHECK` (moov-io/ach). | Such batches are rejected on write (`Error::InvalidBatch`) and reported by `File::validate`. |
| N19 | Batch numbers are not checked when reading. | `File::validate` reports batch numbers that do not ascend (NACHA's developer guide). The writer always numbers batches 1, 2, 3, … | None (reading only). |
| N20 | Lines end with `\n`. | Still `\n` by default; `Writer::line_ending(LineEnding::CrLf)` writes `\r\n`, which some banks require (First Citizens, Banc of California). NACHA does not specify line endings. Reading accepts both. | None by default. |

`File::validate` (new) reports N1, N2 and N3 as issues when reading files
written by `nacha.py`; see `crates/bryl-nacha/tests/reader.rs`.

## metro2

Only one deviation changes the bytes written for a record both libraries
accept: the trailer block count (M6). `crates/bryl-metro2/tests/golden.rs`
applies it to `metro2.py`'s output and checks the rest is byte-identical. The
other differences are in what is accepted (the payment rating rule M8, the
character rules M17, the retired status M18) and in the API.

| ID | Python | Rust | Effect |
|----|--------|------|--------|
| M2 | A required date set to `None` silently writes `00000000`. | Required dates are `NaiveDate`; only optional dates can be `None`. | Compile error instead of a zero date. |
| M3 | Code tables are plain dicts that never validate fields. | Code fields are enums. | Unknown codes are rejected on write (by the type) and on read. |
| M4 | The base segment's record descriptor word is overwritten on a copy when dumping a data record. | Same: `BaseSegment::record_descriptor_word` is read from files, and recomputed when writing a `DataRecord`. | None. (The plan proposed not storing it; it is kept so field offsets match the spec.) |
| M6 | Trailer `block_count = base records + 2`. | The writer reports 0: the CRRG (2020) defines the field as the number of blocks "if applicable", and its fixed-length example trailer reports 0. Other totals are unchanged. `File::validate` checks the block count only for variable-blocked files. | Trailer positions 57–65 are `000000000` instead of records + 2. |
| M7 | `validate_payment_rating`, `validate_amount_past_due` and `validate_payment_history` exist but the writer never calls them. | `Writer` checks each base segment with `BaseSegment::validate` (turn off with `Writer::validate(false)`). | Records breaking these rules are rejected instead of written. |
| M8 | `PAYMENT_RATING_FOR_STATUS` requires a rating per delinquency status (e.g. `0` for 11, `1` for 71, `L` for 97) and allows anything for the others. | A rating (any of `0`–`6`, `G`, `L`) is required for statuses 05, 13, 65, 88, 89, 94 and 95, and must be blank for every other status (`AccountStatus::requires_payment_rating`). Three independent sources agree: moov-io/metro2's validator, Upstart's `metro_2` Ruby gem and The Mortgage Office's Metro 2 documentation. Not yet checked against the CRRG text itself. | With M7, records following Python's table are rejected: e.g. status 11 with rating `0`. Python never enforced its table, so files the LMS writes today may carry either form. |
| M10 | Record type errors always report offset 0. | Errors carry the record's real byte offset (or line number in newline mode). | Better messages. |
| M12 | Variable-blocked files (block descriptor words) are not supported. | Read in both RDW and newline mode: a 4-digit block descriptor word (the block length, counting itself) followed by one or more RDW-prefixed records and optional blank padding, per the CRRG (2020). A file is treated as blocked only if its first unit is a block holding the header, so unblocked files are read exactly as before. `Reader::blocks` / `File::blocks` count the blocks, with a record outside a block counting as one (moov-io's sample blocks only the header). The writer does not write blocked files. | Blocked files can be read. moov-io's `header_record.dat` and `base_segment.dat` still cannot: they have a newline inside the block. |
| M13 | `DataRecord.load` (unlike the reader) silently decodes a truncated segment; a repeated K1–N1 segment overwrites the earlier one. | A truncated segment is always an error; a repeated K1, K2, K3, K4, L1 or N1 is an error. | Malformed records are rejected. |
| M14 | A data record longer than 9999 characters writes a 5-digit record descriptor word. | `Error::RecordTooLong`. | Rejected instead of written corrupt. |
| M15 | The blank payment history code (`" "`) is an entry in `PaymentHistoryCodes`. | It is a space in `PaymentHistoryProfile`, not a `PaymentHistoryCode` variant (codes cannot be blank). | None. |
| M16 | Several account status names are wrong: 61 `VOLUNTARY_SURRENDER`, 62 `MFR_COLLECTED`, 63 `MFR_NOT_COLLECTED`, 64 `FORECLOSURE`, 65 `VOLUNTARY_SURRENDER_ALT`, 88 `CLAIM_FILED`, 94 `GOVT_CLAIM_INSURED`, 95 `GOVT_CLAIM_GUARANTEED`, 96 `GOVT_CLAIM_ADJUSTMENT`, DF `DEFERRED`. | Named for what the codes mean: `PaidVoluntarySurrender`, `PaidCollection`, `PaidRepossession`, `PaidChargeOff`, `PaidForeclosureStarted`, `GovernmentClaimFiled`, `ForeclosureCompleted`, `VoluntarySurrender` (now 95, not 61), `Repossession`, `DeleteAccountFraud`. Each variant's docs give the full description. Sources: Oracle Financial Services Lending & Leasing and The Mortgage Office Metro 2 documentation, Upstart's `metro_2` Ruby gem. | None: codes are unchanged. |
| M17 | Alphanumeric fields allow `string.printable`. | `DataRecord::validate` (and so the writer and `File::validate`) checks CRRG (2020) character rules. Consumer names (base, J1, J2): letters, spaces and hyphens. Address lines and city (base, J2): letters, digits, spaces, slashes, dashes and periods. Consumer account number, identification number, and the L1 replacements: letters and digits only. | Records with e.g. `O'BRIEN`, `APT #4`, `MAIN ST,` or `ACCT-001` are rejected on write (`Writer::validate(false)` turns all checks off). |
| M18 | Account status 05 can be written. | Rejected on write: CDIA retired 05 in April 2022. Report the status at the time of transfer with special comment AT (internal transfer) or O (transferred to another company) instead. Files containing 05 can still be read. | Records with status 05 are rejected on write. |

Sources for M6, M12, M17 and M18: the CDIA Credit Reporting Resource Guide
(2020 edition); [CDIA's announcement retiring status 05](https://www.cdiaonline.org/retirementaccountstatus05/).
Check them against the current edition before relying on them.

Sources for M8 and M16: [moov-io/metro2](https://github.com/moov-io/metro2)
(`pkg/lib/base_segment.go` `ValidatePaymentRating`, `pkg/file/file_instance.go`);
Upstart's [`metro_2` gem](https://github.com/teamupstart/metro_2)
(`lib/metro_2.rb`); [The Mortgage Office](https://help.themortgageoffice.com/knowledge/account-status-codes);
[Oracle Financial Services Lending & Leasing](https://docs.oracle.com/en/industries/financial-services/financial-lending-leasing/14.12.0.0.0/metro-ii-data-preparation-and-reporting/appendix-handling-metro-ii-account-statuses.html).

Reading follows the bryl rules: text keeps its case (B5; moov-io's
`n1_segment.dat` reads as `Employer Name`, which Python returned as
`EMPLOYER NAME`), control characters such as embedded newlines are rejected
(B6), and reserved fields must be blank (B10).
