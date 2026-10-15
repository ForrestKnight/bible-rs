//! bible-rs: a fast, native Bible reader.
//!
//! Environment:
//! - `BIBLE_RS_HOME` — data directory override (study database, imports, caches)
//! - `BIBLE_RS_RENDERER=glow|wgpu` — force a renderer (default: glow, the fastest to start)
//! - `BIBLE_RS_STARTUP_PROBE=1` — print startup timings as JSON after the first frame and exit
//! - `BIBLE_RS_SCROLL_PROBE=1` — scroll through the Bible programmatically, print frame timings, exit

fn main() -> eframe::Result {
    bible_rs::run()
}
