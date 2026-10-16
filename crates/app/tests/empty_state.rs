//! The empty state paints every row (glyph, title, each line of the body, the buttons, each row
//! of hints) centred on one axis, within the room it is given, and as tall as it says it is.

use bible_rs::pages::{Empty, Tone};
use bible_rs::theme::{self, Theme};
use bible_rs::widgets::{Button, Kind};
use eframe::egui::{self, Pos2, Rect, Shape, vec2};

const HINTS: &[(&[&str], &str)] =
    &[(&["N"], "New note on the selected verse"), (&["⌘", "E"], "Export as Markdown")];

fn notes() -> [Button<'static>; 2] {
    [Button::new(Kind::Primary, "Open John 1"), Button::new(Kind::Ghost, "Import a backup")]
}

/// Paints `empty` centred on `cx` from `top` in `room`. Returns the ink of each painted row, with
/// each line of text as its own row, and the height `empty` reports.
fn paint(empty: &Empty<'_>, cx: f32, top: f32, room: f32) -> (Vec<Rect>, f32) {
    let ctx = egui::Context::default();
    ctx.set_fonts(theme::font_definitions());
    let input = || egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1600.0, 1200.0))),
        ..Default::default()
    };
    ctx.run_ui(input(), |_| {}).textures_delta.clear();
    let mut height = 0.0;
    let mut out = ctx.run_ui(input(), |ui| {
        let p = Theme::Light.palette();
        height = empty.height(ui.painter(), room);
        empty.show(ui, p, cx, top, room);
    });
    out.textures_delta.clear();
    let mut rects = Vec::new();
    fn collect(shape: &Shape, rects: &mut Vec<Rect>) {
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| collect(s, rects)),
            Shape::Text(t) => {
                for row in &t.galley.rows {
                    if !row.glyphs.is_empty() {
                        rects.push(row.rect().translate(t.pos.to_vec2()));
                    }
                }
            }
            // Shadows are not ink.
            Shape::Noop => {}
            Shape::Rect(r) if r.blur_width > 0.0 => {}
            s => rects.push(s.visual_bounding_rect()),
        }
    }
    for s in &out.shapes {
        collect(&s.shape, &mut rects);
    }
    (bands(rects), height)
}

/// Merges shapes that share any height into horizontal bands, top to bottom.
fn bands(mut rects: Vec<Rect>) -> Vec<Rect> {
    rects.retain(|r| r.is_positive());
    rects.sort_by(|a, b| a.top().total_cmp(&b.top()));
    let mut out: Vec<Rect> = Vec::new();
    for r in rects {
        match out.last_mut() {
            Some(b) if r.top() < b.bottom() - 0.5 => *b = b.union(r),
            _ => out.push(r),
        }
    }
    out
}

fn assert_centred(empty: &Empty<'_>, room: f32, min_bands: usize) {
    let (cx, top) = (700.0, 100.0);
    let (bands, height) = paint(empty, cx, top, room);
    assert!(bands.len() >= min_bands, "expected {min_bands}+ rows, painted {bands:?}");
    for b in &bands {
        let off = b.center().x - cx;
        assert!(off.abs() <= 1.0, "row {b:?} is {off:+.1} pt off the axis (room {room})");
        assert!(b.width() <= room + 1.0, "row {b:?} is wider than the room {room}");
    }
    // Nothing is painted below the reported height; a ghost button's box ends below its label.
    let bottom = bands.iter().map(|b| b.bottom()).fold(top, f32::max);
    let painted = bottom - top;
    assert!(
        painted <= height + 1.0 && painted >= height - 10.0,
        "reported height {height} but painted {painted} (room {room})"
    );
}

#[test]
fn notes_empty_state_is_centred_at_any_width() {
    let buttons = notes();
    let empty = Empty {
        glyph: bible_rs::icons::note,
        tone: Tone::Accent,
        title: "Your notes will live here",
        body: "Select a verse and press N, or right-click it and choose Add Note. Notes stay on this Mac and \
               export to Markdown.",
        buttons: &buttons,
        hints: HINTS,
        width: 344.0,
    };
    // A wide window; the compact window with the sidebar open, where the hints wrap.
    assert_centred(&empty, 1200.0, 7);
    assert_centred(&empty, 356.0, 8);
}

