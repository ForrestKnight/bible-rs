//! Reading themes (light, sepia, dark), fonts and egui styling.

use std::sync::Arc;

use bible_core::study::HighlightColor;
use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Shadow, Stroke, TextStyle,
    Visuals,
};
use eframe::epaint::text::{FontTweak, VariationCoords};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Theme {
    #[default]
    Light,
    Sepia,
    Dark,
}

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::Light, Theme::Sepia, Theme::Dark];

    pub fn label(self) -> &'static str {
        match self {
            Theme::Light => "Light",
            Theme::Sepia => "Sepia",
            Theme::Dark => "Dark",
        }
    }

    pub fn next(self) -> Theme {
        match self {
            Theme::Light => Theme::Sepia,
            Theme::Sepia => Theme::Dark,
            Theme::Dark => Theme::Light,
        }
    }

    pub fn is_dark(self) -> bool {
        self == Theme::Dark
    }

    pub fn palette(self) -> &'static Palette {
        match self {
            Theme::Light => &LIGHT,
            Theme::Sepia => &SEPIA,
            Theme::Dark => &DARK,
        }
    }
}

/// Semantic colours for everything we paint ourselves.
pub struct Palette {
    pub page: Color32,
    pub chrome: Color32,
    pub sidebar: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub faint: Color32,
    pub verse_num: Color32,
    pub heading: Color32,
    pub accent: Color32,
    pub red_letter: Color32,
    pub selection: Color32,
    pub hover: Color32,
    pub field: Color32,
    pub search_hit: Color32,
    pub flash: Color32,
    pub highlights: [Color32; 5],
    /// Opaque swatches used for the highlight colour buttons.
    pub swatches: [Color32; 5],
}

