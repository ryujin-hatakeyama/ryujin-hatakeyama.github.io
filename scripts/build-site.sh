#!/bin/sh
set -eu

mkdir -p target/generated target/client
dune exec ./ocaml/generate_updates.exe -- content/updates target/generated/updates.json
npm run build:client
cargo run --locked --release -p site-builder -- build
