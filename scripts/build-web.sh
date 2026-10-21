#!/usr/bin/env bash
# Builds bible-core for the browser: crates/web as WebAssembly, with its JavaScript bindings, into
# site/src/lib/read/wasm/, where the web reader (/read) imports it. The output is committed so the
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
RUSTFLAGS="-C target-feature=+simd128" \
  cargo build --quiet --profile wasm --locked -p bible-web --target wasm32-unknown-unknown

rm -rf "$OUT"
wasm-bindgen --target web --out-dir "$OUT" --out-name bible \
  target/wasm32-unknown-unknown/wasm/bible_web.wasm

if command -v wasm-opt >/dev/null; then
  wasm-opt -O3 --enable-simd --enable-bulk-memory --enable-nontrapping-float-to-int \
    --enable-sign-ext --enable-mutable-globals -o "$OUT/bible_bg.wasm" "$OUT/bible_bg.wasm"
fi

ls -l "$OUT/bible_bg.wasm" | awk '{printf "%s: %.0f KB\n", $NF, $5 / 1024}'