const fn hex(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

const fn rgba(rgb: u32, a: u8) -> Color32 {
    Color32::from_rgba_premultiplied(
        (((rgb >> 16) & 0xFF) * a as u32 / 255) as u8,
        (((rgb >> 8) & 0xFF) * a as u32 / 255) as u8,
        ((rgb & 0xFF) * a as u32 / 255) as u8,
        a,
    )
}

const SWATCHES: [Color32; 5] = [hex(0xF2C94C), hex(0x6FCF97), hex(0x56CCF2), hex(0xF295B6), hex(0xB18CE8)];

pub static LIGHT: Palette = Palette {
    page: hex(0xFCFAF6),
    chrome: hex(0xF4F1EA),
    sidebar: hex(0xF6F3EC),
    border: hex(0xE2DCD0),
    text: hex(0x24221E),
    muted: hex(0x75705F),
    faint: hex(0xA9A392),
    verse_num: hex(0xA4793A),
    heading: hex(0x3C362B),
    accent: hex(0x9A6B2F),
    red_letter: hex(0xB0281F),
    selection: rgba(0x6E8BC4, 58),
    hover: hex(0xEBE6DB),
    field: hex(0xFFFFFF),
    search_hit: rgba(0xF2C94C, 150),
    flash: rgba(0xC9A25E, 90),
    highlights: [rgba(0xF5D451, 110), rgba(0x7ED39F, 95), rgba(0x6CC9F0, 90), rgba(0xF49CBB, 95), rgba(0xBC9CEB, 95)],
    swatches: SWATCHES,
};

pub static SEPIA: Palette = Palette {
    page: hex(0xF5EDDC),
    chrome: hex(0xEBE1CB),
    sidebar: hex(0xEFE6D2),
    border: hex(0xDACDB0),
    text: hex(0x46382A),
    muted: hex(0x86704F),
    faint: hex(0xB09C7C),
    verse_num: hex(0xA67A40),
    heading: hex(0x4E3D2B),
    accent: hex(0x96672E),
    red_letter: hex(0xA02D1B),
    selection: rgba(0x7D8FB0, 62),
    hover: hex(0xE5D9BF),
    field: hex(0xFBF6EA),
    search_hit: rgba(0xE8B94A, 150),
    flash: rgba(0xC49A57, 100),
    highlights: [rgba(0xF1CB45, 115), rgba(0x86C795, 100), rgba(0x74BEDF, 95), rgba(0xEB98B0, 100), rgba(0xB49BDE, 100)],
    swatches: SWATCHES,
};

pub static DARK: Palette = Palette {
    page: hex(0x18181B),
    chrome: hex(0x202024),
    sidebar: hex(0x1D1D21),
    border: hex(0x323238),
    text: hex(0xDCD8CF),
    muted: hex(0x908B80),
    faint: hex(0x5F5B54),
    verse_num: hex(0xC4A06A),
    heading: hex(0xE6E1D6),
    accent: hex(0xD2AE6D),
    red_letter: hex(0xEC7F72),
    selection: rgba(0x7F9BD8, 60),
    hover: hex(0x2A2A30),
    field: hex(0x26262B),
    search_hit: rgba(0xD9A93E, 110),
    flash: rgba(0xD2AE6D, 70),
    highlights: [rgba(0xD8B13A, 80), rgba(0x4FA873, 80), rgba(0x3E9CC6, 80), rgba(0xC86A8E, 80), rgba(0x8E6CC6, 85)],
    swatches: SWATCHES,
};

impl Palette {
    pub fn highlight(&self, c: HighlightColor) -> Color32 {
        self.highlights[HighlightColor::ALL.iter().position(|x| *x == c).unwrap_or(0)]
    }

    pub fn swatch(&self, c: HighlightColor) -> Color32 {
        self.swatches[HighlightColor::ALL.iter().position(|x| *x == c).unwrap_or(0)]
    }
}

// Font families.
pub const SERIF: &str = "serif";
pub const SERIF_ITALIC: &str = "serif-italic";
pub const SERIF_SEMIBOLD: &str = "serif-semibold";
pub const SERIF_DISPLAY: &str = "serif-display";
pub const UI_MEDIUM: &str = "ui-medium";

pub fn family(name: &str) -> FontFamily {
    FontFamily::Name(name.into())
}

pub fn ui_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn ui_medium(size: f32) -> FontId {
    FontId::new(size, family(UI_MEDIUM))
}

static LITERATA: &[u8] = include_bytes!("../assets/fonts/Literata.ttf");
static LITERATA_ITALIC: &[u8] = include_bytes!("../assets/fonts/Literata-Italic.ttf");
static INTER: &[u8] = include_bytes!("../assets/fonts/Inter.ttf");

fn variable(bytes: &'static [u8], coords: &[(&[u8; 4], f32)]) -> Arc<FontData> {
    let tweak = FontTweak { coords: VariationCoords::new(coords.iter().map(|(t, v)| (**t, *v))), ..Default::default() };
    Arc::new(FontData::from_static(bytes).tweak(tweak))
}

/// Literata (reading) and Inter (interface), both SIL OFL, registered at several weights by
/// pinning variable-font axes; egui's bundled fonts remain as fallbacks for symbols.
pub fn font_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let fallbacks: Vec<String> = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let data = [
        ("literata", variable(LITERATA, &[(b"wght", 400.0), (b"opsz", 14.0)])),
        ("literata-italic", variable(LITERATA_ITALIC, &[(b"wght", 400.0), (b"opsz", 14.0)])),
        ("literata-semibold", variable(LITERATA, &[(b"wght", 600.0), (b"opsz", 16.0)])),
        ("literata-display", variable(LITERATA, &[(b"wght", 300.0), (b"opsz", 60.0)])),
        ("inter", variable(INTER, &[(b"wght", 420.0), (b"opsz", 14.0)])),
        ("inter-medium", variable(INTER, &[(b"wght", 560.0), (b"opsz", 14.0)])),
    ];
    for (name, d) in data {
        fonts.font_data.insert(name.into(), d);
    }
    let with_fallbacks = |first: &str| {
        let mut v = vec![first.to_string()];
        v.extend(fallbacks.iter().cloned());
        v
    };
    fonts.families.insert(FontFamily::Proportional, with_fallbacks("inter"));
    fonts.families.insert(family(UI_MEDIUM), with_fallbacks("inter-medium"));
    fonts.families.insert(family(SERIF), with_fallbacks("literata"));
    fonts.families.insert(family(SERIF_ITALIC), with_fallbacks("literata-italic"));
    fonts.families.insert(family(SERIF_SEMIBOLD), with_fallbacks("literata-semibold"));
    fonts.families.insert(family(SERIF_DISPLAY), with_fallbacks("literata-display"));
    fonts
}

