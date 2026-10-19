#!/bin/sh
set -eu

# The Updates pages depend on the build day (Asia/Tokyo); fix it once so that
# the two builds compared below describe the same day even across midnight.
SITE_BUILD_DATE=${SITE_BUILD_DATE:-$(TZ=Asia/Tokyo date +%Y-%m-%d)}
export SITE_BUILD_DATE

dune runtest
npm run check:client
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
./scripts/build-site.sh
cargo run --locked --release -p site-builder -- build --output target/determinism-dist
node scripts/search-index.mjs target/determinism-dist
cargo run --locked --release -p site-builder -- compare dist target/determinism-dist
node scripts/test-search-index.mjs
./scripts/test-update-phases.sh
