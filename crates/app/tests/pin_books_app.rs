//! Pinning books in the running app: right-clicking a book in a testament pins it above the
//! testaments, the pinned row opens the book's chapters right under it (the testaments stay as
//! they are), and its pin or a right-click unpins it. Pinning the book being read shows its
//! chapters twice, and both grids take clicks; a pinned book's highlights show as its ring. Its
//! own test binary because it sets `BIBLE_RS_OFFSCREEN` (see `book_picker_app.rs`).

use std::sync::Arc;
use std::time::{Duration, Instant};

use bible_core::books::{BookId, ChapterIdx, VerseRef};
use bible_core::study::HighlightColor;
use bible_rs::app::{BibleApp, Startup, initialize, online_counter};
use bible_rs::theme;
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, Shape};
use egui_kittest::Harness;

const JOHN: BookId = BookId(42);
const PHILIPPIANS: BookId = BookId(49);
const MARK: BookId = BookId(40);

/// Where the visible text `label` is drawn (its bounds), top to bottom.
fn find_rects(h: &Harness<'_, BibleApp>, label: &str) -> Vec<egui::Rect> {
    fn walk(shape: &Shape, clip: egui::Rect, label: &str, out: &mut Vec<egui::Rect>) {
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, clip, label, out)),
            Shape::Text(t) if t.galley.text() == label => {
                let r = t.visual_bounding_rect();
                if clip.contains(r.center()) {
                    out.push(r);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for c in &h.output().shapes {
        walk(&c.shape, c.clip_rect, label, &mut out);
    }
    out.sort_by(|a, b| a.center().y.total_cmp(&b.center().y));
    out
}

fn find(h: &Harness<'_, BibleApp>, label: &str) -> Vec<Pos2> {
    find_rects(h, label).iter().map(|r| r.center()).collect()
}

/// Closed shapes drawn inside `area`, filled (`true`: a filled pin's parts) or outlined.
fn closed_in(h: &Harness<'_, BibleApp>, area: egui::Rect, filled: bool) -> usize {
    fn walk(shape: &Shape, area: egui::Rect, filled: bool, n: &mut usize) {
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, area, filled, n)),
            Shape::Path(p)
                if p.closed
                    && (p.fill != egui::Color32::TRANSPARENT) == filled
                    && p.points.iter().all(|q| area.contains(*q)) =>
            {
                *n += 1;
            }
            _ => {}
        }
    }
    let mut n = 0;
    for c in &h.output().shapes {
        walk(&c.shape, area, filled, &mut n);
    }
    n
}

fn fills_in(h: &Harness<'_, BibleApp>, area: egui::Rect) -> usize {
    closed_in(h, area, true)
}

/// Rings (a stroked circle: a book's highlights and notes, all one colour) centred in `area`.
fn rings_in(h: &Harness<'_, BibleApp>, area: egui::Rect) -> usize {
    fn walk(shape: &Shape, area: egui::Rect, n: &mut usize) {
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, area, n)),
            Shape::Circle(c) if c.stroke.width > 0.0 && area.contains(c.center) => *n += 1,
            _ => {}
        }
    }
    let mut n = 0;
    for c in &h.output().shapes {
        walk(&c.shape, area, &mut n);
    }
    n
}

fn one(h: &Harness<'_, BibleApp>, label: &str) -> Pos2 {
    let found = find(h, label);
    assert_eq!(found.len(), 1, "{label:?} drawn at {found:?}");
    found[0]
}

fn click(h: &mut Harness<'_, BibleApp>, at: Pos2, button: PointerButton) {
    h.event(Event::PointerMoved(at));
    h.step();
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos: at, button, pressed, modifiers: Modifiers::NONE });
        h.step();
    }
    h.run_steps(4);
}

