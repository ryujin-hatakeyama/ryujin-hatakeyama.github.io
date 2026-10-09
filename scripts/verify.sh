#!/bin/sh
set -eu

dune runtest
npm run check:client
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
./scripts/build-site.sh
cargo run --locked --release -p site-builder -- build --output target/determinism-dist
cargo run --locked --release -p site-builder -- compare dist target/determinism-dist
