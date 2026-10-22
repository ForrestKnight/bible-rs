#!/usr/bin/env bash
# Builds a downloadable macOS release of Bible into dist/ with one command:
#
#   Bible.app                           universal (arm64 + x86_64), Developer ID signed, stapled
#   bible-rs-<version>-universal.dmg    drag-to-Applications disk image, signed, stapled
#   bible-rs-universal.dmg              the same DMG under the name the website links to
#   bible-rs-<version>-universal.zip    the app alone, for direct downloads and Sparkle updates
#   bible-rs-<version>-SHA256SUMS.txt   checksums of the DMGs and the zip
#   appcast.xml                         the update feed installed copies check, signed
#
#   scripts/release-macos.sh                  # sign, notarize, staple: ready to publish
#   scripts/release-macos.sh --skip-notarize  # signed, not notarized: other Macs refuse to
#                                             # open it once downloaded
#   scripts/release-macos.sh --ad-hoc         # no Developer ID (CI without secrets)
#
#   SIGN_IDENTITY    default "Developer ID Application: Fabrika LLC (5S3KCRYBD2)"
#   NOTARY_PROFILE   notarytool keychain profile, default bible-notary
#   NOTARY_KEYCHAIN  keychain holding that profile, default the login keychain
#   SPARKLE_ACCOUNT  login keychain account of the update signing key, default bible-rs
#   ALLOW_DIRTY=1    build even though the working tree has uncommitted changes
#
# RELEASING.md covers the one-time setup and how to publish. Runs on the bash 3.2 that ships
# with macOS.
set -euo pipefail

die() {
  echo "error: $*" >&2
  exit 1
}
step() { printf '\n==> %s\n' "$*"; }

NOTARIZE=1
ADHOC=0
for arg in "$@"; do
  case "$arg" in
    --skip-notarize) NOTARIZE=0 ;;
    --ad-hoc)
      ADHOC=1
      NOTARIZE=0
      ;;
    -h | --help)
      # The header comment, up to `set -euo pipefail`.
      awk 'NR > 1 && /^set / { exit } NR > 1' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) die "unknown option '$arg' (see --help)" ;;
  esac
done

[[ "$(uname -s)" == "Darwin" ]] || die "release-macos.sh must run on macOS"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
MIN_MACOS=11.0
APP="dist/Bible.app"
BIN="$APP/Contents/MacOS/Bible"
NAME="bible-rs-${VERSION}-universal"
DMG="dist/$NAME.dmg"
# simplymybible.com/download redirects to releases/latest/download/bible-rs-universal.dmg,
# so every release needs this unversioned copy too.
LATEST_DMG="dist/bible-rs-universal.dmg"
ZIP="dist/$NAME.zip"
SUMS="dist/bible-rs-${VERSION}-SHA256SUMS.txt"
# Installed copies read releases/latest/download/appcast.xml (SUFeedURL), which points them at
# this release's zip.
APPCAST="dist/appcast.xml"
REPO_URL="https://github.com/michaelshimeles/bible-rs"
SPARKLE_ACCOUNT="${SPARKLE_ACCOUNT:-bible-rs}"
SPARKLE_PUBLIC_KEY="$(sed -n 's/^SPARKLE_PUBLIC_KEY="\(.*\)"$/\1/p' scripts/bundle-macos.sh)"
SIGN_IDENTITY="${SIGN_IDENTITY:-Developer ID Application: Fabrika LLC (5S3KCRYBD2)}"
TEAM_ID="$(sed -n 's/.*(\([A-Z0-9]\{10\}\))$/\1/p' <<<"$SIGN_IDENTITY")"
NOTARY_PROFILE="${NOTARY_PROFILE:-bible-notary}"
TMP="$(cd "$(mktemp -d)" && pwd -P)" # /private/var/…, the path `mount` reports
MOUNT="$TMP/dmg"
cleanup() {
  # A failure while the DMG is attached must not leave it mounted (nor rm a read-only volume).
  if [[ -d "$MOUNT" ]]; then hdiutil detach "$MOUNT" -quiet 2>/dev/null || true; fi
  rm -rf "$TMP"
}
trap cleanup EXIT

export NOTARY_PROFILE
# Never empty: bash 3.2 treats "${empty[@]}" as unbound under set -u.
NOTARY_AUTH=(--keychain-profile "$NOTARY_PROFILE")
if [[ -n "${NOTARY_KEYCHAIN:-}" ]]; then NOTARY_AUTH+=(--keychain "$NOTARY_KEYCHAIN"); fi

