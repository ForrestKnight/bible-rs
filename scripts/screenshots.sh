#!/usr/bin/env bash
# Regenerates docs/screenshots/*.png by driving the release build under Xvfb with xdotool.
#
#   scripts/screenshots.sh              # every scene
#   scripts/screenshots.sh search dark  # selected scenes
#
# Needs: Xvfb, xdotool, ImageMagick (import), sqlite3. Each scene starts the app with a fresh
# data directory and its own settings, so scenes are independent and reproducible.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${OUT:-$ROOT/docs/screenshots}"
BIN="${BIN:-$ROOT/target/release/bible-rs}"
export DISPLAY="${DISPLAY_NUM:-:98}"
export LIBGL_ALWAYS_SOFTWARE=1

mkdir -p "$OUT"
if ! xdpyinfo >/dev/null 2>&1; then
  Xvfb "$DISPLAY" -screen 0 1600x1000x24 >/dev/null 2>&1 &
  XVFB_PID=$!
  sleep 1
fi
[[ -x "$BIN" ]] || (cd "$ROOT" && cargo build --release -p bible-rs)
WORK="$(mktemp -d /tmp/bible-rs-shots.XXXXXX)"
cleanup() {
  pkill -x bible-rs 2>/dev/null || true
  [[ -n "${XVFB_PID:-}" ]] && kill "$XVFB_PID" 2>/dev/null || true
  rm -rf "$WORK"
}
trap cleanup EXIT

# Demo study data (public-domain verses only).
cat >"$WORK/notes.json" <<'JSON'
{"app":"bible-rs","format":1,"exported_at":"2026-10-14T12:00:00Z","translation":"KJV",
 "highlights":[
  {"reference":"John 1:1","book":"JHN","chapter":1,"verse":1,"color":"yellow","created":"2026-10-14T12:00:00Z"},
  {"reference":"John 1:4","book":"JHN","chapter":1,"verse":4,"color":"green","created":"2026-10-14T12:00:00Z"},
  {"reference":"John 1:5","book":"JHN","chapter":1,"verse":5,"color":"green","created":"2026-10-14T12:00:00Z"},
  {"reference":"John 1:14","book":"JHN","chapter":1,"verse":14,"color":"blue","created":"2026-10-14T12:00:00Z"},
  {"reference":"John 3:16","book":"JHN","chapter":3,"verse":16,"color":"yellow","created":"2026-10-14T12:00:00Z"},
  {"reference":"Psalm 23:1","book":"PSA","chapter":23,"verse":1,"color":"green","created":"2026-10-14T12:00:00Z"},
  {"reference":"Romans 8:28","book":"ROM","chapter":8,"verse":28,"color":"pink","created":"2026-10-14T12:00:00Z"},
  {"reference":"Romans 8:38","book":"ROM","chapter":8,"verse":38,"color":"purple","created":"2026-10-14T12:00:00Z"},
  {"reference":"Romans 8:39","book":"ROM","chapter":8,"verse":39,"color":"purple","created":"2026-10-14T12:00:00Z"}],
 "notes":[
  {"reference":"John 1:14","book":"JHN","chapter":1,"verse":14,"note":"The Word became flesh — compare Philippians 2:6–8 and Hebrews 1:3.","created":"2026-10-14T12:00:00Z","updated":"2026-10-14T12:00:00Z"},
  {"reference":"Romans 8:28","book":"ROM","chapter":8,"verse":28,"note":"Not that all things are good, but that God works all things together for good.\n\nSee Genesis 50:20.","created":"2026-10-14T12:00:00Z","updated":"2026-10-14T12:00:00Z"}],
 "bookmarks":[
  {"reference":"John 1:1","book":"JHN","chapter":1,"verse":1,"label":"Prologue","created":"2026-10-14T12:00:00Z"},
  {"reference":"Psalm 23:1","book":"PSA","chapter":23,"verse":1,"label":"The Shepherd","created":"2026-10-14T12:00:00Z"},
  {"reference":"Romans 8:28","book":"ROM","chapter":8,"verse":28,"label":"","created":"2026-10-14T12:00:00Z"}]}
JSON

