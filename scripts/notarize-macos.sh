#!/usr/bin/env bash
# Submits a .zip, .dmg or .pkg to Apple's notary service and waits for the verdict, using a
# notarytool keychain profile (see RELEASING.md). notarytool exits 0 even when Apple rejects
# an upload, so the status is read from its JSON, and on anything but Accepted Apple's log
# (which names each offending file) is printed before failing.
#
#   scripts/notarize-macos.sh <file>
#   NOTARY_PROFILE=bible-notary   keychain profile name (default bible-notary)
#   NOTARY_KEYCHAIN=<path>        keychain holding the profile (default: the login keychain)
set -euo pipefail

if [[ $# -ne 1 || ! -f "$1" ]]; then
  echo "usage: notarize-macos.sh <file.zip|file.dmg|file.pkg>" >&2
  exit 64
fi

AUTH=(--keychain-profile "${NOTARY_PROFILE:-bible-notary}")
if [[ -n "${NOTARY_KEYCHAIN:-}" ]]; then AUTH+=(--keychain "$NOTARY_KEYCHAIN"); fi

echo "Submitting $(basename "$1") to Apple's notary service (usually a few minutes)…"
OUT="$(xcrun notarytool submit "$1" "${AUTH[@]}" --wait --timeout 45m --output-format json)" || true
JSON="$(grep '^{' <<<"$OUT" | tail -1 || true)"
field() { plutil -extract "$1" raw -o - - <<<"$JSON" 2>/dev/null || true; }
STATUS="$(field status)"
ID="$(field id)"
echo "Notarization of $(basename "$1"): ${STATUS:-no verdict} (${ID:-no submission id})"
if [[ "$STATUS" != "Accepted" ]]; then
  if [[ -z "$JSON" ]]; then echo "$OUT" >&2; fi
  if [[ -n "$ID" ]]; then xcrun notarytool log "$ID" "${AUTH[@]}" >&2 || true; fi
  exit 1
fi
