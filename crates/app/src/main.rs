//! bible-rs: a fast, native Bible reader.
//!
//! Environment:
//! - `BIBLE_RS_HOME` — data directory override (study database, imports, caches)
//! - `BIBLE_RS_RENDERER=glow|wgpu` — force a renderer (default: wgpu on macOS, glow elsewhere)
//! - `BIBLE_RS_STARTUP_PROBE=1` — print startup timings as JSON after the first frame and exit

mod actions;
mod app;
mod cli;
mod icons;
#[cfg(target_os = "macos")]
mod macos_menu;
mod palette;
mod panel;
mod reader;
mod settings_ui;
mod sidebar;
mod theme;
mod widgets;

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use eframe::egui;

fn data_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("BIBLE_RS_HOME") {
        return PathBuf::from(d);
    }
    dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("bible-rs")
}

fn main() -> eframe::Result {
    let t0 = Instant::now();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = data_dir();
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

    let renderer = match std::env::var("BIBLE_RS_RENDERER").as_deref() {
        Ok("wgpu") => eframe::Renderer::Wgpu,
        Ok("glow") => eframe::Renderer::Glow,
        _ if cfg!(target_os = "macos") => eframe::Renderer::Wgpu,
        _ => eframe::Renderer::Glow,
    };
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("bible-rs")
        .with_app_id("bible-rs")
        .with_inner_size([1280.0, 860.0])
        .with_min_inner_size([620.0, 420.0]);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon-256.png")) {
        viewport = viewport.with_icon(Arc::new(icon));
    }
    if cfg!(target_os = "macos") {
        // Unified toolbar: content runs under a transparent title bar, like native reading apps.
        viewport = viewport.with_fullsize_content_view(true).with_titlebar_shown(false).with_title_shown(false);
    }
    let options = eframe::NativeOptions { viewport, renderer, centered: true, ..Default::default() };
    let probe = std::env::var_os("BIBLE_RS_STARTUP_PROBE").is_some();
    eframe::run_native(
        "bible-rs",
        options,
        Box::new(move |cc| {
            let _ = ctx_cell.set(cc.egui_ctx.clone());
            cc.egui_ctx.set_fonts(theme::font_definitions());
            let init = init.join().map_err(|_| "initialization panicked")??;
            let startup = app::Startup { t0, probe, init: std::time::Duration::ZERO, first_ui: None, frames: 0 };
            Ok(Box::new(app::BibleApp::new(cc, init, online_gen, startup)))
        }),
    )
}
