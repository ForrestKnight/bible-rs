//! Highlighting in the running app: dragging across the text chooses words (in a verse or across
//! verses), a colour key highlights just them and 0 clears just them, ⌘C copies them, and clicking
//! and shift-clicking still selects whole verses. Its own test binary because it sets
//! `BIBLE_RS_OFFSCREEN` (see `book_picker_app.rs`).

use std::sync::Arc;
use std::time::{Duration, Instant};

use bible_core::books::{BookId, ChapterIdx, VerseRef};
use bible_core::study::{HighlightColor, TextSpan};
use bible_rs::app::{BibleApp, Startup, initialize, online_counter};
use bible_rs::reader::TextPos;
use bible_rs::theme;
use eframe::egui::{self, Event, Key, Modifiers, OutputCommand, PointerButton, Pos2};
use egui_kittest::Harness;

const JOHN: BookId = BookId(42);

fn press(h: &mut Harness<'_, BibleApp>, pos: Pos2, pressed: bool, modifiers: Modifiers) {
    h.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers });
    h.step();
}

fn drag(h: &mut Harness<'_, BibleApp>, from: Pos2, to: Pos2) {
    h.event(Event::PointerMoved(from));
    h.step();
    press(h, from, true, Modifiers::NONE);
    for i in 1..=8 {
        h.event(Event::PointerMoved(from + (to - from) * (i as f32 / 8.0)));
        h.step();
    }
    press(h, to, false, Modifiers::NONE);
    h.run_steps(2);
}

fn click(h: &mut Harness<'_, BibleApp>, at: Pos2, modifiers: Modifiers) {
    // The keys held come from their own event, not the pointer's.
    h.event(Event::ModifiersChanged(modifiers));
    h.event(Event::PointerMoved(at));
    h.step();
    press(h, at, true, modifiers);
    press(h, at, false, modifiers);
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(2);
}

fn key(h: &mut Harness<'_, BibleApp>, key: Key) {
    h.key_press(key);
    h.run_steps(2);
}

/// What one press of a chord copied, if anything.
fn copied(h: &mut Harness<'_, BibleApp>, key: Key, modifiers: Modifiers) -> Option<String> {
    h.event(Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers });
    h.step();
    let text = h.output().platform_output.commands.iter().find_map(|c| match c {
        OutputCommand::CopyText(t) => Some(t.clone()),
        _ => None,
    });
    h.event(Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers });
    h.step();
    text
}

/// The char offset of `words` in `text`.
fn offset(text: &str, words: &str) -> usize {
    text[..text.find(words).unwrap_or_else(|| panic!("{words:?} not in {text:?}"))].chars().count()
}

