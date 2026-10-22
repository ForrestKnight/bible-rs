#!/usr/bin/env bash
# Builds the Windows installer (WINDOWS.md). Writes to dist/:
#
#   bible-rs-<version>-windows-x64-setup.exe     per-user installer (NSIS)
#   bible-rs-<version>-windows-x64-SHA256SUMS.txt
#
#   scripts/package-windows.sh                  # on Windows (Git Bash): build, then package
#   BIN=path/to/bible-rs.exe scripts/package-windows.sh   # package an executable built already
#
# Off Windows it cross-builds with cargo-xwin (`cargo install cargo-xwin`, plus LLVM for llvm-rc
# and lld-link) and packages with makensis (`brew install makensis`).
#
# Signing: with WINDOWS_SIGN_COMMAND set, scripts/sign-windows.sh signs bible-rs.exe, the
# uninstaller and the installer. Without it everything is unsigned, as CI builds are.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
TARGET="${TARGET:-x86_64-pc-windows-msvc}"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
COPYRIGHT="$(grep -m1 '^Copyright' LICENSE)"
DIST="$ROOT/dist"
STAGE="$DIST/windows"
SETUP="$DIST/bible-rs-$VERSION-windows-x64-setup.exe"
SUMS="$DIST/bible-rs-$VERSION-windows-x64-SHA256SUMS.txt"

case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*) on_windows=1 ;;
  *) on_windows=0 ;;
esac
# makensis wants native paths on Windows.
native() { if [[ $on_windows == 1 ]]; then cygpath -w "$1"; else echo "$1"; fi; }

if [[ -z "${BIN:-}" ]]; then
  if [[ $on_windows == 1 ]]; then
    cargo build --release --locked -p bible-rs --target "$TARGET"
  else
    command -v cargo-xwin >/dev/null || { echo "cargo-xwin is needed off Windows (cargo install cargo-xwin)" >&2; exit 2; }
    cargo xwin build --release --locked -p bible-rs --target "$TARGET"
  fi
  BIN="$ROOT/target/$TARGET/release/bible-rs.exe"
fi
[[ -f "$BIN" ]] || { echo "no executable at $BIN" >&2; exit 1; }

MAKENSIS="${MAKENSIS:-$(command -v makensis || true)}"
if [[ -z "$MAKENSIS" && -x "/c/Program Files (x86)/NSIS/makensis.exe" ]]; then
  MAKENSIS="/c/Program Files (x86)/NSIS/makensis.exe"
fi
[[ -n "$MAKENSIS" ]] || { echo "makensis (NSIS 3) is needed: choco install nsis, or brew install makensis" >&2; exit 2; }

# A fresh staging folder: the executable and the licences the installer copies.
rm -rf "$STAGE"
mkdir -p "$STAGE"
cp "$BIN" "$STAGE/bible-rs.exe"
cp LICENSE "$STAGE/LICENSE.txt"
cp crates/app/assets/fonts/OFL-Inter.txt crates/app/assets/fonts/OFL-Literata.txt "$STAGE/"

defines=(-DVERSION="$VERSION" -DCOPYRIGHT="$COPYRIGHT" -DSRC="$(native "$STAGE")" -DOUT="$(native "$SETUP")")
if [[ -n "${WINDOWS_SIGN_COMMAND:-}" ]]; then
  scripts/sign-windows.sh "$STAGE/bible-rs.exe"
  # makensis runs this script on the uninstaller and the installer it writes, with this bash
  # (on Windows a bare `bash` can be WSL's).
  defines+=(-DSIGN="$(native "$ROOT/scripts/sign-windows.sh")" -DSIGN_SHELL="$(native "$(command -v bash)")")
else
  echo "WINDOWS_SIGN_COMMAND is not set: the app and its installer are unsigned"
fi

rm -f "$SETUP"
(cd packaging/windows && "$MAKENSIS" -V2 -INPUTCHARSET UTF8 "${defines[@]}" installer.nsi)
[[ -f "$SETUP" ]] || { echo "makensis wrote no installer" >&2; exit 1; }

(cd "$DIST" && if command -v sha256sum >/dev/null; then sha256sum "$(basename "$SETUP")"; else shasum -a 256 "$(basename "$SETUP")"; fi) > "$SUMS"
echo "Wrote:"
ls -l "$SETUP" "$SUMS"
cat "$SUMS"
