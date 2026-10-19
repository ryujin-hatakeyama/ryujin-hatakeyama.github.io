#!/bin/sh
set -eu

mkdir -p target/generated target/client
dune exec ./ocaml/generate_updates.exe -- content/updates target/generated/updates.json
npm run build:client
cargo run --locked --release -p site-builder -- build
# The Miscellany search index, made from the built pages and checked
# (scripts/search-index.mjs), then the whole site is checked again with it.
node scripts/search-index.mjs dist
cargo run --locked --release -p site-builder -- check-dist
