#!/usr/bin/env bash
# Builds the iPhone app: the Rust core as apps/ios/Frameworks/BibleFFI.xcframework (device and
# simulator) with its Swift bindings, the Xcode project from apps/ios/project.yml, then the app.
#
#   scripts/build-ios.sh              # simulator build; needs no signing
#   scripts/build-ios.sh --run        # …then install and launch it in a simulator
#   scripts/build-ios.sh --test       # …then run the UI tests in a simulator
#   scripts/build-ios.sh --device     # device build, signed if Config/Local.xcconfig names a team
#   scripts/build-ios.sh --framework  # only the framework and project; then open apps/ios/Bible.xcodeproj
#
#   SIMULATOR="iPhone 17 Pro"  the simulator to use (default: one of its own, "Bible iPhone")
#   TESTS="BibleUITests/testSwitchesTranslation Screenshots"  only these tests
#   CONFIGURATION=Debug        Swift build configuration (default Release; the core is always release)
set -euo pipefail

usage() { sed -n '2,14s/^# \{0,1\}//p' "$0" >&2; }

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "build-ios.sh must run on macOS with Xcode" >&2
  exit 1
fi

RUN=0
TEST=0
DEVICE=0
FRAMEWORK_ONLY=0
for arg in "$@"; do
  case "$arg" in
    --run) RUN=1 ;;
    --test) TEST=1 ;;
    --device) DEVICE=1 ;;
    --framework) FRAMEWORK_ONLY=1 ;;
    -h | --help) usage; exit 0 ;;
    *) usage; exit 1 ;;
  esac
done
if [[ "$DEVICE" == 1 && ("$RUN" == 1 || "$TEST" == 1) ]]; then
  echo "--run and --test use a simulator; install device builds from Xcode" >&2
  exit 1
fi

if ! command -v xcodegen >/dev/null; then
  echo "build-ios.sh needs XcodeGen: brew install xcodegen" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
IOS="apps/ios"
GENERATED="$IOS/Generated"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "workspace version '$VERSION' is not MAJOR.MINOR.PATCH" >&2
  exit 1
fi

# Also reaches the C code cc builds (SQLite, zstd), which would otherwise target the SDK's newest
# iOS.
export IPHONEOS_DEPLOYMENT_TARGET=17.0

# The core as a static library per platform. Apple Silicon simulators only: an Intel Mac would
# need x86_64-apple-ios too, lipo'd into the simulator slice.
TARGETS=(aarch64-apple-ios aarch64-apple-ios-sim)
rustup target add "${TARGETS[@]}" >/dev/null 2>&1 || rustup target add "${TARGETS[@]}"
for target in "${TARGETS[@]}"; do
  cargo rustc --quiet --release -p bible-ffi --lib --target "$target" --crate-type staticlib
done

# The Info.plist tells the App Store the app uses no encryption (ITSAppUsesNonExemptEncryption).
# That holds only while bible-ffi builds bible-core without its "tls" feature.
if cargo tree --quiet -p bible-ffi --target aarch64-apple-ios -e normal --prefix none | grep -q '^rustls '; then
  echo "bible-ffi links rustls now: revisit ITSAppUsesNonExemptEncryption in apps/ios/project.yml" >&2
  exit 1
fi

# Swift bindings, read from the library's own metadata.
mkdir -p "$GENERATED"
BINDINGS="$(mktemp -d)"
trap 'rm -rf "$BINDINGS"' EXIT
cargo run --quiet -p bible-ffi --features bindgen --bin uniffi-bindgen -- \
  generate --library "target/aarch64-apple-ios-sim/release/libbible_ffi.a" \
  --language swift --out-dir "$BINDINGS"
cp "$BINDINGS/bible_ffi.swift" "$GENERATED/BibleFFI.swift"
mkdir -p "$BINDINGS/include"
cp "$BINDINGS/bible_ffiFFI.h" "$BINDINGS/include/"
cp "$BINDINGS/bible_ffiFFI.modulemap" "$BINDINGS/include/module.modulemap"

