# bryl-metro2

Metro 2 credit reporting files, character format (`use metro2::...`).

- Header, base segment, J1/J2/K1–K4/L1/N1 segments and trailer, with every
  code table as an enum.
- `DataRecord`: a base segment and its segments; the record descriptor word is
  computed when writing.
- `Writer` validates each base segment and computes the trailer; `Reader`
  reads RDW-framed or newline-delimited files; `File::validate` recomputes
  the trailer and re-checks every account.

Not supported: packed (binary) format and variable-blocked files. Spec decisions
are listed in the repository's `DEVIATIONS.md`.
