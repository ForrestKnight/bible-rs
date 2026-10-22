//! Renders the app offscreen to a PNG, for design comparisons and screenshots, without opening a
//! window (so it works whatever is on screen, and on a machine without one):
//!
//! ```text
//! BIBLE_RS_HOME=<data dir> BIBLE_RS_DEMO="palette:jn 3:16" \
//!     cargo run --release --example shot -- out.png [1280x800]
//! ```
//!
//! `BIBLE_RS_SHOT_CLICKS` and `BIBLE_RS_SHOT_HOVER` (points, `x,y`) click and point after the
//! demo steps, for states only the pointer reaches, such as an open menu.
//! `BIBLE_RS_SHOT_SCRIPT` then runs steps in order, `;` apart: `click:x,y`, `shift-click:x,y`,
//! `hover:x,y`, `drag:x,y>x,y` and `key:1`. A point can also be a place in the chapter being
//! read, `verse.char`: `drag:16.4>16.26` drags from char 4 of verse 16 to char 26. Each step
//! prints the points it used.
//! `BIBLE_RS_SHOT_PAUSE_MS` waits that long after each frame, so chapters of an online translation
//! can arrive before the picture is taken.
//!
//! `BIBLE_RS_SHOT_FILMSTRIP=<n>` then presses ⌘B and writes the next n frames at 120 Hz as
//! `<out>-01.png` …, to see the sidebar's slide frame by frame. Other renders show transitions
//! already finished.
//!
//! egui's test harness drives the real app with wgpu at 2× scale. On macOS the window's traffic
//! lights belong to AppKit, so they are drawn onto the image where AppKit puts them.

use std::sync::Arc;
use std::time::{Duration, Instant};

use bible_rs::app::{BibleApp, Startup, bump, initialize, online_counter};
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
    // Online text that arrives bumps the counter, so the reader lays its chapters out again.
    let online = online_counter();
    let notify = {
        let online = online.clone();
        Arc::new(move || bump(&online)) as Arc<dyn Fn() + Send + Sync>
    };
    let init = initialize(bible_rs::data_dir(), notify).expect("loading data");
    let startup =
        Startup { t0: Instant::now(), probe: false, init: Duration::ZERO, first_ui: None, frames: 0 };
    let filmstrip: Option<usize> = std::env::var("BIBLE_RS_SHOT_FILMSTRIP").ok().and_then(|s| s.parse().ok());
    let mut builder = Harness::builder().with_size(egui::vec2(w, h)).with_pixels_per_point(2.0);
    if filmstrip.is_some() {
        builder = builder.with_step_dt(1.0 / 120.0);
    }
    let mut harness = builder.wgpu().build_eframe(|cc| {
        cc.egui_ctx.set_fonts(theme::font_definitions());
        BibleApp::new(cc, init, online, startup)
    });
    // The harness sets its own (dark) egui theme after the app has applied the reading theme,
    // which would leave egui's widgets (menus, fields) in stock dark style.
    let ctx = harness.ctx.clone();
    theme::apply(&ctx, harness.state().settings.theme);
    ctx.all_styles_mut(|style| style.animation_time = 0.0);
    // A font atlas as large as the GPU's (egui assumes 2048 px without one): a small atlas fills
    // at 2× and egui rebuilds it, leaving the reader's cached text pointing at the old glyphs.
    harness.input_mut().max_texture_side = Some(8192);
    harness.step();
    harness.state_mut().reader.invalidate();
    let demo_steps = std::env::var("BIBLE_RS_DEMO").map(|s| s.split(';').count()).unwrap_or(0);
    let steps =
        std::env::var("BIBLE_RS_SHOT_STEPS").ok().and_then(|s| s.parse().ok()).unwrap_or(30 + 4 * demo_steps);
    let pause =
        std::env::var("BIBLE_RS_SHOT_PAUSE_MS").ok().and_then(|s| s.parse().ok()).map(Duration::from_millis);
    for _ in 0..steps {
        harness.step();
        if let Some(pause) = pause {
            std::thread::sleep(pause);
        }
    }
    // `BIBLE_RS_SHOT_CLICKS="x,y;x,y"`: click those points in turn (opening a menu from its
    // button, say), then `BIBLE_RS_SHOT_HOVER="x,y"`: leave the pointer there.
    let point = |s: &str| {
        let (x, y) = s.split_once(',')?;
        Some(egui::pos2(x.trim().parse().ok()?, y.trim().parse().ok()?))
    };
    let clicks = std::env::var("BIBLE_RS_SHOT_CLICKS").unwrap_or_default();
    for pos in clicks.split(';').filter_map(point) {
        harness.hover_at(pos);
        harness.step();
        for pressed in [true, false] {
            harness.event(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
            harness.step();
        }
        harness.run_steps(8);
    }
    if let Some(pos) = std::env::var("BIBLE_RS_SHOT_HOVER").ok().as_deref().and_then(point) {
        harness.hover_at(pos);
        harness.run_steps(8);
    }
    // `BIBLE_RS_SHOT_KEYS="ArrowRight;Enter"`: then press those keys (egui's names) in turn, for
    // states the keyboard reaches.
    let keys = std::env::var("BIBLE_RS_SHOT_KEYS").unwrap_or_default();
    for name in keys.split(';').filter(|s| !s.is_empty()) {
        let Some(key) = egui::Key::from_name(name.trim()) else {
            eprintln!("BIBLE_RS_SHOT_KEYS: unknown key {name:?}");
            continue;
        };
        for pressed in [true, false] {
            harness.event(egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
            harness.step();
        }
        harness.run_steps(4);
    }
    let script = std::env::var("BIBLE_RS_SHOT_SCRIPT").unwrap_or_default();
    for step in script.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        run_step(&mut harness, step);
    }
    let save = |harness: &mut Harness<'_, BibleApp>, path: &str| {
        let mut img = harness.render().expect("rendering");
        if cfg!(target_os = "macos") && std::env::var_os("BIBLE_RS_NO_TRAFFIC_LIGHTS").is_none() {
            traffic_lights(&mut img);
        }
        img.save(path).expect("writing the image");
    };
    let Some(frames) = filmstrip else {
        save(&mut harness, out);
        return;
    };
    // The frame that handles ⌘B (the harness runs one frame per event, so the key's release
    // comes with the slide's first frame), then the slide.
    let key = |pressed| egui::Event::Key {
        key: egui::Key::B,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    };
    harness.event(key(true));
    harness.step();
    harness.event(key(false));
    let stem = out.strip_suffix(".png").unwrap_or(out);
    for i in 1..=frames {
        harness.step();
        save(&mut harness, &format!("{stem}-{i:02}.png"));
    }
}

