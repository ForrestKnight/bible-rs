#!/usr/bin/env bash
# Builds the Linux release archive (LINUX.md). Writes to dist/:
#
#   bible-rs-<version>-linux-<arch>.tar.gz           the binary, desktop entry, icons, install.sh
#   bible-rs-<version>-linux-<arch>-SHA256SUMS.txt
#
#   scripts/package-linux.sh                         # build, then package
#   BIN=path/to/bible-rs scripts/package-linux.sh    # package a binary built already
#
# <arch> is `uname -m`: x86_64 or aarch64. Everything the app needs (translations, fonts) is in
# the binary; at run time it loads the system's Wayland or X11, xkbcommon and OpenGL libraries.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
ARCH="${ARCH:-$(uname -m)}"
DIST="$ROOT/dist"
NAME="bible-rs-$VERSION-linux-$ARCH"
STAGE="$DIST/$NAME"

if [[ -z "${BIN:-}" ]]; then
  cargo build --release --locked -p bible-rs
  BIN="$ROOT/target/release/bible-rs"
fi
[[ -f "$BIN" ]] || { echo "no executable at $BIN" >&2; exit 1; }

rm -rf "$STAGE"
mkdir -p "$STAGE"
install -m755 "$BIN" "$STAGE/bible-rs"
install -m644 packaging/linux/bible-rs.desktop "$STAGE/bible-rs.desktop"
install -m755 packaging/linux/install.sh "$STAGE/install.sh"
cp -R packaging/linux/icons "$STAGE/icons"
install -m644 LICENSE README.md LINUX.md "$STAGE/"

tar -C "$DIST" -czf "$DIST/$NAME.tar.gz" "$NAME"
rm -rf "$STAGE"
(cd "$DIST" && sha256sum "$NAME.tar.gz" > "$NAME-SHA256SUMS.txt")

echo "$DIST/$NAME.tar.gz"
cat "$DIST/$NAME-SHA256SUMS.txt"
