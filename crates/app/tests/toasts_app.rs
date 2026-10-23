//! Toasts in the running app: a stack of toasts of different widths stays on one centre line as
//! the oldest goes, with motion off and on, and toggling a bookmark again and again shows one
//! toast with its latest state. Its own test binary because it sets `BIBLE_RS_OFFSCREEN` (see
//! `book_picker_app.rs`).

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bible_core::books::{BookId, ChapterIdx};
use bible_rs::app::{BibleApp, Startup, initialize, online_counter};
use bible_rs::theme;
use eframe::egui::{self, Key, Modifiers};
use egui_kittest::Harness;

const JOHN: BookId = BookId(42);

/// The app reading John 3 in a 1280×800 window, each step `step_dt` seconds apart.
fn app(dir: &Path, step_dt: f32) -> Harness<'static, BibleApp> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).unwrap();
    let mut init = initialize(dir.to_path_buf(), Arc::new(|| {})).expect("loading data");
    init.settings.onboarded = true;
    init.settings.translation = "kjv".into();
    init.settings.chapter = ChapterIdx::new(JOHN, 3).unwrap().0;
    let startup =
        Startup { t0: Instant::now(), probe: false, init: Duration::ZERO, first_ui: None, frames: 0 };
    let mut h = Harness::builder().with_size(egui::vec2(1280.0, 800.0)).with_step_dt(step_dt).build_eframe(
        move |cc| {
            cc.egui_ctx.set_fonts(theme::font_definitions());
            BibleApp::new(cc, init, online_counter(), startup)
        },
    );
    h.run_steps(4);
    h
}

/// Each toast capsule painted last frame, with its fill's alpha: 40 high, radius 20, unblurred
/// (the shadow beneath is blurred). Where it was painted, not where its area ended up: an area
/// moved after its content was laid out records the new place but showed the old one.
fn capsules(h: &Harness<'_, BibleApp>) -> Vec<(egui::Rect, u8)> {
    h.output()
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Rect(r)
                if r.rect.height() == 40.0 && r.corner_radius.nw == 20 && r.blur_width == 0.0 =>
            {
                Some((r.rect, r.fill.a()))
            }
            _ => None,
        })
        .collect()
}

fn centres(h: &Harness<'_, BibleApp>) -> Vec<f32> {
    capsules(h).iter().map(|(r, _)| r.center().x).collect()
}

fn assert_one_centre_line(h: &Harness<'_, BibleApp>) {
    let c = centres(h);
    assert_eq!(c.len(), h.state().toast_areas().len(), "a capsule for each toast");
    assert!(c.iter().all(|x| (x - c[0]).abs() < 1.0), "toasts off the centre line: {c:?}");
}

/// Alternating widths, as toggling a bookmark used to stack them.
const MESSAGES: [&str; 2] = ["Bookmarked John 3:8", "Removed bookmark John 3:8"];