# --- Preflight: everything that can fail before a two-minute build -------------------------

step "Preflight (bible-rs $VERSION)"
if [[ -n "$(git status --porcelain 2>/dev/null)" && "${ALLOW_DIRTY:-0}" != "1" ]]; then
  die "the working tree has uncommitted changes; commit them first (or set ALLOW_DIRTY=1)"
fi
command -v xcrun >/dev/null || die "the Xcode command line tools are missing (xcode-select --install)"
command -v rustup >/dev/null || die "rustup is missing (https://rustup.rs)"

if [[ "$ADHOC" == "0" ]]; then
  # Captured first: under pipefail, `grep -q` exiting early could fail the pipeline with SIGPIPE.
  identities="$(security find-identity -v -p codesigning)"
  grep -qF "\"$SIGN_IDENTITY\"" <<<"$identities" ||
    die "the signing identity is not in your keychains: $SIGN_IDENTITY
       Import the Developer ID Application certificate with its private key, or set SIGN_IDENTITY."
  echo "Signing identity: $SIGN_IDENTITY"
else
  echo "Signing: ad-hoc (runs only on this Mac; not for distribution)"
fi

if [[ "$NOTARIZE" == "1" ]]; then
  # `history` reads the profile first, so a missing profile fails here without a network call.
  if ! out="$(xcrun notarytool history "${NOTARY_AUTH[@]}" --output-format json 2>&1)"; then
    if grep -q "No Keychain password item found" <<<"$out"; then
      cat >&2 <<MSG

error: no notarization credentials. The notarytool keychain profile "$NOTARY_PROFILE"
does not exist on this Mac, so nothing was built.

Create it once (notarytool asks for the secret itself and keeps it in your login keychain;
nothing is written to this repository), with an app-specific password from
https://account.apple.com (Sign-In and Security, App-Specific Passwords):

  xcrun notarytool store-credentials $NOTARY_PROFILE --apple-id <apple-id-email> --team-id ${TEAM_ID:-<team-id>}

or with an App Store Connect API key (Users and Access, Integrations, Team Keys):

  xcrun notarytool store-credentials $NOTARY_PROFILE --key AuthKey_<KEYID>.p8 --key-id <KEYID> --issuer <issuer-id>

Then run scripts/release-macos.sh again. To build signed but un-notarized artifacts
meanwhile (for testing on this Mac only):

  scripts/release-macos.sh --skip-notarize

MSG
      exit 2
    fi
    die "notarytool could not use the keychain profile \"$NOTARY_PROFILE\":
$out"
  fi
  echo "Notary profile: $NOTARY_PROFILE"
else
  echo "Notarization: skipped"
fi

SPARKLE="$(scripts/fetch-sparkle.sh)"
echo "Sparkle: $(plutil -extract CFBundleShortVersionString raw "$SPARKLE/Sparkle.framework/Resources/Info.plist")"
if [[ "$NOTARIZE" == "1" ]]; then
  # Installed copies refuse an update signed with any other key than the one in their Info.plist.
  key="$("$SPARKLE/bin/generate_keys" --account "$SPARKLE_ACCOUNT" -p 2>/dev/null || true)"
  [[ -n "$key" ]] || die "no update signing key in the login keychain (account $SPARKLE_ACCOUNT).
       Import the exported key: $SPARKLE/bin/generate_keys --account $SPARKLE_ACCOUNT -f <key-file>
       (RELEASING.md). Never generate a new one: installed copies only accept the existing key."
  [[ "$key" == "$SPARKLE_PUBLIC_KEY" ]] ||
    die "the update signing key in the keychain ($key) is not the one the app trusts ($SPARKLE_PUBLIC_KEY)"
  echo "Update signing key: $key"
fi

