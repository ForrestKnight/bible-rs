//! bible-rs: a fast, native Bible reader.
//!
//! Environment:
//! - `BIBLE_RS_HOME` — data directory override (study database, imports, caches)
//! - `BIBLE_RS_RENDERER=glow|wgpu` — force a renderer (default: wgpu/Metal on macOS, glow elsewhere)
//! - `BIBLE_RS_STARTUP_PROBE=1` — print startup timings as JSON after the first frame and exit
//! - `BIBLE_RS_SCROLL_PROBE=1` — scroll through the Bible programmatically, print frame timings, exit
//! - `BIBLE_RS_SIDEBAR_PROBE=<n>` — open and close the sidebar n times, print frame timings, exit
//! - `BIBLE_RS_BACKGROUND=1` — macOS: open the window without taking focus (screenshots, benchmarks)

fn main() -> eframe::Result {
    bible_rs::run()
}