#[test]
fn toasts_stay_centred_and_bookmark_toasts_collapse() {
    // SAFETY: the only test in this binary, so nothing else reads the environment meanwhile.
    unsafe { std::env::set_var("BIBLE_RS_OFFSCREEN", "1") };
    let dir = std::env::temp_dir().join(format!("bible-rs-toasts-{}", std::process::id()));

    // Still (offscreen renders, like Reduce Motion): each step is 0.25 s.
    let mut h = app(&dir, 0.25);
    for (i, m) in MESSAGES.iter().cycle().take(4).enumerate() {
        let ctx = h.ctx.clone();
        h.state_mut().toast(&ctx, *m);
        h.step();
        if i == 0 {
            h.run_steps(3);
        }
    }
    h.step(); // a new area's first frame only measures it
    assert_eq!(centres(&h).len(), 4);
    assert_one_centre_line(&h);

    // Every frame as the oldest goes: the ones left keep to the line (they zigzagged by half the
    // difference in width when the oldest went).
    for _ in 0..16 {
        h.step();
        if !h.state().toast_areas().is_empty() {
            assert_one_centre_line(&h);
        }
    }
    h.run_steps(20);
    assert!(h.state().toast_areas().is_empty());

    // ⌘D four times on one verse: one toast, saying where it ended up.
    for _ in 0..4 {
        h.key_press_modifiers(Modifiers::COMMAND, Key::D);
        h.step();
    }
    let shown: Vec<_> = h.state().toast_areas().into_iter().map(|(m, _)| m.to_string()).collect();
    assert_eq!(shown.len(), 1, "{shown:?}");
    assert!(shown[0].starts_with("Removed bookmark "), "{shown:?}");
    h.key_press_modifiers(Modifiers::COMMAND, Key::D);
    h.step();
    let shown: Vec<_> = h.state().toast_areas().into_iter().map(|(m, _)| m.to_string()).collect();
    assert_eq!(shown.len(), 1, "{shown:?}");
    assert!(shown[0].starts_with("Bookmarked "), "{shown:?}");
    drop(h);

    // With motion, at 50 frames a second: the oldest fades out over its last 0.2 s, then the
    // ones above slide down 48 pt into its place over 0.15 s, all on the centre line.
    if bible_rs::motion::system_reduces_motion() {
        eprintln!("skipping the motion half: Reduce Motion is on in System Settings");
        let _ = std::fs::remove_dir_all(&dir);
        return;
    }
    // SAFETY: as above.
    unsafe { std::env::set_var("BIBLE_RS_SHOT_FILMSTRIP", "1") };
    let mut h = app(&dir, 0.02);
    for m in MESSAGES.iter().cycle().take(3) {
        let ctx = h.ctx.clone();
        h.state_mut().toast(&ctx, *m);
        h.run_steps(50); // 1 s apart
    }
    let first = capsules(&h);
    assert_eq!(first.len(), 3);
    let full = first[0].1;
    assert!(first.iter().all(|&(_, a)| a == full), "nothing fades yet: {first:?}");
    // The oldest is lowest; the next sits one capsule and an 8 pt gap above it.
    let low = first.iter().map(|(r, _)| r.bottom()).fold(f32::MIN, f32::max);
    let (mut fading, mut sliding, mut gone) = (0, Vec::new(), false);
    // The oldest is 3 s old: it fades from 3.8 s and goes at 4 s; the next starts to fade at
    // 4.8 s, so stop at 4.5 s.
    for _ in 0..75 {
        h.step();
        assert_one_centre_line(&h);
        let now = capsules(&h);
        if now.len() == 3 {
            assert!(!gone, "the oldest came back");
            fading += now.iter().any(|&(_, a)| a > 0 && a < full) as usize;
        } else {
            gone = true;
            assert_eq!(now.len(), 2);
            let bottom = now.iter().map(|(r, _)| r.bottom()).fold(f32::MIN, f32::max);
            sliding.push(bottom);
        }
    }
    assert!(gone, "the oldest toast should go after 4 s");
    // 0.2 s at 50 frames a second is 10 frames; the first and last may round to an end.
    assert!(fading >= 8, "the oldest faded over {fading} frames");
    // From its own place 48 pt up, down into the oldest's, passing through the frames between.
    assert!((sliding[0] - (low - 48.0)).abs() < 0.5, "slid from {sliding:?}, not {}", low - 48.0);
    assert!(sliding.windows(2).all(|w| w[1] >= w[0]), "slid back up: {sliding:?}");
    let between = sliding.iter().filter(|&&b| b > low - 48.0 + 1.0 && b < low - 1.0).count();
    assert!((3..=8).contains(&between), "a 0.15 s slide took {between} frames: {sliding:?}");
    assert!((sliding.last().unwrap() - low).abs() < 0.5, "slid to {sliding:?}, not {low}");
    let _ = std::fs::remove_dir_all(&dir);
}
