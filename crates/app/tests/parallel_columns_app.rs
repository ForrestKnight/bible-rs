//! Parallel columns in the running app: "Remove column" in a column head's menu removes that
//! column, and removing the only extra one leaves parallel view (rather than `columns()` filling
//! in a default translation, so the column seemed to stay). Its own test binary because it sets
//! `BIBLE_RS_OFFSCREEN` (see `book_picker_app.rs`).

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bible_core::books::{BookId, ChapterIdx};
use bible_rs::app::{BibleApp, Startup, initialize, online_counter};
use bible_rs::theme;
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

const JOHN: BookId = BookId(42);

/// The app reading John 3 in the KJV beside `parallel`, in a 1280×800 window.
fn app(dir: &Path, parallel: &[&str]) -> Harness<'static, BibleApp> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).unwrap();
    let mut init = initialize(dir.to_path_buf(), Arc::new(|| {})).expect("loading data");
    init.settings.onboarded = true;
    init.settings.translation = "kjv".into();
    init.settings.chapter = ChapterIdx::new(JOHN, 3).unwrap().0;
    init.settings.parallel = parallel.iter().map(|id| id.to_string()).collect();
    init.settings.parallel_on = true;
    let startup =
        Startup { t0: Instant::now(), probe: false, init: Duration::ZERO, first_ui: None, frames: 0 };
    let mut h = Harness::builder().with_size(egui::vec2(1280.0, 800.0)).build_eframe(move |cc| {
        cc.egui_ctx.set_fonts(theme::font_definitions());
        BibleApp::new(cc, init, online_counter(), startup)
    });
    h.run_steps(4);
    h
}

fn click(h: &mut Harness<'_, BibleApp>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.step();
    for pressed in [true, false] {
        h.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        h.step();
    }
    h.run_steps(2);
}

/// Opens column `i`'s head menu (its abbreviation, 16 pt under the toolbar) and picks "Remove
/// column".
fn remove_column(h: &mut Harness<'_, BibleApp>, i: usize) {
    let (x, _) = h.state().reader.columns_x[i];
    click(h, egui::pos2(x + 8.0, theme::metrics::TOOLBAR_HEIGHT + 16.0 + 8.0));
    h.get_by_label("Remove column").click();
    h.run_steps(2);
}

#[test]
fn remove_column_removes_that_column_and_the_last_one_leaves_parallel_view() {
    // SAFETY: the only test in this binary, so nothing else reads the environment meanwhile.
    unsafe { std::env::set_var("BIBLE_RS_OFFSCREEN", "1") };
    let dir = std::env::temp_dir().join(format!("bible-rs-parallel-columns-{}", std::process::id()));

    // The only extra column: parallel view goes, not just the column's saved translation.
    let mut h = app(&dir, &["web"]);
    assert_eq!(h.state().columns(), ["kjv", "web"]);
    remove_column(&mut h, 1);
    assert!(h.state().settings.parallel.is_empty());
    assert!(!h.state().settings.parallel_on, "parallel view is off");
    assert_eq!(h.state().columns(), ["kjv"]);
    assert!(h.query_by_label("Remove column").is_none(), "the menu closed");
    drop(h);

    // One of several: that column goes and the others stay.
    let mut h = app(&dir, &["web", "asv"]);
    assert_eq!(h.state().columns(), ["kjv", "web", "asv"]);
    remove_column(&mut h, 1);
    assert!(h.state().settings.parallel_on);
    assert_eq!(h.state().columns(), ["kjv", "asv"]);
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}
