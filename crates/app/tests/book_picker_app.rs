//! The book picker in the running app: while it's open the reader's single-key shortcuts (the
//! arrows, escape) go to the picker, and the copy chords still copy the selected verses. This is
//! its own test binary because it sets `BIBLE_RS_OFFSCREEN` (no macOS menu bar, which needs the
//! main thread) before anything else runs.

use std::sync::Arc;
use std::time::{Duration, Instant};

use bible_core::books::{BookId, ChapterIdx, VerseRef};
use bible_rs::app::{BibleApp, Startup, initialize, online_counter};
use bible_rs::{book_picker, theme};
use eframe::egui::{self, Key, Modifiers, OutputCommand};
use egui_kittest::Harness;

const JOHN: BookId = BookId(42);

#[test]
fn in_the_app_the_picker_takes_single_keys_but_copying_still_works() {
    // SAFETY: the only test in this binary, so nothing else reads the environment meanwhile.
    unsafe { std::env::set_var("BIBLE_RS_OFFSCREEN", "1") };
    let dir = std::env::temp_dir().join(format!("bible-rs-book-picker-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut init = initialize(dir.clone(), Arc::new(|| {})).expect("loading data");
    init.settings.onboarded = true;
    init.settings.chapter = ChapterIdx::new(JOHN, 1).unwrap().0;
    let startup =
        Startup { t0: Instant::now(), probe: false, init: Duration::ZERO, first_ui: None, frames: 0 };
    let mut harness = Harness::builder().with_size(egui::vec2(1280.0, 800.0)).build_eframe(|cc| {
        cc.egui_ctx.set_fonts(theme::font_definitions());
        BibleApp::new(cc, init, online_counter(), startup)
    });
    harness.run_steps(4);
    harness.state_mut().reader.set_selection(Some(VerseRef::new(JOHN, 1, 1)));
    egui::Popup::open_id(&harness.ctx, egui::Id::new("book-picker"));
    harness.run_steps(2);
    assert!(book_picker::is_open(&harness.ctx));
    // Presses a chord in one frame and says whether that frame copied text.
    let copies = |harness: &mut Harness<'_, BibleApp>, modifiers: Modifiers| {
        harness.event(egui::Event::Key {
            key: Key::C,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        });
        harness.step();
        let copied =
            harness.output().platform_output.commands.iter().any(|c| matches!(c, OutputCommand::CopyText(_)));
        harness.event(egui::Event::Key {
            key: Key::C,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers,
        });
        harness.step();
        copied
    };

    // Arrows move the picker's highlight, not the reader to the next chapter.
    harness.key_press(Key::ArrowRight);
    harness.run_steps(2);
    assert_eq!(harness.state().reader.location.0, ChapterIdx::new(JOHN, 1).unwrap(), "the reader stays");
    assert!(book_picker::is_open(&harness.ctx));

    // ⌘C and the copy-with-reference chord still copy the selected verse.
    assert!(copies(&mut harness, Modifiers::COMMAND), "⌘C copies with the picker open");
    assert!(copies(&mut harness, Modifiers::COMMAND | Modifiers::SHIFT), "⇧⌘C copies with the picker open");
    assert!(book_picker::is_open(&harness.ctx));

    // Escape reaches the picker rather than clearing the selection.
    harness.key_press(Key::Escape);
    harness.run_steps(2);
    assert!(!book_picker::is_open(&harness.ctx), "escape closes the picker");
    assert!(!harness.state().reader.selected_refs().is_empty(), "the verse stays selected");
    let _ = std::fs::remove_dir_all(&dir);
}
