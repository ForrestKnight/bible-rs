//! Omarchy's theme as a reading theme (Linux).
//!
//! Omarchy keeps the active theme's palette in `~/.local/state/omarchy/current/theme/colors.toml`
//! and its name in `theme.name` next to it; `omarchy-theme-set` replaces both when the theme
//! changes. When those files exist the app offers an "Omarchy" theme that is derived from that
//! palette the way the design's own themes are built (a [`Palette`] with the same roles), and a
//! background thread notices a change within half a second and repaints. Nothing here depends on
//! Omarchy at build time: elsewhere the files don't exist and the theme is never offered.
//!
//! The palette file is flat TOML (`key = "#rrggbb"`), read here without a TOML parser: only
//! `background`, `foreground` and `accent` are needed, the rest refine the result when present.
//! Older themes name the same colours `bg`, `fg` and `color0..15`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use eframe::egui::{self, Color32};

use crate::theme::{DARK, LIGHT, Palette, SWATCHES};

/// The loaded theme: its palette, whether it is dark, and Omarchy's name for it.
pub struct Loaded {
    /// Leaked once per change, so it can be handed out as `&'static` like the design's palettes.
    pub palette: &'static Palette,
    pub dark: bool,
    pub name: String,
}

static CURRENT: RwLock<Option<Arc<Loaded>>> = RwLock::new(None);
/// Bumped on every reload; [`crate::theme::apply`] records the generation it applied.
static GENERATION: AtomicU64 = AtomicU64::new(0);
static APPLIED: AtomicU64 = AtomicU64::new(0);

/// Where Omarchy keeps the active theme: `$XDG_STATE_HOME/omarchy/current`, or the older
/// `~/.config/omarchy/current`.
fn current_dir() -> Option<PathBuf> {
    let state = dirs::state_dir().map(|d| d.join("omarchy/current"));
    let legacy = dirs::config_dir().map(|d| d.join("omarchy/current"));
    [state, legacy].into_iter().flatten().find(|d| d.join("theme/colors.toml").is_file())
}

fn read(dir: &Path) -> Option<(String, String)> {
    let colors = std::fs::read_to_string(dir.join("theme/colors.toml")).ok()?;
    let name = std::fs::read_to_string(dir.join("theme.name")).unwrap_or_default();
    Some((colors, name.trim().to_string()))
}

/// Loads the theme if it hasn't been yet. Whether Omarchy is here is decided once, at startup.
fn load_once() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if let Some(dir) = current_dir()
            && let Some((colors, name)) = read(&dir)
        {
            install(&colors, name);
        }
    });
}

fn install(colors: &str, name: String) -> bool {
    let Some((palette, dark)) = palette_from_toml(colors) else { return false };
    let loaded = Loaded { palette: Box::leak(Box::new(palette)), dark, name };
    *CURRENT.write().unwrap() = Some(Arc::new(loaded));
    GENERATION.fetch_add(1, Ordering::Relaxed);
    true
}

/// The Omarchy theme, if this machine has one.
pub fn current() -> Option<Arc<Loaded>> {
    load_once();
    CURRENT.read().unwrap().clone()
}

pub fn available() -> bool {
    current().is_some()
}

/// Omarchy's name for the active theme ("tokyo-night"), for the theme card.
pub fn theme_name() -> Option<String> {
    current().map(|l| l.name.clone())
}

/// Counts reloads, so a layout cached under the Omarchy theme can tell the palette changed.
pub fn generation() -> u64 {
    GENERATION.load(Ordering::Relaxed)
}

/// True once the palette has changed since the theme was last applied.
pub fn changed_since_applied() -> bool {
    GENERATION.load(Ordering::Relaxed) != APPLIED.load(Ordering::Relaxed)
}

pub fn mark_applied() {
    APPLIED.store(GENERATION.load(Ordering::Relaxed), Ordering::Relaxed);
}