#[test]
fn a_pinned_book_sits_above_the_testaments() {
    // SAFETY: the only test in this binary, so nothing else reads the environment meanwhile.
    unsafe { std::env::set_var("BIBLE_RS_OFFSCREEN", "1") };
    let dir = std::env::temp_dir().join(format!("bible-rs-pin-books-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut init = initialize(dir.clone(), Arc::new(|| {})).expect("loading data");
    init.settings.onboarded = true;
    init.settings.translation = "kjv".into();
    init.settings.sidebar_open = true;
    let john3 = ChapterIdx::new(JOHN, 3).unwrap();
    init.settings.chapter = john3.0;
    let startup =
        Startup { t0: Instant::now(), probe: false, init: Duration::ZERO, first_ui: None, frames: 0 };
    let mut h = Harness::builder().with_size(egui::vec2(1280.0, 1000.0)).build_eframe(|cc| {
        cc.egui_ctx.set_fonts(theme::font_definitions());
        BibleApp::new(cc, init, online_counter(), startup)
    });
    h.run_steps(8);

    // Reading John opens the New Testament; right-click Philippians and pin it.
    let row = one(&h, "Philippians");
    click(&mut h, row, PointerButton::Secondary);
    let pin = one(&h, "Pin to Top");
    click(&mut h, pin, PointerButton::Primary);
    assert_eq!(h.state().settings.pinned(), [PHILIPPIANS]);

    // It now shows twice: pinned above the Old Testament, and still in the New Testament.
    let old = one(&h, "Old Testament");
    let rows = find(&h, "Philippians");
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert!(rows[0].y < old.y && rows[1].y > old.y, "{rows:?} vs Old Testament at {old:?}");

    // Clicking the pinned row opens Philippians' four chapters right under it, above the Old
    // Testament; the New Testament's Philippians stays closed and John's grid stays open.
    let here = h.state().reader.location.0;
    click(&mut h, rows[0], PointerButton::Primary);
    let old = one(&h, "Old Testament");
    let pinned = find(&h, "Philippians")[0];
    for n in 1..=4 {
        let chips: Vec<Pos2> = find(&h, &n.to_string()).into_iter().filter(|c| c.x < 300.0).collect();
        assert!(chips.iter().any(|c| c.y > pinned.y && c.y < old.y), "chapter {n} under the pin: {chips:?}");
    }
    assert!(find(&h, "5").iter().all(|c| c.x > 300.0 || c.y > old.y), "only Philippians' chapters");
    assert!(find(&h, "21").iter().any(|c| c.x < 300.0 && c.y > old.y), "John's grid is still open");
    let nt_row = find(&h, "Philippians")[1];
    let colossians = one(&h, "Colossians");
    assert!(
        colossians.y - nt_row.y < 40.0,
        "the New Testament's Philippians is closed: {nt_row:?} {colossians:?}"
    );
    assert_eq!(h.state().reader.location.0, here, "opening the chapters doesn't move the reader");

    // A chapter there reads it, without opening or scrolling to the testament's row.
    let chip = *find(&h, "2").iter().find(|c| c.x < 300.0 && c.y > pinned.y && c.y < old.y).unwrap();
    click(&mut h, chip, PointerButton::Primary);
    assert_eq!(h.state().reader.location.0, ChapterIdx::new(PHILIPPIANS, 2).unwrap());
    assert_eq!(find(&h, "Philippians")[0], pinned, "the list didn't scroll");
    let nt_row = find(&h, "Philippians")[1];
    assert!(one(&h, "Colossians").y - nt_row.y < 40.0, "still closed");

    // Clicking the pinned row again closes its chapters.
    click(&mut h, pinned, PointerButton::Primary);
    assert!(find(&h, "4").iter().all(|c| c.x > 300.0 || c.y > old.y));

    // Pin John too: pins keep their order.
    let john = find(&h, "John");
    let john_row = *john.iter().find(|p| p.y > old.y).expect("John in the New Testament");
    click(&mut h, john_row, PointerButton::Secondary);
    let pin = one(&h, "Pin to Top");
    click(&mut h, pin, PointerButton::Primary);
    assert_eq!(h.state().settings.pinned(), [PHILIPPIANS, JOHN], "in the order they were pinned");
    let john = find(&h, "John");
    let old = one(&h, "Old Testament");
    assert!(john[0].y < old.y && find(&h, "Philippians")[0].y < john[0].y);

    // Pinned rows carry a filled pin where other rows have their chevron: the New Testament's
    // count ends 8 pt before its 16 pt chevron lane.
    let nt = one(&h, "New Testament");
    let count = *find_rects(&h, "27").iter().find(|r| (r.center().y - nt.y).abs() < 2.0).expect("NT count");
    let pin_at = |y: f32| egui::pos2(count.right() + 16.0, y);
    let lane = |y: f32| egui::Rect::from_center_size(pin_at(y), egui::vec2(16.0, 16.0));
    assert!(fills_in(&h, lane(john[0].y)) >= 3, "a filled pin on the pinned row");
    assert_eq!(fills_in(&h, lane(old.y)), 0, "no pin on the testament rows");
    // And a filled book on the left, where the testaments keep the outline.
    let label = find_rects(&h, "Old Testament")[0];
    let left = |y: f32| {
        egui::Rect::from_min_max(
            egui::pos2(label.left() - 30.0, y - 10.0),
            egui::pos2(label.left() - 2.0, y + 10.0),
        )
    };
    assert!(fills_in(&h, left(john[0].y)) >= 1, "a filled book on the pinned row");
    assert_eq!(fills_in(&h, left(old.y)), 0, "an outline book on the testament rows");

    // Right-clicking a pinned row unpins it.
    let pinned = find(&h, "Philippians")[0];
    click(&mut h, pinned, PointerButton::Secondary);
    let unpin = one(&h, "Unpin");
    click(&mut h, unpin, PointerButton::Primary);
    assert_eq!(h.state().settings.pinned(), [JOHN]);

    // So does clicking its pin, without opening its chapters.
    let john = find(&h, "John")[0];
    h.event(Event::PointerMoved(john));
    h.run_steps(2);
    assert_eq!(h.output().platform_output.cursor_icon, egui::CursorIcon::Default, "pinned rows too");
    click(&mut h, pin_at(john.y), PointerButton::Primary);
    assert!(h.state().settings.pinned().is_empty());

    // Book rows in the testaments show an outline pin on hover (none otherwise), and clicking
    // it pins the book without opening its chapters or moving the reader.
    h.event(Event::PointerMoved(egui::pos2(700.0, 400.0)));
    h.run_steps(4);
    let mark = one(&h, "Mark");
    assert_eq!(closed_in(&h, lane(mark.y), false) + fills_in(&h, lane(mark.y)), 0, "no pin until hovered");
    h.event(Event::PointerMoved(mark));
    h.run_steps(4);
    assert_eq!(closed_in(&h, lane(mark.y), false), 1, "an outline pin on hover");
    let cursor = |h: &Harness<'_, BibleApp>| h.output().platform_output.cursor_icon;
    assert_eq!(cursor(&h), egui::CursorIcon::Default, "the row keeps the default cursor");
    assert_eq!(fills_in(&h, lane(mark.y)), 0);
    h.event(Event::PointerMoved(pin_at(mark.y)));
    h.run_steps(2);
    assert_eq!(cursor(&h), egui::CursorIcon::PointingHand, "the pin takes the pointing hand");
    let here = h.state().reader.location.0;
    click(&mut h, pin_at(mark.y), PointerButton::Primary);
    assert_eq!(h.state().settings.pinned(), [MARK]);
    assert_eq!(h.state().reader.location.0, here);
    let marks = find(&h, "Mark");
    assert_eq!(marks.len(), 2, "pinned above, still in the New Testament: {marks:?}");
    assert!(one(&h, "Luke").y - marks[1].y < 40.0, "Mark's chapters stay closed");
    assert!(marks[0].y < one(&h, "Old Testament").y);
    // Pinned, its testament row keeps a filled pin after the pointer leaves.
    h.event(Event::PointerMoved(egui::pos2(700.0, 400.0)));
    h.run_steps(4);
    assert!(fills_in(&h, lane(marks[1].y)) >= 3, "a filled pin on the testament row");

    // Back in John 3 with a highlight there, pin John: the book being read. Its pinned row shows
    // John's ring where the filled book goes, like its row in the New Testament.
    h.state_mut().study.set_highlight(VerseRef::new(JOHN, 3, 16), Some(HighlightColor::Green)).unwrap();
    h.state_mut().reader.jump(john3, None, false);
    h.run_steps(6);
    let john_row = *find(&h, "John").iter().find(|p| p.y > one(&h, "Old Testament").y).unwrap();
    click(&mut h, pin_at(john_row.y), PointerButton::Primary);
    assert_eq!(h.state().settings.pinned(), [MARK, JOHN]);
    let old = one(&h, "Old Testament");
    let john = find(&h, "John");
    assert_eq!(john.len(), 2, "{john:?}");
    assert!(john[0].y < old.y && john[1].y > old.y);
    assert_eq!(rings_in(&h, left(john[1].y)), 1, "a ring on the testament row");
    assert_eq!(rings_in(&h, left(john[0].y)), 1, "and on the pinned row");
    assert_eq!(fills_in(&h, left(john[0].y)), 0, "the ring takes the filled book's place");
    assert!(fills_in(&h, left(find(&h, "Mark")[0].y)) >= 1, "unmarked Mark keeps its filled book");

    // Opening the pinned row puts a second grid of John's chapters on screen; each takes clicks.
    click(&mut h, john[0], PointerButton::Primary);
    let old = one(&h, "Old Testament");
    let chip = |h: &Harness<'_, BibleApp>, n: u8, pinned: bool| {
        let found: Vec<Pos2> = find(h, &n.to_string())
            .into_iter()
            .filter(|c| c.x < 300.0 && (c.y < old.y) == pinned && c.y > john[0].y)
            .collect();
        assert_eq!(found.len(), 1, "chapter {n} (pinned: {pinned}) at {found:?}");
        found[0]
    };
    let pinned_5 = chip(&h, 5, true);
    let testament_7 = chip(&h, 7, false);
    click(&mut h, pinned_5, PointerButton::Primary);
    assert_eq!(h.state().reader.location.0, ChapterIdx::new(JOHN, 5).unwrap(), "the pinned grid");
    click(&mut h, testament_7, PointerButton::Primary);
    assert_eq!(h.state().reader.location.0, ChapterIdx::new(JOHN, 7).unwrap(), "the testament's grid");
    click(&mut h, pinned_5, PointerButton::Primary);
    assert_eq!(h.state().reader.location.0, ChapterIdx::new(JOHN, 5).unwrap(), "the pinned grid again");
    let _ = std::fs::remove_dir_all(&dir);
}
