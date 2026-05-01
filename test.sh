#!/bin/sh
set -e

# native tests
cargo test

# wasm tests
cd dportable
wasm-pack test --firefox --headless
cd ..
