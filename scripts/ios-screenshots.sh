#!/usr/bin/env bash
# Screenshots of the iPhone app, taken in simulators by the Screenshots UI test
# (apps/ios/BibleUITests/Screenshots.swift): light, dark and sepia.
#
#   scripts/ios-screenshots.sh              # docs/screenshots/ios/*.png, 2x, for the README
#   scripts/ios-screenshots.sh --app-store  # target/ios/app-store-screenshots/: full size, in
#                                           # listing order, for a 6.9-inch iPhone and a
#                                           # 13-inch iPad (App Store Connect scales them down
#                                           # for smaller screens)
#
#   SIMULATOR="iPhone 17 Pro" scripts/ios-screenshots.sh  # see scripts/build-ios.sh
set -euo pipefail

APP_STORE=0
for arg in "$@"; do
  case "$arg" in
    --app-store) APP_STORE=1 ;;
    -h | --help)
      awk 'NR > 1 && /^set / { exit } NR > 1' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "unknown option '$arg' (see --help)" >&2
      exit 1
      ;;
  esac
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# The order the App Store listing shows them in.
ORDER="reader-light selection-light search-light psalm-sepia books-light reader-dark psalm-dark
  search-dark selection-dark books-dark psalm-light reader-sepia"

# Runs the test and writes its screenshots to $1: resized to width $2, or full size and
# numbered in listing order if $2 is empty.
capture() {
  local out="$1" width="$2"
  TEST_RUNNER_BIBLE_SCREENSHOTS=1 TESTS=Screenshots "$ROOT/scripts/build-ios.sh" --test >/dev/null
  local attachments
  attachments="$(mktemp -d)"
  xcrun xcresulttool export attachments --path "$ROOT/target/ios/Tests.xcresult" \
    --output-path "$attachments" >/dev/null
  local manifest="$attachments/manifest.json" count i file name n
  mkdir -p "$out"
  count="$(plutil -extract 0.attachments raw "$manifest")"
  for ((i = 0; i < count; i++)); do
    file="$(plutil -extract "0.attachments.$i.exportedFileName" raw "$manifest")"
    # "ios-reader-light_0_<uuid>.png" → reader-light
    name="$(plutil -extract "0.attachments.$i.suggestedHumanReadableName" raw "$manifest")"
    name="${name%%_*}"
    name="${name#ios-}"
    if [[ -n "$width" ]]; then
      sips --resampleWidth "$width" "$attachments/$file" --out "$out/$name.png" >/dev/null
      echo "$out/$name.png"
    else
      n=1
      for wanted in $ORDER; do
        [[ "$wanted" == "$name" ]] && break
        n=$((n + 1))
      done
      name="$(printf '%02d' "$n")-$name"
      cp "$attachments/$file" "$out/$name.png"
      echo "$out/$name.png"
    fi
  done
  rm -rf "$attachments"
}

if [[ "$APP_STORE" == 0 ]]; then
  # 2x; the simulator's 3x originals would triple what the repo carries.
  capture "${OUT:-$ROOT/docs/screenshots/ios}" 804
  exit 0
fi

STORE="$ROOT/target/ios/app-store-screenshots"
rm -rf "$STORE"
SIMULATOR="Bible iPhone 6.9-inch" DEVICE_TYPE="iPhone 17 Pro Max" capture "$STORE/iphone-6.9-inch" ""
SIMULATOR="Bible iPad 13-inch" DEVICE_TYPE="iPad Pro 13-inch (M4)" capture "$STORE/ipad-13-inch" ""