#[test]
fn dragging_chooses_words_to_highlight() {
    // SAFETY: the only test in this binary, so nothing else reads the environment meanwhile.
    unsafe { std::env::set_var("BIBLE_RS_OFFSCREEN", "1") };
    let dir = std::env::temp_dir().join(format!("bible-rs-highlight-words-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut init = initialize(dir.clone(), Arc::new(|| {})).expect("loading data");
    init.settings.onboarded = true;
    init.settings.translation = "kjv".into();
    let c = ChapterIdx::new(JOHN, 3).unwrap();
    init.settings.chapter = c.0;
    let startup =
        Startup { t0: Instant::now(), probe: false, init: Duration::ZERO, first_ui: None, frames: 0 };
    let mut h = Harness::builder().with_size(egui::vec2(1280.0, 800.0)).build_eframe(|cc| {
        cc.egui_ctx.set_fonts(theme::font_definitions());
        BibleApp::new(cc, init, online_counter(), startup)
    });
    h.run_steps(4);
    h.state_mut().reader.jump(c, Some(16), false);
    h.run_steps(6);

    let kjv = bible_core::bundled::load("kjv").unwrap();
    let v = |n: u8| VerseRef::new(JOHN, 3, n);
    let text = |n: u8| kjv.find(v(n)).unwrap().text.to_string();
    let at = |h: &Harness<'_, BibleApp>, n: u8, offset: usize| {
        h.state().reader.point_at(TextPos { verse: v(n), offset, column: 0 }).expect("on screen")
    };
    let shown = |h: &Harness<'_, BibleApp>, n: u8| {
        let t = text(n);
        h.state()
            .study
            .highlight_ranges(v(n), &t)
            .into_iter()
            .map(|(r, c)| (t.chars().skip(r.start).take(r.len()).collect::<String>(), c))
            .collect::<Vec<_>>()
    };

    // Words in one verse.
    let words = "God so loved the world";
    let (a, b) = (offset(&text(16), words), offset(&text(16), words) + words.len());
    let (p, q) = (at(&h, 16, a), at(&h, 16, b));
    drag(&mut h, p, q);
    assert_eq!(
        h.state().reader.text_span(),
        Some((TextSpan { first: v(16), start: a, last: v(16), end: b }, "kjv"))
    );
    assert_eq!(h.state().reader.selection, Some((v(16), v(16))), "the verse is selected with its words");
    assert_eq!(copied(&mut h, Key::C, Modifiers::COMMAND).as_deref(), Some(words), "⌘C copies the words");
    assert_eq!(
        copied(&mut h, Key::C, Modifiers::COMMAND | Modifiers::SHIFT).as_deref(),
        Some("“God so loved the world” — John 3:16 (KJV)")
    );
    key(&mut h, Key::Num1);
    assert_eq!(shown(&h, 16), vec![(words.to_string(), HighlightColor::Yellow)]);

    // Across a verse boundary: the rest of 16 and the start of 17.
    let (from, to) = (offset(&text(16), "everlasting"), offset(&text(17), "to condemn"));
    let (p, q) = (at(&h, 16, from), at(&h, 17, to));
    drag(&mut h, p, q);
    assert_eq!(h.state().reader.selection, Some((v(16), v(17))));
    key(&mut h, Key::Num2);
    let rest16: String = text(16).chars().skip(from).collect();
    let start17: String = text(17).chars().take(to).collect();
    assert_eq!(
        shown(&h, 16),
        vec![(words.to_string(), HighlightColor::Yellow), (rest16.clone(), HighlightColor::Green)]
    );
    assert_eq!(shown(&h, 17), vec![(start17.trim_end().to_string(), HighlightColor::Green)]);

    // Shift-click carries the words on; 0 clears just them.
    let (s, e) = (offset(&text(16), "so loved"), offset(&text(16), "so loved") + "so loved".len());
    let (p, q) = (at(&h, 16, s), at(&h, 16, s + 3));
    drag(&mut h, p, q);
    let p = at(&h, 16, e);
    click(&mut h, p, Modifiers::SHIFT);
    assert_eq!(
        h.state().reader.text_span(),
        Some((TextSpan { first: v(16), start: s, last: v(16), end: e }, "kjv"))
    );
    key(&mut h, Key::Num0);
    assert_eq!(
        shown(&h, 16),
        vec![
            ("God".into(), HighlightColor::Yellow),
            ("the world".into(), HighlightColor::Yellow),
            (rest16, HighlightColor::Green)
        ]
    );

    // Clicking the words' verse selects it whole; clicking it again lets go.
    let p = at(&h, 16, 1);
    click(&mut h, p, Modifiers::NONE);
    assert_eq!((h.state().reader.selection, h.state().reader.text_span()), (Some((v(16), v(16))), None));
    click(&mut h, p, Modifiers::NONE);
    assert_eq!(h.state().reader.selection, None);

    // Clicking selects whole verses again; shift-click extends them; a colour covers them whole.
    let p = at(&h, 18, 3);
    click(&mut h, p, Modifiers::NONE);
    assert_eq!(h.state().reader.text_span(), None);
    let p = at(&h, 19, 3);
    click(&mut h, p, Modifiers::SHIFT);
    assert_eq!(h.state().reader.selection, Some((v(18), v(19))));
    key(&mut h, Key::Num3);
    for n in [18, 19] {
        assert_eq!(shown(&h, n), vec![(text(n), HighlightColor::Blue)], "verse {n}");
        assert!(h.state().study.highlight_parts(v(n))[0].is_whole());
    }
    // Words chosen in one translation go when the reader switches to another; the verse stays.
    let (p, q) = (at(&h, 16, a), at(&h, 16, b));
    drag(&mut h, p, q);
    assert!(h.state().reader.text_span().is_some());
    h.state_mut().settings.translation = "web".into();
    h.run_steps(3);
    assert_eq!((h.state().reader.text_span(), h.state().reader.selection), (None, Some((v(16), v(16)))));
    h.state_mut().settings.translation = "kjv".into();
    h.run_steps(3);

    // Escape lets go of the selection.
    key(&mut h, Key::Escape);
    assert_eq!(h.state().reader.selection, None);
    let _ = std::fs::remove_dir_all(&dir);
}