rm -rf "$IOS/Frameworks/BibleFFI.xcframework"
xcodebuild -quiet -create-xcframework \
  -library target/aarch64-apple-ios/release/libbible_ffi.a -headers "$BINDINGS/include" \
  -library target/aarch64-apple-ios-sim/release/libbible_ffi.a -headers "$BINDINGS/include" \
  -output "$IOS/Frameworks/BibleFFI.xcframework" >/dev/null

BUILD_NUMBER="${BUILD_NUMBER:-$(git rev-list --count HEAD 2>/dev/null || echo 1)}"
cat > "$GENERATED/Version.xcconfig" <<EOF
// Written by scripts/build-ios.sh from Cargo.toml and the commit count.
MARKETING_VERSION = $VERSION
CURRENT_PROJECT_VERSION = $BUILD_NUMBER
EOF

xcodegen generate --quiet --spec "$IOS/project.yml" --project "$IOS"
if [[ "$FRAMEWORK_ONLY" == 1 ]]; then
  echo "$IOS/Bible.xcodeproj"
  exit 0
fi

CONFIGURATION="${CONFIGURATION:-Release}"
DERIVED="target/ios/DerivedData"
XCODEBUILD=(xcodebuild -quiet -project "$IOS/Bible.xcodeproj" -scheme Bible
  -configuration "$CONFIGURATION" -derivedDataPath "$DERIVED")
if [[ "$DEVICE" == 1 ]]; then
  if grep -qs '^ *DEVELOPMENT_TEAM *= *[A-Z0-9]' "$IOS/Config/Local.xcconfig"; then
    "${XCODEBUILD[@]}" -destination "generic/platform=iOS" build
  else
    echo "No DEVELOPMENT_TEAM in $IOS/Config/Local.xcconfig: building unsigned" >&2
    "${XCODEBUILD[@]}" -destination "generic/platform=iOS" CODE_SIGNING_ALLOWED=NO build
  fi
  echo "$DERIVED/Build/Products/$CONFIGURATION-iphoneos/Bible.app"
  exit 0
fi

"${XCODEBUILD[@]}" -destination "generic/platform=iOS Simulator" build
APP="$DERIVED/Build/Products/$CONFIGURATION-iphonesimulator/Bible.app"
echo "$APP"
[[ "$RUN" == 1 || "$TEST" == 1 ]] || exit 0

SIMULATOR="${SIMULATOR:-Bible iPhone}"
udid() {
  xcrun simctl list devices available |
    sed -n "s/^ *$(printf '%s' "$SIMULATOR" | sed 's/[]\/$*.^[]/\\&/g') (\([0-9A-F-]\{36\}\)).*/\1/p" | head -1
}
UDID="$(udid)"
if [[ -z "$UDID" ]]; then
  RUNTIME="$(xcrun simctl list runtimes available | sed -n 's/.*\(com\.apple\.CoreSimulator\.SimRuntime\.iOS-[0-9-]*\).*/\1/p' | tail -1)"
  UDID="$(xcrun simctl create "$SIMULATOR" "${DEVICE_TYPE:-iPhone 17 Pro}" "$RUNTIME")"
fi
xcrun simctl boot "$UDID" 2>/dev/null || true

if [[ "$TEST" == 1 ]]; then
  xcrun simctl bootstatus "$UDID" >/dev/null
  # The same status bar in every run, for the screenshots.
  xcrun simctl status_bar "$UDID" override --time 9:41 --dataNetwork wifi --wifiMode active \
    --wifiBars 3 --cellularMode active --cellularBars 4 --batteryState charged --batteryLevel 100
  trap 'xcrun simctl status_bar "$UDID" clear' EXIT
  ONLY=()
  for test in ${TESTS:-}; do ONLY+=("-only-testing:BibleUITests/$test"); done
  RESULTS="target/ios/Tests.xcresult"
  rm -rf "$RESULTS"
  "${XCODEBUILD[@]}" -destination "id=$UDID" -resultBundlePath "$RESULTS" ${ONLY[@]+"${ONLY[@]}"} test
  echo "$RESULTS"
  exit 0
fi

open -a Simulator --args -CurrentDeviceUDID "$UDID"
xcrun simctl bootstatus "$UDID" >/dev/null
xcrun simctl install "$UDID" "$APP"
xcrun simctl launch "$UDID" "$(/usr/libexec/PlistBuddy -c 'Print CFBundleIdentifier' "$APP/Info.plist")"
