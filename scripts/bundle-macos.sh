#!/usr/bin/env bash
# Builds dist/Bible.app on macOS. scripts/release-macos.sh wraps it to make the signed,
# notarized DMG and zip (see RELEASING.md).
#
#   scripts/bundle-macos.sh              # native architecture, ad-hoc signature
#   UNIVERSAL=1 scripts/bundle-macos.sh  # arm64 + x86_64 universal binary
#   SIGN_IDENTITY="Developer ID Application: …" scripts/bundle-macos.sh  # timestamped signature
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "bundle-macos.sh must run on macOS (needs sips, iconutil, lipo, codesign)" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  # CFBundleShortVersionString and CFBundleVersion take at most three integers.
  echo "workspace version '$VERSION' is not MAJOR.MINOR.PATCH" >&2
  exit 1
fi
COPYRIGHT="$(sed -n 's/^Copyright (c) /Copyright © /p' LICENSE | head -1)"
APP="dist/Bible.app"
ENTITLEMENTS="packaging/macos/entitlements.plist"
# Sparkle checks this feed for updates and installs only archives signed with the private half
# of this key, which `generate_keys --account bible-rs` keeps in the release Mac's login keychain
# (RELEASING.md). Every release must be signed with the same key.
FEED_URL="https://github.com/michaelshimeles/bible-rs/releases/latest/download/appcast.xml"
SPARKLE_PUBLIC_KEY="Pq8ULcnRQOe3FxC6l7qBrhZ4VS3W53yxzZeO9z9dmY0="
SPARKLE="$(scripts/fetch-sparkle.sh)"

# The window code calls NSWindow.toolbarStyle, new in macOS 11, and the x86_64 target would
# otherwise default to 10.12. This also reaches the C code that cc builds (SQLite, zstd).
export MACOSX_DEPLOYMENT_TARGET=11.0

if [[ "${UNIVERSAL:-0}" == "1" ]]; then
  rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
  cargo build --release -p bible-rs --target aarch64-apple-darwin --target x86_64-apple-darwin
  BIN="$(mktemp -d)/bible-rs"
  lipo -create -output "$BIN" \
    target/aarch64-apple-darwin/release/bible-rs \
    target/x86_64-apple-darwin/release/bible-rs
else
  cargo build --release -p bible-rs
  BIN="target/release/bible-rs"
fi

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$APP/Contents/Frameworks"
cp "$BIN" "$APP/Contents/MacOS/Bible"

# Sparkle, loaded once the first frame is up (src/updater.rs). The app isn't sandboxed, so
# Sparkle's XPC services aren't needed; headers and modules are build-time only.
FRAMEWORK="$APP/Contents/Frameworks/Sparkle.framework"
ditto "$SPARKLE/Sparkle.framework" "$FRAMEWORK"
for part in XPCServices Headers PrivateHeaders Modules; do
  rm -rf "${FRAMEWORK:?}/$part" "${FRAMEWORK:?}/Versions/B/$part"
done

# App icon from the 1024 px PNG.
ICONSET="$(mktemp -d)/AppIcon.iconset"
mkdir -p "$ICONSET"
SRC="crates/app/assets/icon.png"
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" "$SRC" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  sips -z "$double" "$double" "$SRC" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"

# The bundle ID is part of the signature's designated requirement, which the Keychain checks
# before handing stored API keys back: changing it after a release makes every user approve
# access again.
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Bible</string>
  <key>CFBundleDisplayName</key><string>Bible</string>
  <key>CFBundleIdentifier</key><string>dev.bible-rs.app</string>
  <key>CFBundleExecutable</key><string>Bible</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>LSMinimumSystemVersion</key><string>${MACOSX_DEPLOYMENT_TARGET}</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.reference</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSSupportsAutomaticGraphicsSwitching</key><true/>
  <key>NSRequiresAquaSystemAppearance</key><false/>
  <key>NSHumanReadableCopyright</key><string>${COPYRIGHT}. MIT License. Bundled translations are public domain.</string>
  <key>SUFeedURL</key><string>${FEED_URL}</string>
  <key>SUPublicEDKey</key><string>${SPARKLE_PUBLIC_KEY}</string>
  <key>SURequireSignedFeed</key><true/>
  <key>SUVerifyUpdateBeforeExtraction</key><true/>
  <key>SUEnableAutomaticChecks</key><true/>
</dict>
</plist>
PLIST
plutil -lint "$APP/Contents/Info.plist" >/dev/null

# Hardened runtime either way. An ad-hoc signature (required on Apple silicon) runs only on the
# Mac that built it; a Developer ID signature gets a secure timestamp, which notarization needs.
# Sparkle's helpers are signed inside out before the app; the framework must carry the app's
# team, or library validation stops the hardened runtime from loading it.
if [[ -n "${SIGN_IDENTITY:-}" ]]; then
  SIGN=(codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY")
else
  SIGN=(codesign --force --options runtime --sign -)
fi
"${SIGN[@]}" "$FRAMEWORK/Versions/B/Autoupdate"
"${SIGN[@]}" "$FRAMEWORK/Versions/B/Updater.app"
"${SIGN[@]}" "$FRAMEWORK"
"${SIGN[@]}" --entitlements "$ENTITLEMENTS" "$APP"

echo "Built $APP ($VERSION, $(lipo -archs "$APP/Contents/MacOS/Bible"))"
