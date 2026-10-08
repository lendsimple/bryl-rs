#!/usr/bin/env bash
# Runs every crate's tests from the packaged crates, outside the repository, as
# crater and Linux distributions do: everything the tests need must be in the
# package. Extra arguments go to `cargo test`.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
version=$(cargo metadata --manifest-path "$root/Cargo.toml" --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.name == "bryl") | .version')
crates=(bryl bryl-derive bryl-pattern bryl-nacha bryl-metro2)

cargo package --manifest-path "$root/Cargo.toml" --workspace --no-verify --allow-dirty

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
for crate in "${crates[@]}"; do
  tar -xzf "$root/target/package/$crate-$version.crate" -C "$work"
done

# Test the unpacked crates together, each depending on the others' packages,
# as if all of them had been downloaded from crates.io.
{
  echo '[workspace]'
  echo 'resolver = "3"'
  echo 'members = ['
  for crate in "${crates[@]}"; do echo "  \"$crate-$version\","; done
  echo ']'
  echo
  echo '[patch.crates-io]'
  for crate in "${crates[@]}"; do echo "$crate = { path = \"$crate-$version\" }"; done
} > "$work/Cargo.toml"

cd "$work"
cargo test --workspace "$@"
