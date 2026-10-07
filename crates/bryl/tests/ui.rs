//! Compile-time errors from `#[derive(Record)]` and `#[derive(Code)]`.
//!
//! The expected `.stderr` files match the current stable compiler; rustc's
//! diagnostic wording changes between releases, so this test fails on older
//! toolchains such as the 1.85 MSRV (CI only builds there). Regenerate with
//! `TRYBUILD=overwrite cargo test -p bryl --test ui` and review the diff.

#[test]
fn ui() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