# The DMG's Finder layout is written by dmgbuild (pinned with hashes), installed into
# target/dmgbuild with uv, or with pip into a Python 3.10+ venv when uv is missing.
REQS="packaging/macos/dmgbuild-requirements.txt"
VENV="target/dmgbuild"
STAMP="$VENV/requirements.sha256"
want="$(shasum -a 256 "$REQS" | cut -d' ' -f1)"
if [[ ! -x "$VENV/bin/dmgbuild" || "$(cat "$STAMP" 2>/dev/null)" != "$want" ]]; then
  rm -rf "$VENV"
  if command -v uv >/dev/null; then
    uv venv --quiet --python '>=3.10' "$VENV"
    uv pip install --quiet --python "$VENV/bin/python" --require-hashes -r "$REQS"
  else
    PY=""
    for candidate in python3.13 python3.12 python3.11 python3.10 python3; do
      if command -v "$candidate" >/dev/null &&
        "$candidate" -c 'import sys; sys.exit(sys.version_info < (3, 10))' 2>/dev/null; then
        PY="$candidate"
        break
      fi
    done
    [[ -n "$PY" ]] || die "dmgbuild needs uv (https://docs.astral.sh/uv/) or Python 3.10+"
    "$PY" -m venv "$VENV"
    "$VENV/bin/pip" install --quiet --disable-pip-version-check --require-hashes -r "$REQS"
  fi
  echo "$want" >"$STAMP"
fi
echo "dmgbuild: $("$VENV/bin/python" -c 'import importlib.metadata as m; print(m.version("dmgbuild"))')"

# --- Build and check the app -----------------------------------------------------------------

# bundle-macos.sh used to write an unsigned dist/bible-rs-<version>.dmg; don't leave one next
# to the release where it could be uploaded by mistake.
rm -rf "$APP" "$DMG" "$LATEST_DMG" "$ZIP" "$SUMS" "$APPCAST" "dist/bible-rs-${VERSION}.dmg" dist/bible-rs.app

step "Building the universal app"
if [[ "$ADHOC" == "1" ]]; then
  UNIVERSAL=1 SIGN_IDENTITY='' scripts/bundle-macos.sh # ignore an exported identity
else
  UNIVERSAL=1 SIGN_IDENTITY="$SIGN_IDENTITY" scripts/bundle-macos.sh
fi

step "Checking the app"
ARCHS="$(lipo -archs "$BIN")"
echo "Architectures: $ARCHS"
[[ " $ARCHS " == *" arm64 "* && " $ARCHS " == *" x86_64 "* ]] || die "not a universal binary"
for arch in arm64 x86_64; do
  # LC_BUILD_VERSION carries `minos`; older linkers write LC_VERSION_MIN_MACOSX's `version`.
  minos="$(otool -arch "$arch" -l "$BIN" | awk '
    /cmd LC_BUILD_VERSION/ { b = 1 } /cmd LC_VERSION_MIN_MACOSX/ { v = 1 }
    b && $1 == "minos" { print $2; exit } v && $1 == "version" { print $2; exit }')"
  echo "Minimum macOS, $arch: $minos"
  [[ "$minos" == "$MIN_MACOS" ]] || die "the $arch slice targets macOS $minos, expected $MIN_MACOS"
done

PLIST="$APP/Contents/Info.plist"
plist() { plutil -extract "$1" raw -o - "$PLIST"; }
check_plist() {
  local got
  got="$(plist "$1")"
  printf '%-30s %s\n' "$1" "$got"
  [[ "$got" == "$2" ]] || die "Info.plist $1 is '$got', expected '$2'"
}
check_plist CFBundleIdentifier dev.bible-rs.app
check_plist CFBundleName Bible
check_plist CFBundleExecutable Bible
check_plist SUFeedURL "$REPO_URL/releases/latest/download/appcast.xml"
check_plist SUPublicEDKey "$SPARKLE_PUBLIC_KEY"
# Sparkle refuses to start with SURequireSignedFeed unless SUVerifyUpdateBeforeExtraction is on.
check_plist SURequireSignedFeed true
check_plist SUVerifyUpdateBeforeExtraction true
check_plist SUEnableAutomaticChecks true
check_plist CFBundleShortVersionString "$VERSION"
check_plist CFBundleVersion "$VERSION"
check_plist LSMinimumSystemVersion "$MIN_MACOS"
check_plist LSApplicationCategoryType public.app-category.reference
check_plist CFBundleIconFile AppIcon
printf '%-30s %s\n' NSHumanReadableCopyright "$(plist NSHumanReadableCopyright)"
[[ "$(plist NSHumanReadableCopyright)" == Copyright\ ©\ 2* ]] || die "Info.plist has no copyright line"
[[ -s "$APP/Contents/Resources/AppIcon.icns" ]] || die "AppIcon.icns is missing"