fn point(s: &str) -> Option<egui::Pos2> {
    let (x, y) = s.split_once(',')?;
    Some(egui::pos2(x.trim().parse().ok()?, y.trim().parse().ok()?))
}

/// A place in the chapter being read, `verse.char`, on screen.
fn text_point(harness: &Harness<'_, BibleApp>, s: &str) -> Option<egui::Pos2> {
    let (verse, offset) = s.split_once('.')?;
    let reader = &harness.state().reader;
    let c = reader.location.0;
    let verse = bible_core::VerseRef::new(c.book(), c.chapter(), verse.trim().parse().ok()?);
    reader.point_at(bible_rs::reader::TextPos { verse, offset: offset.trim().parse().ok()?, column: 0 })
}

fn press(harness: &mut Harness<'_, BibleApp>, pos: egui::Pos2, modifiers: egui::Modifiers) {
    harness.event(egui::Event::ModifiersChanged(modifiers));
    harness.hover_at(pos);
    harness.step();
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers,
        });
        harness.step();
    }
    harness.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    harness.run_steps(4);
}

fn drag(harness: &mut Harness<'_, BibleApp>, from: egui::Pos2, to: egui::Pos2) {
    harness.hover_at(from);
    harness.step();
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    harness.event(button(from, true));
    harness.step();
    for i in 1..=10 {
        harness.event(egui::Event::PointerMoved(from + (to - from) * (i as f32 / 10.0)));
        harness.step();
    }
    harness.event(button(to, false));
    harness.run_steps(4);
}

/// A point, `x,y`, or a place in the chapter being read, `verse.char`.
fn resolve(harness: &Harness<'_, BibleApp>, s: &str) -> Option<egui::Pos2> {
    point(s).or_else(|| text_point(harness, s))
}

/// One step of `BIBLE_RS_SHOT_SCRIPT`. Each prints the points it used, to repeat it elsewhere.
fn run_step(harness: &mut Harness<'_, BibleApp>, step: &str) {
    let (kind, arg) = step.split_once(':').unwrap_or((step, ""));
    let at = |p: egui::Pos2| format!("{},{}", p.x, p.y);
    match kind {
        "click" | "shift-click" | "hover" => {
            let Some(p) = resolve(harness, arg) else {
                eprintln!("BIBLE_RS_SHOT_SCRIPT: {step:?} isn't on screen");
                return;
            };
            eprintln!("{step} → {kind}:{}", at(p));
            match kind {
                "click" => press(harness, p, egui::Modifiers::NONE),
                "shift-click" => press(harness, p, egui::Modifiers::SHIFT),
                _ => {
                    harness.hover_at(p);
                    harness.run_steps(4);
                }
            }
        }
        "drag" => {
            let ends =
                arg.split_once('>').and_then(|(a, b)| Some((resolve(harness, a)?, resolve(harness, b)?)));
            let Some((a, b)) = ends else {
                eprintln!("BIBLE_RS_SHOT_SCRIPT: {step:?} isn't on screen");
                return;
            };
            eprintln!("{step} → drag:{}>{}", at(a), at(b));
            drag(harness, a, b);
        }
        "key" => match egui::Key::from_name(arg.trim()) {
            Some(key) => {
                harness.key_press(key);
                harness.run_steps(4);
            }
            None => eprintln!("BIBLE_RS_SHOT_SCRIPT: unknown key {arg:?}"),
        },
        _ => eprintln!("BIBLE_RS_SHOT_SCRIPT: unknown step {step:?}"),
    }
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
