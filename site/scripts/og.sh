#!/bin/sh
# Renders scripts/og/og.html to static/og.jpg (1200x630) with headless Chrome. macOS only (sips).
set -eu

cd "$(dirname "$0")/.."
chrome=${CHROME:-"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# Rendered at 2x and downscaled for crisper text; the time budget lets the web fonts load.
"$chrome" --headless --disable-gpu --hide-scrollbars --force-device-scale-factor=2 \
	--window-size=1200,630 --virtual-time-budget=10000 \
	--screenshot="$tmp/og.png" "file://$PWD/scripts/og/og.html" 2>/dev/null
sips -z 630 1200 -s format jpeg -s formatOptions 85 "$tmp/og.png" --out static/og.jpg >/dev/null
sips -g pixelWidth -g pixelHeight static/og.jpg | tail -n 2
