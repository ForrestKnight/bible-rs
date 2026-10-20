#!/usr/bin/env bash
# Regenerates docs/screenshots/ios/*.png by driving the iPhone app in a simulator through the
# Screenshots UI test (apps/ios/BibleUITests/Screenshots.swift): light, dark and sepia.
#
#   scripts/ios-screenshots.sh
#   SIMULATOR="iPhone 17 Pro" scripts/ios-screenshots.sh  # see scripts/build-ios.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${OUT:-$ROOT/docs/screenshots/ios}"
# 2x; the simulator's 3x originals would triple what the repo carries.
WIDTH=804

TEST_RUNNER_BIBLE_SCREENSHOTS=1 TESTS=Screenshots "$ROOT/scripts/build-ios.sh" --test >/dev/null

ATTACHMENTS="$(mktemp -d)"
trap 'rm -rf "$ATTACHMENTS"' EXIT
xcrun xcresulttool export attachments --path "$ROOT/target/ios/Tests.xcresult" \
  --output-path "$ATTACHMENTS" >/dev/null
MANIFEST="$ATTACHMENTS/manifest.json"
mkdir -p "$OUT"
count="$(plutil -extract 0.attachments raw "$MANIFEST")"
for ((i = 0; i < count; i++)); do
  file="$(plutil -extract "0.attachments.$i.exportedFileName" raw "$MANIFEST")"
  # "ios-reader-light_0_<uuid>.png" → reader-light.png
  name="$(plutil -extract "0.attachments.$i.suggestedHumanReadableName" raw "$MANIFEST")"
  name="${name%%_*}"
  name="${name#ios-}.png"
  sips --resampleWidth "$WIDTH" "$ATTACHMENTS/$file" --out "$OUT/$name" >/dev/null
  echo "$OUT/$name"
done
