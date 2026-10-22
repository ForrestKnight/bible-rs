#!/bin/sh
# Renders the Ark page's still (static/ark-of-the-covenant/ark.jpg, shown before the 3D loads and
# without WebGL) and its share image (og.jpg, 1200x630) from the model itself, with headless Chrome.
# Start the site first (npm run dev); BASE is its address. macOS only (sips).
set -eu

cd "$(dirname "$0")/.."
chrome=${CHROME:-"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"}
base=${BASE:-http://localhost:5173}
out=static/ark-of-the-covenant
tmp=$(mktemp -d)
trap 'rm -rf "$tmp" scripts/og/og-ark-render.png' EXIT
mkdir -p "$out"

shoot() { # <url> <width,height> <png>
	"$chrome" --headless --hide-scrollbars --use-angle=metal --window-size="$2" --virtual-time-budget=15000 \
		--screenshot="$3" "$1" 2>/dev/null
}

shoot "$base/ark-of-the-covenant/embed?shot" 1600,1000 "$tmp/ark.png"
sips -s format jpeg -s formatOptions 80 "$tmp/ark.png" --out "$out/ark.jpg" >/dev/null

shoot "$base/ark-of-the-covenant/embed?shot=og" 1000,914 scripts/og/og-ark-render.png
# Rendered at 2x and downscaled for crisper text.
"$chrome" --headless --disable-gpu --hide-scrollbars --force-device-scale-factor=2 \
	--window-size=1200,630 --virtual-time-budget=10000 \
	--screenshot="$tmp/og.png" "file://$PWD/scripts/og/ark.html" 2>/dev/null
sips -z 630 1200 -s format jpeg -s formatOptions 85 "$tmp/og.png" --out "$out/og.jpg" >/dev/null
for f in ark og; do printf '%s: ' "$out/$f.jpg"; sips -g pixelWidth -g pixelHeight "$out/$f.jpg" | tail -n 2 | awk '{printf "%s ", $2}'; echo; done
