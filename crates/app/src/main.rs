//! bible-rs: a fast, native Bible reader.
//!
//! Environment:
//! - `BIBLE_RS_HOME` — data directory override (study database, imports, caches)
//! - `BIBLE_RS_RENDERER=glow|wgpu` — force a renderer (default: wgpu with Metal on macOS and Direct3D 12
//!   on Windows, glow elsewhere)
//! - `BIBLE_RS_STARTUP_PROBE=1` — print startup timings as JSON after the first frame and exit
//! - `BIBLE_RS_SCROLL_PROBE=1` — scroll through the Bible programmatically, print frame timings, exit
//! - `BIBLE_RS_SIDEBAR_PROBE=<n>` — open and close the sidebar n times, print frame timings, exit
//! - `BIBLE_RS_BACKGROUND=1` — macOS: open the window without taking focus (screenshots, benchmarks)
//!
//! On Windows it is a windowed program, so no console window opens with the app; the command line
//! writes to the console it was started from (`attach_console` in `lib.rs`).
#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() -> eframe::Result {
    bible_rs::run()
}
