#!/usr/bin/env bash
# Measures the sidebar's open/close transitions: how the frames around each toggle are paced,
# what each frame costs, how many chapters are typeset, and whether drawing stops afterwards.
#
#   scripts/bench-sidebar.sh                 # every scenario, 8 toggles each
#   TOGGLES=12 scripts/bench-sidebar.sh default compact
#   BIN=/path/to/bible-rs scripts/bench-sidebar.sh
#
# macOS opens a real window in front for each scenario (a covered window isn't paced by the
# display). Put it on the display under test: ProMotion panels pace at 120 Hz.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${BIN:-$ROOT/target/release/bible-rs}"
TOGGLES="${TOGGLES:-8}"
[[ -x "$BIN" ]] || (cd "$ROOT" && cargo build --release -p bible-rs)

# scenario <name> <window size> <settings JSON fields> [demo steps]
run() {
  local name="$1" size="$2" fields="$3" demo="${4:-}"
  local home
  home="$(mktemp -d)"
  # A first launch creates the database; the settings are then written into it.
  BIBLE_RS_HOME="$home" BIBLE_RS_STARTUP_PROBE=1 BIBLE_RS_BACKGROUND=1 "$BIN" >/dev/null 2>&1 || true
  printf '{"translation":"kjv",%s}' "$fields" >"$home/settings.json"
  sqlite3 "$home/study.sqlite" \
    "INSERT OR REPLACE INTO settings(key, value) VALUES('app', CAST(readfile('$home/settings.json') AS TEXT));"
  local out
  out=$(BIBLE_RS_HOME="$home" BIBLE_RS_DEMO="size:$size;$demo" BIBLE_RS_SIDEBAR_PROBE="$TOGGLES" \
    "$BIN" 2>/dev/null | tail -1)
  printf '%-22s %s\n' "$name" "$out"
  rm -rf "$home"
}

scenarios=("$@")
[[ ${#scenarios[@]} -gt 0 ]] || scenarios=(default dark sepia compact medium parallel parallel-compact psalm119-compact)
for s in "${scenarios[@]}"; do
  case "$s" in
    default) run default 1280x800 '"theme":"Light"' ;;
    dark) run dark 1280x800 '"theme":"Dark"' ;;
    sepia) run sepia 1280x800 '"theme":"Sepia"' ;;
    compact) run compact 720x760 '"theme":"Light"' ;;
    medium) run medium 960x760 '"theme":"Light"' ;;
    parallel) run parallel 1280x800 '"theme":"Light","parallel_on":true,"parallel":["web"]' ;;
    parallel-compact) run parallel-compact 720x760 '"theme":"Light","parallel_on":true,"parallel":["web"]' ;;
    psalm119-compact) run psalm119-compact 720x760 '"theme":"Light"' 'jump:ps 119:40' ;;
    *) echo "unknown scenario: $s" >&2; exit 1 ;;
  esac
done