/// Applies a theme to egui's widgets.
pub fn apply(ctx: &egui::Context, theme: Theme) {
    let p = theme.palette();
    let dark = theme.is_dark();
    ctx.set_theme(if dark { egui::Theme::Dark } else { egui::Theme::Light });
    ctx.style_mut_of(if dark { egui::Theme::Dark } else { egui::Theme::Light }, |style| {
        style.text_styles = [
            (TextStyle::Small, ui_font(11.0)),
            (TextStyle::Body, ui_font(13.5)),
            (TextStyle::Button, ui_font(13.5)),
            (TextStyle::Heading, ui_medium(17.0)),
            (TextStyle::Monospace, FontId::monospace(12.5)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(9.0, 4.0);
        style.spacing.interact_size = egui::vec2(24.0, 26.0);
        style.spacing.window_margin = Margin::same(16);
        style.spacing.menu_margin = Margin::same(6);
        style.spacing.scroll.bar_width = 7.0;
        style.spacing.scroll.floating = true;
        style.interaction.tooltip_delay = 0.35;
        style.animation_time = 0.12;
        style.url_in_tooltip = false;

        let mut v = if dark { Visuals::dark() } else { Visuals::light() };
        v.panel_fill = p.page;
        v.window_fill = p.chrome;
        v.window_stroke = Stroke::new(1.0, p.border);
        v.window_corner_radius = CornerRadius::same(12);
        v.menu_corner_radius = CornerRadius::same(8);
        v.window_shadow =
            Shadow { offset: [0, 10], blur: 32, spread: 0, color: Color32::from_black_alpha(if dark { 120 } else { 40 }) };
        v.popup_shadow =
            Shadow { offset: [0, 6], blur: 18, spread: 0, color: Color32::from_black_alpha(if dark { 100 } else { 30 }) };
        v.extreme_bg_color = p.field;
        v.text_edit_bg_color = Some(p.field);
        v.faint_bg_color = p.hover;
        v.code_bg_color = p.hover;
        v.hyperlink_color = p.accent;
        v.selection.bg_fill = p.selection;
        v.selection.stroke = Stroke::new(1.0, p.text);
        v.text_cursor.stroke = Stroke::new(1.5, p.accent);
        v.override_text_color = Some(p.text);
        v.weak_text_color = Some(p.muted);
        v.indent_has_left_vline = false;
        let radius = CornerRadius::same(6);
        let w = &mut v.widgets;
        let quiet = Stroke::new(1.0, p.border);
        for (state, fill) in [
            (&mut w.noninteractive, p.page),
            (&mut w.inactive, p.chrome),
            (&mut w.hovered, p.hover),
            (&mut w.active, p.hover),
            (&mut w.open, p.hover),
        ] {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.bg_stroke = quiet;
            state.fg_stroke = Stroke::new(1.0, p.text);
            state.corner_radius = radius;
            state.expansion = 0.0;
        }
        w.inactive.weak_bg_fill = Color32::TRANSPARENT;
        // Checkbox/radio frames use `bg_stroke`; keep it visible but quiet.
        w.inactive.bg_stroke = Stroke::new(1.0, p.faint);
        w.active.bg_stroke = Stroke::new(1.0, p.accent);
        style.visuals = v;
    });
}
