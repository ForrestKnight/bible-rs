#!/usr/bin/env bash
# Makes the web reader's fonts (static/fonts/read/) from the Mac app's: Literata and Inter, cut to
# the axis ranges the reader uses and the characters the bundled translations and the interface
# need. The output is committed; run this again only when those change.
#
#   site/scripts/fonts.sh
#
# Needs fonttools and brotli: pip3 install --user fonttools brotli
set -euo pipefail

cd "$(dirname "$0")/.."
SRC=../crates/app/assets/fonts
OUT=static/fonts/read
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$OUT"

# Latin with Latin-1 and Extended-A (notes are free text), general punctuation, arrows and the
# keycap symbols (⌘ ⌥ ⇧ ⌃ ⎋ ⌫ ↵), check marks and the few shapes the interface draws.
UNICODES="U+0020-007E,U+00A0-017F,U+0192,U+02BC,U+02C6,U+02DA,U+02DC,U+2000-206F,U+20AC,U+2116,U+2122,U+2190-21FF,U+2212,U+2215,U+2260,U+2264-2265,U+2300-23FF,U+25A0-25FF,U+2713,U+2715"

cut() {
  local src="$1" out="$2"
  shift 2
  python3 -m fontTools.varLib.instancer "$src" "$@" -o "$TMP/instance.ttf" --quiet
  python3 -m fontTools.subset "$TMP/instance.ttf" --unicodes="$UNICODES" --layout-features='*' \
    --flavor=woff2 --output-file="$OUT/$out"
}

# The weights and optical sizes the reader pins (body 400 at opsz 12 up to book titles 300 at 48).
cut "$SRC/Literata.ttf" literata.woff2 wght=300:600 opsz=12:48
cut "$SRC/Literata-Italic.ttf" literata-italic.woff2 wght=300:600 opsz=12:28
# The share card's verse (the Mac's heading cut), static so a canvas can draw it too.
cut "$SRC/Literata.ttf" literata-heading.woff2 wght=400 opsz=28
# Inter at its default optical size, as the Mac app's static instances are.
cut "$SRC/Inter.ttf" inter.woff2 wght=400:700 opsz=14
cp "$SRC/OFL-Literata.txt" "$OUT/OFL-Literata.txt"
cp "$SRC/OFL-Inter.txt" "$OUT/OFL-Inter.txt"

ls -l "$OUT"/*.woff2 | awk '{printf "%s: %.0f KB\n", $NF, $5 / 1024}'