codesign --verify --deep --strict --verbose=2 "$APP"
codesign --display --verbose=2 "$APP" >"$TMP/codesign.txt" 2>&1
grep -E '^(Identifier|Format|CodeDirectory|Authority|Timestamp|TeamIdentifier|Runtime Version)' "$TMP/codesign.txt"
grep -q 'flags=.*runtime' "$TMP/codesign.txt" || die "the hardened runtime is not enabled"
if [[ "$ADHOC" == "0" ]]; then
  grep -q "^TeamIdentifier=$TEAM_ID$" "$TMP/codesign.txt" || die "not signed by team $TEAM_ID"
  grep -q '^Timestamp=' "$TMP/codesign.txt" || die "the signature has no secure timestamp"
  # The hardened runtime loads only frameworks signed by the app's own team.
  codesign --display --verbose=2 "$APP/Contents/Frameworks/Sparkle.framework" >"$TMP/sparkle.txt" 2>&1
  grep -q "^TeamIdentifier=$TEAM_ID$" "$TMP/sparkle.txt" || die "Sparkle.framework is not signed by team $TEAM_ID"
fi
echo "Entitlements: $(codesign --display --entitlements - --xml "$APP" 2>/dev/null |
  plutil -convert json -o - - 2>/dev/null || echo '{}')"

# The signed binary must still run: the CLI decodes a bundled Bible and opens the study database.
HOME_DIR="$TMP/home"
"$BIN" --version
native="$(uname -m)"
other=x86_64
if [[ "$native" == "x86_64" ]]; then other=arm64; fi
BIBLE_RS_HOME="$HOME_DIR" "$BIN" read "jn 3:16" >/dev/null || die "the signed app fails to run"
echo "Smoke test, $native: read jn 3:16 ok"
if arch "-$other" /usr/bin/true 2>/dev/null; then
  BIBLE_RS_HOME="$HOME_DIR" arch "-$other" "$BIN" read "jn 3:16" >/dev/null ||
    die "the $other slice fails to run"
  echo "Smoke test, $other: read jn 3:16 ok"
else
  echo "Smoke test, $other: skipped, this Mac can't run it (an Apple silicon Mac needs Rosetta);"
  echo "  the slice's architecture and minimum macOS were checked above"
fi

if [[ "$NOTARIZE" == "1" ]]; then
  step "Notarizing the app"
  ditto -c -k --keepParent "$APP" "$TMP/bible-rs.zip"
  scripts/notarize-macos.sh "$TMP/bible-rs.zip"
  xcrun stapler staple "$APP"
  xcrun stapler validate "$APP"
fi

# --- Disk image, zip and checksums -----------------------------------------------------------

step "Building the disk image"
"$VENV/bin/dmgbuild" -s packaging/macos/dmg-settings.py -D app="$APP" \
  -D background=packaging/macos/dmg-background.png Bible "$DMG"
if [[ "$ADHOC" == "0" ]]; then
  codesign --force --timestamp --sign "$SIGN_IDENTITY" "$DMG"
  codesign --verify --strict --verbose=2 "$DMG"
fi
hdiutil verify -quiet "$DMG"
echo "hdiutil verify: ok"
# Check the app as a user gets it: copied out of the mounted image.
mkdir -p "$MOUNT"
hdiutil attach -readonly -nobrowse -noautoopen -mountpoint "$MOUNT" "$DMG" >/dev/null
[[ "$(readlink "$MOUNT/Applications")" == /Applications ]] || die "the DMG has no Applications link"
ditto "$MOUNT/Bible.app" "$TMP/installed/Bible.app"
hdiutil detach "$MOUNT" -quiet
cmp -s "$BIN" "$TMP/installed/Bible.app/Contents/MacOS/Bible" || die "the DMG holds a different build"
codesign --verify --deep --strict --verbose=2 "$TMP/installed/Bible.app" ||
  die "the app copied out of the DMG fails strict signature checks"
if [[ "$NOTARIZE" == "1" ]]; then
  step "Notarizing the disk image"
  scripts/notarize-macos.sh "$DMG"
  xcrun stapler staple "$DMG"
  xcrun stapler validate "$DMG"
fi

