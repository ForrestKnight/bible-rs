#!/usr/bin/env bash
# Builds the iPhone and iPad app for the App Store into dist/ios/:
#
#   Bible.xcarchive   the release build with its dSYMs (Xcode's Organizer opens it)
#   Bible.ipa         the archive signed for App Store distribution
#
#   scripts/release-ios.sh             # archive and export Bible.ipa
#   scripts/release-ios.sh --upload    # …and upload it to App Store Connect (TestFlight, then
#                                      # App Review); the app's record must exist there first
#   scripts/release-ios.sh --unsigned  # archive without signing, to check the release build
#
#   TEAM_ID        team to sign for, default 5S3KCRYBD2 (Fabrika LLC, as on the Mac)
#   BUILD_NUMBER   CFBundleVersion, default the commit count; each upload needs a higher one
#   ASC_KEY_PATH, ASC_KEY_ID, ASC_ISSUER_ID
#                  an App Store Connect API key (the .p8 file, its key ID and the issuer ID),
#                  to sign and upload without an Apple ID signed in to Xcode. Never commit it.
#   ALLOW_DIRTY=1  build even though the working tree has uncommitted changes
#
# apps/ios/APP_STORE.md covers the one-time setup and the App Store listing. Runs on the bash 3.2
# that ships with macOS.
set -euo pipefail

die() {
  echo "error: $*" >&2
  exit 1
}
step() { printf '\n==> %s\n' "$*"; }

UPLOAD=0
UNSIGNED=0
for arg in "$@"; do
  case "$arg" in
    --upload) UPLOAD=1 ;;
    --unsigned) UNSIGNED=1 ;;
    -h | --help)
      awk 'NR > 1 && /^set / { exit } NR > 1' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) die "unknown option '$arg' (see --help)" ;;
  esac
done
[[ "$UPLOAD" == 1 && "$UNSIGNED" == 1 ]] && die "--upload needs a signed build"

[[ "$(uname -s)" == "Darwin" ]] || die "release-ios.sh must run on macOS with Xcode"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ "$UNSIGNED" == 0 && -z "${ALLOW_DIRTY:-}" && -n "$(git status --porcelain)" ]]; then
  die "the working tree has uncommitted changes; commit them, or set ALLOW_DIRTY=1"
fi

TEAM_ID="${TEAM_ID:-5S3KCRYBD2}"
OUT="dist/ios"
ARCHIVE="$OUT/Bible.xcarchive"
AUTH=()
if [[ -n "${ASC_KEY_PATH:-}" ]]; then
  [[ -f "$ASC_KEY_PATH" && -n "${ASC_KEY_ID:-}" && -n "${ASC_ISSUER_ID:-}" ]] ||
    die "ASC_KEY_PATH needs ASC_KEY_ID and ASC_ISSUER_ID, and must name the .p8 file"
  AUTH=(-authenticationKeyPath "$ASC_KEY_PATH" -authenticationKeyID "$ASC_KEY_ID"
    -authenticationKeyIssuerID "$ASC_ISSUER_ID")
fi

step "Rust core and Xcode project"
"$ROOT/scripts/build-ios.sh" --framework >/dev/null
VERSION="$(sed -n 's/^MARKETING_VERSION = //p' apps/ios/Generated/Version.xcconfig)"
BUILD="$(sed -n 's/^CURRENT_PROJECT_VERSION = //p' apps/ios/Generated/Version.xcconfig)"

rm -rf "$OUT"
mkdir -p "$OUT"
XCODEBUILD=(xcodebuild -quiet -project apps/ios/Bible.xcodeproj -scheme Bible -configuration Release
  -destination "generic/platform=iOS" -derivedDataPath target/ios/DerivedData)

if [[ "$UNSIGNED" == 1 ]]; then
  step "Archive $VERSION ($BUILD), unsigned"
  "${XCODEBUILD[@]}" -archivePath "$ARCHIVE" CODE_SIGNING_ALLOWED=NO archive
else
  step "Archive $VERSION ($BUILD) for team $TEAM_ID"
  "${XCODEBUILD[@]}" -archivePath "$ARCHIVE" DEVELOPMENT_TEAM="$TEAM_ID" \
    -allowProvisioningUpdates ${AUTH[@]+"${AUTH[@]}"} archive
fi

APP="$ARCHIVE/Products/Applications/Bible.app"
[[ -f "$APP/PrivacyInfo.xcprivacy" ]] || die "the archive has no privacy manifest"
if [[ "$UNSIGNED" == 1 ]]; then
  echo "$ARCHIVE"
  exit 0
fi

OPTIONS="target/ios/ExportOptions.plist"
DESTINATION=export
[[ "$UPLOAD" == 1 ]] && DESTINATION=upload
cat > "$OPTIONS" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>method</key><string>app-store-connect</string>
  <key>destination</key><string>$DESTINATION</string>
  <key>teamID</key><string>$TEAM_ID</string>
  <key>signingStyle</key><string>automatic</string>
  <key>uploadSymbols</key><true/>
  <key>manageAppVersionAndBuildNumber</key><false/>
</dict>
</plist>
EOF

if [[ "$UPLOAD" == 1 ]]; then
  step "Sign and upload $VERSION ($BUILD) to App Store Connect"
else
  step "Sign $OUT/Bible.ipa"
fi
xcodebuild -quiet -exportArchive -archivePath "$ARCHIVE" -exportOptionsPlist "$OPTIONS" \
  -exportPath "$OUT" -allowProvisioningUpdates ${AUTH[@]+"${AUTH[@]}"}

if [[ "$UPLOAD" == 1 ]]; then
  echo "Uploaded $VERSION ($BUILD). It shows in App Store Connect under TestFlight once processed."
else
  echo "$OUT/Bible.ipa"
fi
