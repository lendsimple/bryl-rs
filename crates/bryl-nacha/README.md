# bryl-nacha

NACHA ACH files (`use nacha::...`).

- `Writer` → `FileWriter` → `BatchWriter`: nested guards, so an entry outside
  a batch does not compile. Computes hashes, totals, counts and block count,
  pads to whole blocks, and rejects batches and entries that break NACHA rules
  (service class, SEC code debit/credit and addenda rules, prenote amounts)
  before writing anything. Writes `\n` or `\r\n` line endings.
- `Reader` for structured or record-by-record reading; `File::read` and
  `File::validate` to parse a whole file and check every control total.
- `RoutingNumber` (ABA checksum) and `FileIdModifier` value types.

Spec decisions are listed in the repository's
`DEVIATIONS.md`.