#[test]
fn quiet_and_danger_states_are_centred() {
    let clear = [Button::new(Kind::Ghost, "Clear filters")];
    let search = Empty {
        glyph: bible_rs::icons::search_thin,
        tone: Tone::Quiet,
        title: "No results for “bethsaida pool”",
        body: "Every word has to appear in the same verse.\nTry fewer words or search the whole Bible.",
        buttons: &clear,
        hints: &[],
        width: 320.0,
    };
    assert_centred(&search, 336.0, 5);
    let retry = [Button::new(Kind::Secondary, "Retry"), Button::new(Kind::Ghost, "Read in KJV instead")];
    let offline = Empty {
        glyph: bible_rs::icons::cloud_slash,
        tone: Tone::Danger,
        title: "ESV can’t load John 1 right now",
        body: "Chapters from api.esv.org need a connection. Your highlights and notes on this chapter are safe \
               and will reappear.",
        buttons: &retry,
        hints: &[],
        width: 520.0,
    };
    assert_centred(&offline, 1200.0, 5);
    assert_centred(&offline, 356.0, 6);
}

#[test]
fn hints_wrap_into_centred_rows_when_the_room_is_narrow() {
    let empty = Empty {
        glyph: bible_rs::icons::note,
        tone: Tone::Accent,
        title: "Your notes will live here",
        body: "Notes stay on this Mac.",
        buttons: &[],
        hints: HINTS,
        width: 344.0,
    };
    let (wide, wide_h) = paint(&empty, 700.0, 100.0, 1200.0);
    let (narrow, narrow_h) = paint(&empty, 700.0, 100.0, 300.0);
    assert_eq!(narrow.len(), wide.len() + 1, "the two hints should stack in 300 pt");
    assert!((narrow_h - wide_h - 28.0).abs() < 0.01, "a second hint row adds 20 + 8 pt");
}

#[test]
fn optical_top_sits_a_little_above_the_middle() {
    use bible_rs::pages::optical_top;
    let area = egui::Rangef::new(52.0, 800.0);
    let top = optical_top(area, 200.0, 0.0);
    let middle = (top + top + 200.0) / 2.0;
    assert!(middle < area.center() && middle > area.center() - 40.0, "block middle {middle}");
    // Never above the floor (the page header).
    assert_eq!(optical_top(area, 700.0, 154.0), 154.0);
}

/// In a 620 × 420 window with the sidebar open the empty Notes page is taller than the window;
/// it scrolls like the other pages, so its last hint row can be brought into view.
#[test]
fn the_empty_notes_page_scrolls_to_its_last_row_in_a_short_window() {
    use bible_rs::pages::{Page, PageEnv, Pages};
    let dir = std::env::temp_dir().join(format!("bible-rs-empty-notes-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let library = bible_core::library::Library::open(&dir, std::sync::Arc::new(|| {}));
    let study = bible_core::study::StudyStore::open_in_memory().unwrap();
    let settings = bible_rs::app::Settings::default();
    let env = PageEnv { library: &library, study: &study, settings: &settings, query: "", toolbar: 52.0 };
    let mut pages = Pages::default();
    pages.open(Page::Notes);
    let window = Rect::from_min_size(Pos2::ZERO, vec2(620.0, 420.0));
    let main = Rect::from_min_max(egui::pos2(264.0, 0.0), window.max);

    let ctx = egui::Context::default();
    ctx.set_fonts(theme::font_definitions());
    ctx.all_styles_mut(|s| s.scroll_animation = egui::style::ScrollAnimation::none());
    let mut time = 0.0;
    // Runs a frame; returns the bottom of the last hint's text.
    let mut frame = |events: Vec<egui::Event>| {
        time += 0.1;
        let input =
            egui::RawInput { screen_rect: Some(window), time: Some(time), events, ..Default::default() };
        let mut out = ctx.run_ui(input, |ui| pages.show(ui, main, &env, &mut Vec::new()));
        out.textures_delta.clear();
        out.shapes
            .iter()
            .find_map(|s| match &s.shape {
                Shape::Text(t) if t.galley.job.text == "Export as Markdown" => {
                    Some(t.galley.rect.translate(t.pos.to_vec2()).bottom())
                }
                _ => None,
            })
            .expect("the hint is painted")
    };
    frame(Vec::new());
    let before = frame(Vec::new());
    assert!(before > window.bottom(), "expected the last hint below the window, at {before}");
    let wheel = |dy: f32| {
        vec![
            egui::Event::PointerMoved(main.center()),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, dy),
                modifiers: egui::Modifiers::NONE,
                phase: egui::TouchPhase::Move,
            },
        ]
    };
    for _ in 0..5 {
        frame(wheel(-200.0));
    }
    let after = frame(Vec::new());
    assert!(after <= window.bottom(), "the last hint should scroll into view, still at {after}");
    let _ = std::fs::remove_dir_all(&dir);
}
