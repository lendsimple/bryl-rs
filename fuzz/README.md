# Fuzzing

Requires nightly and `cargo install cargo-fuzz`.

```sh
cargo +nightly fuzz run nacha_file corpus/nacha_file seeds/nacha_file
cargo +nightly fuzz run metro2_file corpus/metro2_file seeds/metro2_file
```
