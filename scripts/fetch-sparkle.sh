#!/usr/bin/env bash
# Downloads Sparkle, the framework that updates an installed copy of the app, into
# target/sparkle/<version> (once, checked against the release's SHA-256) and prints that folder.
# It holds Sparkle.framework, which scripts/bundle-macos.sh embeds, and bin/sign_update and
# bin/generate_keys, which scripts/release-macos.sh uses to sign the update and its feed.
set -euo pipefail

VERSION=2.10.0
SHA256=c2bf58aa8387266ac179357b1415d6f2635f044da8be41042af32425dae6da0c
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIR="$ROOT/target/sparkle/$VERSION"

if [[ ! -d "$DIR/Sparkle.framework" || ! -x "$DIR/bin/sign_update" ]]; then
  TMP="$(mktemp -d)"
  trap 'rm -rf "$TMP"' EXIT
  curl -fsSL --retry 3 -o "$TMP/sparkle.tar.xz" \
    "https://github.com/sparkle-project/Sparkle/releases/download/$VERSION/Sparkle-$VERSION.tar.xz"
  got="$(shasum -a 256 "$TMP/sparkle.tar.xz" | cut -d' ' -f1)"
  if [[ "$got" != "$SHA256" ]]; then
    echo "Sparkle-$VERSION.tar.xz has SHA-256 $got, expected $SHA256" >&2
    exit 1
  fi
  mkdir -p "$TMP/x"
  tar -xf "$TMP/sparkle.tar.xz" -C "$TMP/x" ./Sparkle.framework ./bin/sign_update ./bin/generate_keys ./LICENSE
  rm -rf "$DIR"
  mkdir -p "$(dirname "$DIR")"
  mv "$TMP/x" "$DIR"
fi
echo "$DIR"
