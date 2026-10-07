# Deviations from lms-python

Intentional behavior differences between these crates and
`lms-python/backend/common/{bryl,metro2,nacha}.py`. IDs refer to the behavior
inventory in `implementation.md`.

## bryl

| ID | Python | Rust | Effect on output |
|----|--------|------|------------------|
| B3 | Values are coerced: `"123"` → `123` for Numeric, `5` → `"5"` for Alphanumeric. | No coercion; field types are fixed at compile time. | None (API only). |
| B4 | A global, thread-local `ctx` holds `alpha_upper`/`alpha_filter`/`alpha_truncate`; importing `metro2` or `nacha` turns on uppercasing for every record in the process. | `Sanitize` is set per record (`Record::SANITIZE`), per field, or per encode call. No global state. | Records that don't opt into uppercasing are no longer uppercased just because another module was imported. |
| B5 | `Record.load` re-runs sanitize, so reading uppercases text. | Decoding never changes data. | Decoded text keeps its original case: `"smith"` reads back as `"smith"`, not `"SMITH"`. |
| B6 | Alphanumeric allows `string.printable`, including `\t \n \r \x0b \x0c`. | Printable ASCII only (`0x20..=0x7E`). | Values containing tabs or other control characters are rejected on encode and decode. |
| B8 | Numeric decode uses `int()`, which accepts `" 12"`, `"+12"` and `"-12"`. | After stripping padding, only ASCII digits are accepted. | Such fields are rejected on decode. |
| B10 | Alphanumeric constants are not checked when loading. | All constants and reserved fields are checked on decode. | A record with the wrong constant (e.g. `J2` where `J1` is expected) or non-blank reserved filler fails to decode. |
| B13 | `YY` is parsed by `strptime`: `00`–`68` → 20xx, `69`–`99` → 19xx. `ZZZ` (time zone) is supported. | `YY` always decodes as 20xx, and encoding a year outside 2000–2099 with `YY` is an error. `ZZZ` is not supported. | A `YY` value of `69`–`99` decodes as 2069–2099 instead of 1969–1999. |

### Not deviations, but worth knowing

- `Option<T>` fields treat a blank field (all padding, all spaces, or all zeros for non-alphanumeric kinds) as `None`. For a zero-padded numeric field, `Some(0)` therefore reads back as `None`; use a plain integer when zero is meaningful.
- A left-aligned, zero-padded numeric field strips trailing zeros on decode, exactly as in Python: `420` is written as `42000` and read back as `42`. Avoid `align = left` on zero-padded numerics.
