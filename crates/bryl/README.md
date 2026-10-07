# bryl

Declarative fixed-width records: typed fields at fixed offsets, encoded to
and decoded from printable ASCII.

- `#[derive(Record)]` on a struct with `#[bryl(alpha(N))]`, `numeric(N)`,
  `date("MMDDYYYY")`, `time("hhmm")`, `datetime(...)` or `flatten` fields.
- `#[derive(Code)]` on an enum for fixed code tables.
- `bryl::read`: line- and block-framed readers with one record of lookahead.
- No global state: uppercasing, filtering and truncation are per record, per
  field, or per encode call (`Sanitize`).

See the crate documentation for the full attribute list.
