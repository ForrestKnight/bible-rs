//! The reader after the display's scale changes. On Wayland a window opens at scale 1 and learns
//! the monitor's scale a frame or two later (winit's `ScaleFactorChanged`); on a 2× display the
//! first chapters are typeset before that. Each of them must be typeset again at the new scale, or
//! epaint paints them every frame with a warning ("pixels_per_point have changed between text
//! layout and tessellation") and glyphs placed for the wrong pixel grid. Its own test binary
//! because it sets `BIBLE_RS_OFFSCREEN` (see `book_picker_app.rs`).

use std::sync::Arc;
use std::time::{Duration, Instant};

use bible_core::books::{BookId, ChapterIdx};
use bible_rs::app::{BibleApp, Startup, initialize, online_counter};
use bible_rs::theme;
use eframe::egui;
use egui_kittest::Harness;

/// The scale every text shape painted last frame was laid out for.
fn painted_text_scales(h: &Harness<'_, BibleApp>) -> Vec<f32> {
    h.output()
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Text(t) => Some(t.galley.pixels_per_point),
            _ => None,
        })
        .collect()
}

#[test]
fn chapters_are_typeset_again_at_the_new_scale() {
    unsafe { std::env::set_var("BIBLE_RS_OFFSCREEN", "1") };
    let dir = std::env::temp_dir().join("bible-rs-scale-change");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut init = initialize(dir.clone(), Arc::new(|| {})).expect("loading data");
    init.settings.onboarded = true;
    init.settings.translation = "kjv".into();
    init.settings.chapter = ChapterIdx::new(BookId(42), 3).unwrap().0;
    let startup =
        Startup { t0: Instant::now(), probe: false, init: Duration::ZERO, first_ui: None, frames: 0 };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_pixels_per_point(1.0)
        .build_eframe(move |cc| {
            cc.egui_ctx.set_fonts(theme::font_definitions());
            BibleApp::new(cc, init, online_counter(), startup)
        });
    h.run_steps(4);
    let before = painted_text_scales(&h);
    assert!(
        !before.is_empty() && before.iter().all(|s| *s == 1.0),
        "the chapter is on screen, laid out at scale 1: {before:?}"
    );

    // The monitor's scale arrives, as winit reports it: the native scale changes and the window
    // keeps its size in points, so nothing but the scale changes for the reader.
    h.input_mut().viewports.get_mut(&egui::ViewportId::ROOT).unwrap().native_pixels_per_point = Some(2.0);
    h.run_steps(4);
    assert_eq!(h.ctx.pixels_per_point(), 2.0);
    assert_eq!(
        h.ctx.input(|i| i.content_rect().size()),
        egui::vec2(1280.0, 800.0),
        "the window kept its size in points"
    );
    let after = painted_text_scales(&h);
    let stale = after.iter().filter(|s| **s != 2.0).count();
    assert_eq!(stale, 0, "{stale} of {} text shapes were laid out for another scale", after.len());
    let _ = std::fs::remove_dir_all(&dir);
}