step "Zipping the app"
# ditto keeps the bundle's permissions and extended attributes, and with them the signature
# and stapled ticket; plain zip does not.
ditto -c -k --keepParent "$APP" "$ZIP"
cp -c "$DMG" "$LATEST_DMG" 2>/dev/null || cp "$DMG" "$LATEST_DMG" # -c: APFS clone, no extra space
(cd dist && shasum -a 256 "$(basename "$DMG")" "$(basename "$LATEST_DMG")" "$(basename "$ZIP")") >"$SUMS"

if [[ "$NOTARIZE" == "1" ]]; then
  step "Writing the update feed"
  # The key item trusts generate_keys, which created it; sign_update reading it directly would
  # make the keychain ask. The export lives in $TMP (mode 700) only while signing.
  KEY="$TMP/sparkle-ed25519"
  "$SPARKLE/bin/generate_keys" --account "$SPARKLE_ACCOUNT" -x "$KEY" >/dev/null
  # sign_update prints the enclosure's attributes: sparkle:edSignature="…" sparkle:length="…".
  enclosure="$("$SPARKLE/bin/sign_update" --ed-key-file "$KEY" "$ZIP")"
  [[ "$enclosure" == *'sparkle:edSignature="'*'length="'* ]] || die "sign_update printed '$enclosure'"
  cat >"$APPCAST" <<XML
<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle">
  <channel>
    <title>Bible</title>
    <link>$REPO_URL</link>
    <item>
      <title>Bible $VERSION</title>
      <pubDate>$(LC_ALL=C date -u '+%a, %d %b %Y %H:%M:%S +0000')</pubDate>
      <sparkle:version>$VERSION</sparkle:version>
      <sparkle:shortVersionString>$VERSION</sparkle:shortVersionString>
      <sparkle:minimumSystemVersion>$MIN_MACOS</sparkle:minimumSystemVersion>
      <sparkle:fullReleaseNotesLink>$REPO_URL/releases/tag/v$VERSION</sparkle:fullReleaseNotesLink>
      <enclosure url="$REPO_URL/releases/download/v$VERSION/$(basename "$ZIP")" type="application/octet-stream" $enclosure/>
    </item>
  </channel>
</rss>
XML
  # The app sets SURequireSignedFeed, so the feed carries its own signature as well.
  "$SPARKLE/bin/sign_update" --ed-key-file "$KEY" "$APPCAST"
  "$SPARKLE/bin/sign_update" --ed-key-file "$KEY" --verify "$APPCAST" ||
    die "the update feed's signature does not verify"
  rm -f "$KEY"
  echo "Feed: $APPCAST → $REPO_URL/releases/download/v$VERSION/$(basename "$ZIP")"
fi

# --- Gatekeeper and summary ------------------------------------------------------------------

step "Gatekeeper"
spctl --assess --type execute --verbose=4 "$APP" >"$TMP/spctl-app.txt" 2>&1 || true
spctl --assess --type open --context context:primary-signature --verbose=4 "$DMG" \
  >"$TMP/spctl-dmg.txt" 2>&1 || true
cat "$TMP/spctl-app.txt" "$TMP/spctl-dmg.txt"
if [[ "$NOTARIZE" == "1" ]]; then
  grep -q 'source=Notarized Developer ID' "$TMP/spctl-app.txt" || die "Gatekeeper rejects the app"
  grep -q 'source=Notarized Developer ID' "$TMP/spctl-dmg.txt" || die "Gatekeeper rejects the DMG"
fi

step "Done"
files=("$DMG" "$LATEST_DMG" "$ZIP" "$SUMS")
if [[ -f "$APPCAST" ]]; then files+=("$APPCAST"); fi
for f in "${files[@]}"; do
  bytes="$(stat -f %z "$f")"
  printf '%7.1f MB  %10s bytes  %s\n' "$(bc -l <<<"$bytes / 1048576")" "$bytes" "$ROOT/$f"
done
cat "$SUMS"
if [[ "$NOTARIZE" == "1" ]]; then
  echo "Notarized and stapled: ready to upload (RELEASING.md)."
elif [[ "$ADHOC" == "1" ]]; then
  echo "Ad-hoc signed: runs only on this Mac. Not for distribution."
else
  echo "Signed by $SIGN_IDENTITY but NOT notarized: other Macs will refuse to open it."
  echo "Create the \"$NOTARY_PROFILE\" profile (RELEASING.md), then run scripts/release-macos.sh."
fi