/// Watches the theme files and repaints when they change. Polling every half second costs one
/// small read; a theme change is a directory swap, so file watches would need re-arming anyway.
pub fn watch(ctx: egui::Context) {
    let Some(dir) = current_dir() else { return };
    let _ = std::thread::Builder::new().name("omarchy-theme".into()).spawn(move || {
        let mut last = read(&dir);
        loop {
            std::thread::sleep(Duration::from_millis(500));
            let now = read(&dir);
            if now != last {
                if let Some((colors, name)) = &now
                    && install(colors, name.clone())
                {
                    ctx.request_repaint();
                }
                last = now;
            }
        }
    });
}

/// `key = "#rrggbb"` lines, tables and comments skipped. Values are kept as written.
fn parse(toml: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for line in toml.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let value = value.trim().split('#').take(2).collect::<Vec<_>>();
        // The value is quoted and starts with '#', so the comment split keeps the colour: `"` then
        // `rrggbb"`. Everything after the closing quote is a comment.
        let joined = value.join("#");
        let value = joined.trim().trim_matches(|c| c == '"' || c == '\'').trim();
        out.insert(key.trim().to_string(), value.to_string());
    }
    out
}

fn hex(s: &str) -> Option<Color32> {
    let s = s.trim().trim_start_matches('#');
    let s = s.split('"').next()?;
    if s.len() != 6 && s.len() != 8 {
        return None;
    }
    let n = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok();
    Some(Color32::from_rgb(n(0)?, n(2)?, n(4)?))
}

