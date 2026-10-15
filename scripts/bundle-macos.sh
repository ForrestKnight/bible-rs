#!/usr/bin/env bash
# Builds dist/bible-rs.app and dist/bible-rs-<version>.dmg on macOS.
#
#   scripts/bundle-macos.sh              # native architecture
#   UNIVERSAL=1 scripts/bundle-macos.sh  # arm64 + x86_64 universal binary
#   SIGN_IDENTITY="Developer ID Application: …" scripts/bundle-macos.sh
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "bundle-macos.sh must run on macOS (needs sips, iconutil, codesign, hdiutil)" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
APP="dist/bible-rs.app"

if [[ "${UNIVERSAL:-0}" == "1" ]]; then
  rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
  cargo build --release -p bible-rs --target aarch64-apple-darwin
  cargo build --release -p bible-rs --target x86_64-apple-darwin
  BIN="$(mktemp -d)/bible-rs"
  lipo -create -output "$BIN" \
    target/aarch64-apple-darwin/release/bible-rs \
    target/x86_64-apple-darwin/release/bible-rs
else
  cargo build --release -p bible-rs
  BIN="target/release/bible-rs"
fi

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/bible-rs"

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

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>bible-rs</string>
  <key>CFBundleDisplayName</key><string>bible-rs</string>
  <key>CFBundleIdentifier</key><string>dev.bible-rs.app</string>
  <key>CFBundleExecutable</key><string>bible-rs</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.reference</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSSupportsAutomaticGraphicsSwitching</key><true/>
  <key>NSRequiresAquaSystemAppearance</key><false/>
  <key>NSHumanReadableCopyright</key><string>bible-rs is MIT licensed. Bundled translations are public domain.</string>
</dict>
</plist>
PLIST

# Ad-hoc signature by default (required on Apple silicon); pass SIGN_IDENTITY for a real one.
codesign --force --deep --options runtime --sign "${SIGN_IDENTITY:--}" "$APP"

DMG="dist/bible-rs-${VERSION}.dmg"
rm -f "$DMG"
STAGE="$(mktemp -d)"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
hdiutil create -volname "bible-rs" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null

echo "Built $APP and $DMG"
