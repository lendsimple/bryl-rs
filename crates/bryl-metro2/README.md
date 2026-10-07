# bryl-metro2

Metro 2 credit reporting files, character format (`use metro2::...`).

- Header, base segment, J1/J2/K1–K4/L1/N1 segments and trailer, with every
  code table as an enum.
- `DataRecord`: a base segment and its segments; the record descriptor word is
  computed when writing.
- `Writer` validates each data record (cross-field rules, CRRG character
  rules, retired codes) and computes the trailer; `Reader` reads RDW-framed,
  newline-delimited and variable-blocked files; `File::validate` recomputes
  the trailer and re-checks every account.

Variable-blocked files (block descriptor words) can be read but not written.
The packed (binary) format is not supported. Spec decisions are listed in the
repository's `DEVIATIONS.md`.
