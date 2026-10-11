#!/bin/sh
# Developer-only offline inference benchmark. Model download requires --download.
set -eu
if [ "$#" -lt 2 ]; then
  echo 'Usage: sh scripts/benchmark.sh MODEL_DIR PCM16_MONO_16KHZ_WAV [LANGUAGE] [--download]' >&2
  exit 1
fi
if ! command -v cargo >/dev/null 2>&1; then
  . "$HOME/.cargo/env"
fi
cargo build --manifest-path src-tauri/Cargo.toml --locked --example benchmark --release
mkdir -p docs/benchmarks
/usr/bin/time -l src-tauri/target/release/examples/benchmark "$@" > docs/benchmarks/latest.json 2> docs/benchmarks/latest-runtime.log
node scripts/benchmark-csv.mjs docs/benchmarks/latest.json
cat docs/benchmarks/latest.json
