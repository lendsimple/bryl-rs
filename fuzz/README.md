# Fuzzing

Requires nightly and `cargo install cargo-fuzz`.

```sh
cargo +nightly fuzz run nacha_file corpus/nacha_file seeds/nacha_file
cargo +nightly fuzz run metro2_file corpus/metro2_file seeds/metro2_file
```

`seeds/` holds the snapshot test files plus, for Metro 2, reference and
crasher files from [moov-io/metro2](https://github.com/moov-io/metro2)
(the `.dat` segment and file samples and `moov_crasher_*`), used under the
Apache License 2.0 in `MOOV-LICENSE`.
