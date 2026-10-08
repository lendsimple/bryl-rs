//! Compile-time errors from `#[derive(Record)]` and `#[derive(Code)]`.
//!
//! The expected `.stderr` files match the current stable compiler; rustc's
//! diagnostic wording changes between releases, so this test fails on older
//! toolchains such as the 1.85 MSRV (CI only builds there). Regenerate with
//! `TRYBUILD=overwrite cargo test -p bryl --test ui` and review the diff.
//!
//! It only runs in the repository, where `.cargo/config.toml` sets
//! `BRYL_UI_TESTS`. From the published crate (crater, distributions) it skips.

#[test]
fn ui() {
    if std::env::var_os("BRYL_UI_TESTS").is_none() {
        eprintln!("skipping: compile-error snapshots run only in the repository");
        return;
    }
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
