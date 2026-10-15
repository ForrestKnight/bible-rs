//! Renders the app offscreen to a PNG, for design comparisons and screenshots, without opening a
//! window (so it works whatever is on screen, and on a machine without one):
//!
//! ```text
//! BIBLE_RS_HOME=<data dir> BIBLE_RS_DEMO="palette:jn 3:16" \
//!     cargo run --release --example shot -- out.png [1280x800]
//! ```
//!
//! egui's test harness drives the real app with wgpu at 2× scale. On macOS the window's traffic
//! lights belong to AppKit, so they are drawn onto the image where AppKit puts them.

use std::sync::Arc;
use std::time::{Duration, Instant};

use bible_rs::app::{BibleApp, Startup, initialize, online_counter};
use bible_rs::theme;
use eframe::egui;
use egui_kittest::Harness;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(out) = args.first() else {
        eprintln!("usage: shot <out.png> [WIDTHxHEIGHT]");
        std::process::exit(2);
    };
    let (w, h) = args
        .get(1)
        .and_then(|s| s.split_once('x'))
        .and_then(|(a, b)| Some((a.parse::<f32>().ok()?, b.parse::<f32>().ok()?)))
        .unwrap_or((1280.0, 800.0));
    if std::env::var_os("BIBLE_RS_OFFSCREEN").is_none() {
        eprintln!("set BIBLE_RS_OFFSCREEN=1 (and BIBLE_RS_BACKGROUND=1) so the app lays out as in a window");
    }
    let init = initialize(bible_rs::data_dir(), Arc::new(|| {})).expect("loading data");
    let startup =
        Startup { t0: Instant::now(), probe: false, init: Duration::ZERO, first_ui: None, frames: 0 };
    let mut harness =
        Harness::builder().with_size(egui::vec2(w, h)).with_pixels_per_point(2.0).wgpu().build_eframe(|cc| {
            cc.egui_ctx.set_fonts(theme::font_definitions());
            BibleApp::new(cc, init, online_counter(), startup)
        });
    let demo_steps = std::env::var("BIBLE_RS_DEMO").map(|s| s.split(';').count()).unwrap_or(0);
    let steps =
        std::env::var("BIBLE_RS_SHOT_STEPS").ok().and_then(|s| s.parse().ok()).unwrap_or(30 + 4 * demo_steps);
    for _ in 0..steps {
        harness.step();
    }
    let mut img = harness.render().expect("rendering");
    if cfg!(target_os = "macos") && std::env::var_os("BIBLE_RS_NO_TRAFFIC_LIGHTS").is_none() {
        traffic_lights(&mut img);
    }
    img.save(out).expect("writing the image");
}

/// AppKit's traffic lights for a 52 pt unified toolbar: 14 pt circles 9 pt apart, the first
/// centred 27 pt from the left, on the 26 pt centre line, each with a faint inner hairline.
fn traffic_lights(img: &mut image::RgbaImage) {
    let s = 2.0;
    for (i, c) in [theme::traffic_lights::CLOSE, theme::traffic_lights::MINIMIZE, theme::traffic_lights::ZOOM]
        .into_iter()
        .enumerate()
    {
        let (cx, cy, r) = ((27.0 + 23.0 * i as f32) * s, 26.0 * s, 7.0 * s);
        let [cr, cg, cb, _] = c.to_srgba_unmultiplied();
        for y in (cy - r - 2.0) as u32..=(cy + r + 2.0) as u32 {
            for x in (cx - r - 2.0) as u32..=(cx + r + 2.0) as u32 {
                let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
                let cover = (r - d + 0.5).clamp(0.0, 1.0);
                if cover <= 0.0 || x >= img.width() || y >= img.height() {
                    continue;
                }
                // The hairline: a ring 0.5 pt wide inside the edge, 12 % black.
                let ring = (1.0 - ((r - 0.5) - d).abs()).clamp(0.0, 1.0) * 0.12;
                let px = img.get_pixel_mut(x, y);
                for (k, v) in [cr, cg, cb].into_iter().enumerate() {
                    let fill = v as f32 * (1.0 - ring);
                    px[k] = (px[k] as f32 * (1.0 - cover) + fill * cover).round() as u8;
                }
            }
        }
    }
}
