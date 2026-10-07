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
| N7 | `immediate_origin` is left-aligned, so a 9-digit origin is written `123456789 `. | Right-aligned and space-padded, like `immediate_destination`: ` 123456789`. ⚠ Confirm with the receiving bank. | Differs only for origins shorter than 10 characters. |
| N8 | Any number of addenda per entry. | Limited by SEC code: 0 for ARC, POP, RCK, TEL, XCK; up to 9,999 for CTX, ATX, TRX; 1 otherwise. ⚠ Verify against the NACHA Operating Rules. | Entries over the limit are rejected. |
| N9 | `created_at` defaults to `utcnow()`. | Required; `FileParams::now()` is a convenience. | None (API only). |
| N12 | Lines may be longer than 94 characters; filler lines are not recognized. | Lines must be exactly 94 characters; `9…9` filler lines are recognized and skipped. | Malformed lines are rejected on read. |
| N13 | `EntryDetail.mask()` changes the record in place. | `mask()` returns a masked copy. | None (API only). |
| N14 | `receiving_dfi_account_number` is right-aligned. | Left-aligned and space-padded, as the spec requires for alphanumeric fields. | Account number columns 13–29 differ for numbers shorter than 17 characters. Reading a Python-written file keeps the leading spaces (`"        123456789"`); trim them when comparing. |
| N15 | The file ID modifier is any one character. | Must be `A`–`Z` or `0`–`9`. | Other values are rejected. |

`File::validate` (new) reports N1, N2 and N3 as issues when reading files
written by `nacha.py`; see `crates/bryl-nacha/tests/reader.rs`.
