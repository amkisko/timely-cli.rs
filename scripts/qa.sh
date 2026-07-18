#!/usr/bin/env sh
set -eu

cargo fmt --all -- --check
ruby -c scripts/update-openapi.rb
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
