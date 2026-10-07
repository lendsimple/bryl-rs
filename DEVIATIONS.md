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

Sources for the ⚠ items: [NACHA ACH developer guide](https://achdevguide.nacha.org/ach-file-details),
[moov-io/ach](https://github.com/moov-io/ach) (`fileHeader.go`, `batch*.go`).

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
| N8 | Any number of addenda per entry. | Type-05 addenda per SEC code, following moov-io/ach's batch validators: none for ARC, POP, RCK, TRC, XCK, TEL, ADV; none for COR (needs type 98) and MTE, POS, SHR (need type 02), which this library cannot write; exactly one for DNE and ENR; up to 9,999 for CTX, ATX, TRX; at most one otherwise. ⚠ moov-io's behavior, not the NACHA Operating Rules text. | Entries outside the limits are rejected. |
| N9 | `created_at` defaults to `utcnow()`. | Required; `FileParams::now()` is a convenience. | None (API only). |
| N12 | Lines may be longer than 94 characters; filler lines are not recognized. | Lines must be exactly 94 characters; `9…9` filler lines are recognized and skipped. | Malformed lines are rejected on read. |
| N13 | `EntryDetail.mask()` changes the record in place. | `mask()` returns a masked copy. | None (API only). |
| N14 | `receiving_dfi_account_number` is right-aligned. | Left-aligned and space-padded, as the spec requires for alphanumeric fields. | Account number columns 13–29 differ for numbers shorter than 17 characters. Reading a Python-written file keeps the leading spaces (`"        123456789"`); trim them when comparing. |
| N15 | The file ID modifier is any one character. | Must be `A`–`Z` or `0`–`9`. | Other values are rejected. |

`File::validate` (new) reports N1, N2 and N3 as issues when reading files
written by `nacha.py`; see `crates/bryl-nacha/tests/reader.rs`.

## metro2

No deviation changes the bytes written for a record both libraries accept:
`crates/bryl-metro2/tests/golden.rs` checks that files written by the Rust
writer are byte-identical to `metro2.py`'s output for the same records. The
differences are in what is accepted (most importantly the payment rating
rule, M8) and in the API.

| ID | Python | Rust | Effect |
|----|--------|------|--------|
| M2 | A required date set to `None` silently writes `00000000`. | Required dates are `NaiveDate`; only optional dates can be `None`. | Compile error instead of a zero date. |
| M3 | Code tables are plain dicts that never validate fields. | Code fields are enums. | Unknown codes are rejected on write (by the type) and on read. |
| M4 | The base segment's record descriptor word is overwritten on a copy when dumping a data record. | Same: `BaseSegment::record_descriptor_word` is read from files, and recomputed when writing a `DataRecord`. | None. (The plan proposed not storing it; it is kept so field offsets match the spec.) |
| M6 | Trailer totals, including `block_count = base records + 2`. | Same, and moov-io computes it the same way (`len(f.Bases) + 2`). The CRRG field is "the number of blocks on the file, if applicable", which only differs for blocked files, and those are not supported (M12). | None. |
| M7 | `validate_payment_rating`, `validate_amount_past_due` and `validate_payment_history` exist but the writer never calls them. | `Writer` checks each base segment with `BaseSegment::validate` (turn off with `Writer::validate(false)`). | Records breaking these rules are rejected instead of written. |
| M8 | `PAYMENT_RATING_FOR_STATUS` requires a rating per delinquency status (e.g. `0` for 11, `1` for 71, `L` for 97) and allows anything for the others. | The Credit Reporting Resource Guide rule as implemented by moov-io/metro2: a rating (any of `0`–`6`, `G`, `L`) is required for statuses 05, 13, 65, 88, 89, 94 and 95, and must be blank for every other status (`AccountStatus::requires_payment_rating`). ⚠ Based on moov-io's validator and public CRRG summaries, not the CRRG text. | With M7, records following Python's table are rejected: e.g. status 11 with rating `0`. Python never enforced its table, so files the LMS writes today may carry either form. |
| M10 | Record type errors always report offset 0. | Errors carry the record's real byte offset (or line number in newline mode). | Better messages. |
| M12 | Variable-blocked files (block descriptor words) are not supported. | Still not supported. moov-io treats block descriptor words as part of the packed (binary) format; its one character-format sample has a block descriptor word before the header only, so there is no consistent layout to implement without the CRRG text. | moov-io's `unpacked_variable_file.dat`, `header_record.dat` and `base_segment.dat` cannot be read. |
| M13 | `DataRecord.load` (unlike the reader) silently decodes a truncated segment; a repeated K1–N1 segment overwrites the earlier one. | A truncated segment is always an error; a repeated K1, K2, K3, K4, L1 or N1 is an error. | Malformed records are rejected. |
| M14 | A data record longer than 9999 characters writes a 5-digit record descriptor word. | `Error::RecordTooLong`. | Rejected instead of written corrupt. |
| M15 | The blank payment history code (`" "`) is an entry in `PaymentHistoryCodes`. | It is a space in `PaymentHistoryProfile`, not a `PaymentHistoryCode` variant (codes cannot be blank). | None. |

Sources for M6 and M8: [moov-io/metro2](https://github.com/moov-io/metro2)
(`pkg/lib/base_segment.go` `ValidatePaymentRating`, `pkg/file/file_instance.go`).

Reading follows the bryl rules: text keeps its case (B5; moov-io's
`n1_segment.dat` reads as `Employer Name`, which Python returned as
`EMPLOYER NAME`), control characters such as embedded newlines are rejected
(B6), and reserved fields must be blank (B10).
