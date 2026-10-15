#!/usr/bin/env bash
# Measures startup (process launch → first painted frame) and scrolling frame times.
#
#   scripts/bench-startup.sh          # 10 launches + one scroll run
#   RUNS=20 scripts/bench-startup.sh
#
# On Linux without a display it starts Xvfb (software OpenGL); on macOS it opens real windows.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${BIN:-$ROOT/target/release/bible-rs}"
RUNS="${RUNS:-10}"
[[ -x "$BIN" ]] || (cd "$ROOT" && cargo build --release -p bible-rs)
if [[ "$(uname -s)" == "Linux" ]]; then
  export DISPLAY="${DISPLAY:-:98}"
  export LIBGL_ALWAYS_SOFTWARE="${LIBGL_ALWAYS_SOFTWARE:-1}"
  if ! xdpyinfo >/dev/null 2>&1; then
    Xvfb "$DISPLAY" -screen 0 1600x1000x24 >/dev/null 2>&1 &
    XVFB_PID=$!
    trap 'kill $XVFB_PID 2>/dev/null || true' EXIT
    sleep 1
  fi
fi
HOME_DIR="$(mktemp -d)"
# One warm-up launch creates the database (first run only) and warms the OS file cache.
BIBLE_RS_HOME="$HOME_DIR" BIBLE_RS_STARTUP_PROBE=1 "$BIN" >/dev/null 2>&1 || true

now_ms() { python3 -c 'import time; print(time.time()*1000)'; }
built=() presented=() external=()
for _ in $(seq 1 "$RUNS"); do
  start=$(now_ms)
  line=$(BIBLE_RS_HOME="$HOME_DIR" BIBLE_RS_STARTUP_PROBE=1 "$BIN" 2>/dev/null | tail -1)
  b=$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["first_frame_built_ms"])' "$line")
  p=$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["first_frame_presented_ms"])' "$line")
  e=$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["epoch_ms"] - float(sys.argv[2]))' "$line" "$start")
  built+=("$b") presented+=("$p") external+=("$e")
done
python3 - "${built[*]}" "${presented[*]}" "${external[*]}" <<'PY'
import statistics, sys
names = ["first frame built (in-process)", "first frame presented (in-process)", "launch → first frame presented (wall clock)"]
for name, arg in zip(names, sys.argv[1:]):
    v = sorted(float(x) for x in arg.split())
    print(f"{name:<44} median {statistics.median(v):7.1f} ms   min {v[0]:7.1f}   max {v[-1]:7.1f}   (n={len(v)})")
PY
echo "scrolling (900 frames, ~2,200 px/s through consecutive chapters):"
BIBLE_RS_HOME="$HOME_DIR" BIBLE_RS_SCROLL_PROBE=1 "$BIN" 2>/dev/null | tail -1
rm -rf "$HOME_DIR"