WID=
# prepare <theme> <translation> <extra json fields>: fresh data dir with settings and demo notes.
prepare() {
  APP_HOME="$WORK/home-$RANDOM"
  mkdir -p "$APP_HOME"
  BIBLE_RS_HOME="$APP_HOME" "$BIN" import-notes "$WORK/notes.json" >/dev/null
  local typo='{"size":19.0,"line_height":1.62,"mode":"'"${MODE:-Paragraph}"'","red_letters":true,"justify":false,"verse_numbers":true,"headings":true,"measure":34.0}'
  printf '{"translation":"%s","theme":"%s","typo":%s,%s}' "$2" "$1" "$typo" "${3:-\"sidebar_open\":false}" >"$WORK/settings.json"
  sqlite3 "$APP_HOME/study.sqlite" "INSERT OR REPLACE INTO settings(key, value) VALUES('app', CAST(readfile('$WORK/settings.json') AS TEXT));"
}

launch() {
  pkill -x bible-rs 2>/dev/null || true
  sleep 0.4
  BIBLE_RS_HOME="$APP_HOME" setsid "$BIN" >"$WORK/app.log" 2>&1 </dev/null &
  disown
  WID="$(xdotool search --sync --onlyvisible --name '^Bible$' | head -1)"
  if [[ -n "${SIZE:-}" ]]; then
    xdotool windowsize "$WID" ${SIZE/x/ }
    xdotool windowmove "$WID" $(( (1600 - ${SIZE%x*}) / 2 )) 20
  fi
  sleep 1.2
  xdotool windowfocus "$WID"
  eval "$(xdotool getwindowgeometry --shell "$WID")"
  OX=$X OY=$Y
}

key() { xdotool key "$@"; sleep 0.35; }
typ() { xdotool type --delay 30 -- "$1"; sleep 0.4; }
click() { xdotool mousemove $((OX + $1)) $((OY + $2)) click 1; sleep "${3:-0.5}"; }
park() { xdotool mousemove $((OX + ${1:-1250})) $((OY + ${2:-840})); sleep 0.3; }
goto() { key ctrl+k; typ "$1"; key Return; sleep 1.0; }
shot() {
  sleep "${2:-0.5}"
  import -window "$WID" "$OUT/$1.png"
  echo "  $OUT/$1.png"
}

scene_reader() {
  prepare Light kjv
  launch
  goto "jn 1"
  park
  shot reader-light
}

scene_sepia() {
  prepare Sepia bsb
  launch
  goto "ps 23"
  park
  shot psalm-sepia
}

scene_dark() {
  MODE=VersePerLine prepare Dark kjv
  launch
  goto "mt 5"
  park
  shot dark-verse-per-line
}

scene_search() {
  prepare Light kjv '"sidebar_open":true,"sidebar_tab":"Search"'
  launch
  goto "jn 13"
  key ctrl+f
  typ "love one another"
  park 900 840
  shot search
}

scene_palette() {
  prepare Light kjv
  launch
  goto "rom 8"
  key ctrl+k
  typ "jo 3:16"
  shot jump-palette
}

scene_parallel() {
  SIZE=1560x960 prepare Light kjv '"sidebar_open":false,"parallel_on":true,"parallel":["web","bsb"]'
  SIZE=1560x960 launch
  goto "ps 1"
  park 1500 900
  shot parallel
}

scene_compare() {
  prepare Light kjv
  launch
  goto "jn 3:16"
  key c
  sleep 1.0
  park 40 840
  shot compare
}

scene_study() {
  prepare Light kjv '"sidebar_open":true,"sidebar_tab":"Study"'
  launch
  goto "rom 8:28"
  key n
  key Escape
  park 900 840
  shot study-notes
}

scene_books() {
  prepare Light kjv
  launch
  goto "jn 1"
  click 545 23 0.8
  shot books
}

scene_translations() {
  SIZE=1280x960 prepare Light kjv
  # Import a real SWORD module (the public-domain Geneva Bible from CrossWire) to show imports.
  local mod="$ROOT/.sources/Geneva1599.zip"
  if [[ ! -f "$mod" ]]; then
    mkdir -p "$ROOT/.sources"
    curl -fsSL -o "$mod" https://www.crosswire.org/ftpmirror/pub/sword/packages/rawzip/Geneva1599.zip || true
  fi
  [[ -f "$mod" ]] && BIBLE_RS_HOME="$APP_HOME" "$BIN" import "$mod" --abbrev GNV >/dev/null
  SIZE=1280x960 launch
  goto "jn 1"
  key ctrl+comma
  click 640 234 0.6
  park 1250 840
  shot settings-translations
}

scene_shortcuts() {
  prepare Light kjv
  launch
  goto "jn 1"
  key ctrl+slash
  park 1250 840
  shot shortcuts
}

ALL=(reader sepia dark search palette parallel compare study books translations shortcuts)
for s in "${@:-${ALL[@]}}"; do
  echo "scene: $s"
  SIZE= "scene_$s"
done
