//! bible-rs: a fast, native Bible reader (library part: UI, reader, layout).
//!
//! The binary (`main.rs`) only parses arguments and launches [`run`]; keeping the UI in a
//! library lets benchmarks and integration tests drive the layout engine directly.

pub mod actions;
pub mod app;
pub mod book_picker;
pub mod cli;
pub mod compare_popover;
pub mod icons;
pub mod import_sheet;
pub mod keymap;
#[cfg(target_os = "macos")]
pub mod macos_menu;
#[cfg(target_os = "macos")]
pub mod macos_window;
pub mod menu;
pub mod note_popover;
pub mod offline;
pub mod onboarding;
pub mod pages;
pub mod palette;
pub mod panel;
pub mod probe;
pub mod reader;
pub mod reading;
pub mod settings_ui;
pub mod share_sheet;
pub mod shortcuts_overlay;
pub mod sidebar;
pub mod theme;
pub mod time;
pub mod verse_menu;
pub mod widgets;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use eframe::egui;

pub fn data_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("BIBLE_RS_HOME") {
        return PathBuf::from(d);
    }
    dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("bible-rs")
}

/// `BIBLE_RS_OFFSCREEN=1`: the app is rendered without a window (`examples/shot.rs`).
pub fn offscreen() -> bool {
    std::env::var_os("BIBLE_RS_OFFSCREEN").is_some()
}

/// The startup, scroll and sidebar probes (`scripts/bench-startup.sh`, `scripts/bench-sidebar.sh`)
/// measure the reader, so they skip the first-launch screens.
pub fn probing() -> bool {
    std::env::var_os("BIBLE_RS_STARTUP_PROBE").is_some()
        || std::env::var_os("BIBLE_RS_SCROLL_PROBE").is_some()
        || std::env::var_os("BIBLE_RS_SIDEBAR_PROBE").is_some()
}

/// `BIBLE_RS_TRACE_STARTUP=1` prints the time of each startup phase to stderr.
pub fn trace(t0: Instant, what: &str) {
    if std::env::var_os("BIBLE_RS_TRACE_STARTUP").is_some() {
        eprintln!("{:>8.2} ms  {what}", t0.elapsed().as_secs_f64() * 1000.0);
    }
}

/// The renderer: `BIBLE_RS_RENDERER=glow|wgpu` overrides the default.
pub fn renderer() -> eframe::Renderer {
    match std::env::var("BIBLE_RS_RENDERER").as_deref() {
        Ok("wgpu") => eframe::Renderer::Wgpu,
        Ok("glow") => eframe::Renderer::Glow,
        // macOS: Metal. OpenGL is deprecated there, and on the macOS CI runner it held ~15 fps
        // while scrolling where Metal held the display's 60.
        _ if cfg!(target_os = "macos") => eframe::Renderer::Wgpu,
        // Elsewhere glow, which creates its context about twice as fast as wgpu.
        _ => eframe::Renderer::Glow,
    }
}

/// wgpu settings: the low-power GPU and, on macOS, only the Metal backend
/// (`WGPU_POWER_PREF` and `WGPU_BACKEND` still override).
pub fn wgpu_options() -> eframe::egui_wgpu::WgpuConfiguration {
    let mut options = eframe::egui_wgpu::WgpuConfiguration::default();
    if let eframe::egui_wgpu::WgpuSetup::CreateNew(setup) = &mut options.wgpu_setup {
        // Reading text never needs a discrete GPU; waking one costs start-up time and battery.
        if std::env::var_os("WGPU_POWER_PREF").is_none() {
            setup.power_preference = eframe::wgpu::PowerPreference::LowPower;
        }
        if cfg!(target_os = "macos") && std::env::var_os("WGPU_BACKEND").is_none() {
            setup.instance_descriptor.backends = eframe::wgpu::Backends::METAL;
        }
    }
    options
}

/// `BIBLE_RS_BACKGROUND=1` opens the window without taking focus from the user (screenshots,
/// benchmarks).
#[cfg(target_os = "macos")]
pub fn background_launch() -> Option<eframe::EventLoopBuilderHook> {
    std::env::var_os("BIBLE_RS_BACKGROUND")?;
    Some(Box::new(|builder| {
        use winit::platform::macos::EventLoopBuilderExtMacOS;
        builder.with_activate_ignoring_other_apps(false);
    }))
}

#[cfg(not(target_os = "macos"))]
pub fn background_launch() -> Option<eframe::EventLoopBuilderHook> {
    None
}

/// Whether this process runs from inside a macOS `.app` bundle.
fn in_app_bundle() -> bool {
    cfg!(target_os = "macos")
        && std::env::current_exe().is_ok_and(|p| p.to_string_lossy().contains(".app/Contents/MacOS/"))
}

/// Runs the CLI if arguments name a command, otherwise the GUI.
pub fn run() -> eframe::Result {
    let t0 = Instant::now();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = data_dir();
    // First launch (no study database yet) opens at the onboarding screens' size.
    let first_run = !dir.join("study.sqlite").exists() && !probing();
    if let Some(result) = cli::run(&args, &dir) {
        if let Err(e) = result {
            eprintln!("error: {e:#}");
            std::process::exit(1);
        }
        return Ok(());
    }

    // Load the study database, settings and the current translation on a background thread
    // while the window and GPU context are created.
    let ctx_cell: Arc<OnceLock<egui::Context>> = Arc::new(OnceLock::new());
    let online_gen = app::online_counter();
    let notify = {
        let ctx_cell = ctx_cell.clone();
        let online_gen = online_gen.clone();
        Arc::new(move || {
            app::bump(&online_gen);
            if let Some(ctx) = ctx_cell.get() {
                ctx.request_repaint();
            }
        }) as Arc<dyn Fn() + Send + Sync>
    };
    let init = std::thread::Builder::new()
        .name("init".into())
        .spawn(move || app::initialize(dir, notify))
        .expect("spawn init thread");

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("bible-rs")
        .with_app_id("bible-rs")
        .with_inner_size(if first_run { onboarding::SIZE } else { egui::vec2(1280.0, 800.0) })
        .with_min_inner_size([620.0, 420.0]);
    if in_app_bundle() {
        // The bundle's .icns is already the Dock icon; an empty icon stops eframe from decoding
        // and installing one at runtime during the first frame.
        viewport = viewport.with_icon(Arc::new(egui::IconData::default()));
    } else if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon-256.png")) {
        viewport = viewport.with_icon(Arc::new(icon));
    }
    if cfg!(target_os = "macos") {
        // Unified toolbar: content runs under a transparent title bar, like native reading apps.
        viewport =
            viewport.with_fullsize_content_view(true).with_titlebar_shown(false).with_title_shown(false);
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: renderer(),
        wgpu_options: wgpu_options(),
        centered: true,
        event_loop_builder: background_launch(),
        ..Default::default()
    };
    let probe = std::env::var_os("BIBLE_RS_STARTUP_PROBE").is_some();
    eframe::run_native(
        "bible-rs",
        options,
        Box::new(move |cc| {
            trace(t0, "window + GPU context ready");
            let _ = ctx_cell.set(cc.egui_ctx.clone());
            cc.egui_ctx.set_fonts(theme::font_definitions());
            let init = init.join().map_err(|_| "initialization panicked")??;
            trace(t0, "init joined (study db, library, translation decoded)");
            let startup =
                app::Startup { t0, probe, init: std::time::Duration::ZERO, first_ui: None, frames: 0 };
            Ok(Box::new(app::BibleApp::new(cc, init, online_gen, startup)))
        }),
    )
}
