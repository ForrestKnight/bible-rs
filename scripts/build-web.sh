#!/usr/bin/env bash
# Builds bible-core for the browser: crates/web as WebAssembly, with its JavaScript bindings, into
# site/src/lib/read/wasm/, where the web reader (/read) imports it; and again with the importers,
# for the worker that converts a translation the reader brings. The output is committed so the
# site builds without a Rust toolchain; run this after changing crates/web or what it uses of
# bible-core.
#
#   scripts/build-web.sh
#
# Needs the wasm32-unknown-unknown target and wasm-bindgen-cli at the version Cargo.lock pins for
# wasm-bindgen (the script says which). wasm-opt (binaryen), when installed, shrinks the module.
set -euo pipefail

cd "$(dirname "$0")/.."
OUT=site/src/lib/read/wasm

rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true

# The CLI has to match the crate exactly.
WANT="$(cargo metadata --format-version 1 --locked 2>/dev/null |
  python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "wasm-bindgen"))')"
HAVE="$(wasm-bindgen --version 2>/dev/null | awk '{print $2}' || true)"
if [[ "$HAVE" != "$WANT" ]]; then
  echo "needs wasm-bindgen-cli $WANT (found ${HAVE:-none}): cargo install wasm-bindgen-cli --version $WANT --locked" >&2
  exit 1
fi

# SIMD makes search's substring scans several times faster; every current browser has it.
build() {
  RUSTFLAGS="-C target-feature=+simd128" \
    cargo build --quiet --profile wasm --locked -p bible-web --target wasm32-unknown-unknown "${@:2}"
  wasm-bindgen --target web --out-dir "$OUT" --out-name "$1" \
    target/wasm32-unknown-unknown/wasm/bible_web.wasm
  if command -v wasm-opt >/dev/null; then
    wasm-opt -O3 --enable-simd --enable-bulk-memory --enable-nontrapping-float-to-int \
      --enable-sign-ext --enable-mutable-globals -o "$OUT/$1_bg.wasm" "$OUT/$1_bg.wasm"
  fi
  ls -l "$OUT/$1_bg.wasm" | awk '{printf "%s: %.0f KB\n", $NF, $5 / 1024}'
}

rm -rf "$OUT"
# The reader's module, and the import worker's (the same with the importers).
build bible
build bible_import --features import