fn luminance(c: Color32) -> f32 {
    let lin = |v: u8| {
        let v = v as f32 / 255.0;
        if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

fn alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

/// A palette in the design's roles from Omarchy's keys. The mixes are the ratios the design's own
/// light and dark palettes use between paper and ink (sidebar 5 %, secondary ink a third of the
/// way to paper, tertiary 58 %), so an Omarchy theme sits where Light and Dark do.
pub fn palette_from_toml(toml: &str) -> Option<(Palette, bool)> {
    let keys = parse(toml);
    let get = |names: &[&str]| names.iter().find_map(|n| keys.get(*n)).and_then(|v| hex(v));
    let bg = get(&["background", "bg", "color0"])?;
    let fg = get(&["foreground", "fg", "color7"])?;
    let dark = match keys.get("mode").map(String::as_str) {
        Some("dark") => true,
        Some("light") => false,
        _ => luminance(bg) < 0.4,
    };
    let accent = get(&["accent", "blue", "color4"]).unwrap_or(fg);
    let red = get(&["red", "color1"]).unwrap_or(if dark { DARK.rubric } else { LIGHT.rubric });
    let green = get(&["green", "color2"]).unwrap_or(if dark { DARK.success } else { LIGHT.success });
    let yellow = get(&["yellow", "color3"]).unwrap_or(Color32::from_rgb(0xE5, 0xC0, 0x7B));
    let blue = get(&["blue", "color4"]).unwrap_or(accent);
    let magenta = get(&["magenta", "color5"]).unwrap_or(Color32::from_rgb(0xC6, 0x78, 0xDD));
    let orange = get(&["orange"]).unwrap_or(mix(red, yellow, 0.5));
    let base = if dark { &DARK } else { &LIGHT };

    let toward = |t: f32| mix(bg, fg, t);
    // Text on the accent: black contrasts more than white above a relative luminance of 0.18.
    let on_accent = if luminance(accent) > 0.18 { mix(bg, Color32::BLACK, 0.4) } else { Color32::WHITE };
    let wash = if dark { 0.65 } else { 0.6 };
    let highlight = |c: Color32| mix(c, bg, wash);
    let ink_3 = toward(1.0 - 0.58);
    let palette = Palette {
        paper: bg,
        sidebar: toward(0.05),
        surface: if dark { toward(0.07) } else { bg },
        toolbar: alpha(bg, 204),
        glass: alpha(if dark { toward(0.11) } else { bg }, 224),
        field: if dark { toward(0.035) } else { bg },
        raised: if dark { toward(0.2) } else { bg },
        fill: alpha(fg, if dark { 15 } else { 13 }),
        fill_strong: alpha(fg, if dark { 28 } else { 26 }),
        separator: toward(0.11),
        border: alpha(fg, 31),
        scrim: alpha(Color32::BLACK, if dark { 115 } else { 56 }),
        ink: fg,
        ink_2: toward(0.67),
        ink_3,
        verse: mix(accent, ink_3, 0.5),
        rubric: red,
        accent,
        on_accent,
        accent_soft: mix(accent, bg, if dark { 0.8 } else { 0.88 }),
        selection: alpha(accent, if dark { 51 } else { 36 }),
        focus: alpha(accent, if dark { 128 } else { 102 }),
        danger: red,
        success: green,
        highlights: [
            highlight(yellow),
            highlight(green),
            highlight(blue),
            highlight(mix(magenta, red, 0.3)),
            highlight(mix(magenta, blue, 0.4)),
            highlight(orange),
        ],
        swatches: SWATCHES,
        shadows: base.shadows.clone(),
    };
    Some((palette, dark))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKYO_NIGHT: &str = r##"
# Tokyo Night
mode = "dark"

accent = "#7aa2f7"
selection = "#292e42"
muted = "#414868"

background = "#1a1b26"
foreground = "#c0caf5"

red = "#f7768e"
yellow = "#e0af68"
green = "#9ece6a"
blue = "#7aa2f7"
magenta = "#bb9af7"

[notifications]
border = "rgba(33ccffee)"
"##;

    #[test]
    fn reads_the_semantic_keys() {
        let (p, dark) = palette_from_toml(TOKYO_NIGHT).unwrap();
        assert!(dark);
        assert_eq!(p.paper, Color32::from_rgb(0x1a, 0x1b, 0x26));
        assert_eq!(p.ink, Color32::from_rgb(0xc0, 0xca, 0xf5));
        assert_eq!(p.accent, Color32::from_rgb(0x7a, 0xa2, 0xf7));
        assert_eq!(p.rubric, Color32::from_rgb(0xf7, 0x76, 0x8e));
        // A light accent takes dark text.
        assert!(luminance(p.on_accent) < 0.1);
        assert_eq!(p.selection.a(), 51);
    }

    #[test]
    fn reads_the_legacy_ansi_keys() {
        let toml = "color0 = \"#ffffff\"\ncolor7 = \"#202020\"\ncolor4 = \"#2d4a9e\"\n";
        let (p, dark) = palette_from_toml(toml).unwrap();
        assert!(!dark, "a white background is a light theme without a mode key");
        assert_eq!(p.paper, Color32::WHITE);
        assert_eq!(p.accent, Color32::from_rgb(0x2d, 0x4a, 0x9e));
        assert_eq!(p.on_accent, Color32::WHITE);
        assert_eq!(p.surface, Color32::WHITE, "light surfaces are the paper itself");
    }

    #[test]
    fn needs_a_background_and_a_foreground() {
        assert!(palette_from_toml("accent = \"#ffffff\"\n").is_none());
        assert!(palette_from_toml("background = \"nope\"\nforeground = \"#000000\"").is_none());
    }

    #[test]
    fn a_reload_is_a_new_generation_until_applied() {
        let before = generation();
        assert!(install(TOKYO_NIGHT, "tokyo-night".into()));
        assert_eq!(generation(), before + 1, "the layout cache re-keys on this");
        assert!(changed_since_applied());
        mark_applied();
        assert!(!changed_since_applied());
        assert!(!install("accent = \"#ffffff\"", String::new()), "an unreadable palette keeps the old one");
        assert_eq!(generation(), before + 1);
    }

    #[test]
    fn a_trailing_comment_does_not_eat_the_colour() {
        let keys = parse("background = \"#1a1b26\" # paper\n");
        assert_eq!(keys["background"], "#1a1b26");
    }
}
