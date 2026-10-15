//! Reading themes (light, sepia, dark), fonts and egui styling.
//!
//! Every colour and metric comes from the design's token file, `assets/design-tokens.json`, which
//! `build.rs` turns into Rust constants at compile time: one [`Palette`] per theme, the
//! highlight swatches, the type scale ([`text`]) and the metric groups ([`space`], [`radius`],
//! [`size`], [`titlebar`], [`icon`], [`motion`]).

use std::sync::Arc;

use bible_core::study::HighlightColor;
use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Painter, Rect,
    Shadow, Stroke, StrokeKind, TextStyle, Visuals,
};
use eframe::epaint::RectShape;
use eframe::epaint::text::{FontTweak, VariationCoords};
use serde::{Deserialize, Serialize};

include!(concat!(env!("OUT_DIR"), "/tokens.rs"));

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

/// One theme's colours. Field names are the design's colour roles.
pub struct Palette {
    /// Window and reading background.
    pub paper: Color32,
    /// The floating sidebar panel.
    pub sidebar: Color32,
    /// Popovers, menus, cards, sheets and the palette.
    pub surface: Color32,
    /// Toolbar material once content scrolls beneath it.
    pub toolbar: Color32,
    /// Toolbar control capsules.
    pub glass: Color32,
    /// Text fields and selects.
    pub field: Color32,
    /// A raised control: selected segment, toggle and slider knobs.
    pub raised: Color32,
    /// Tracks, wells and hover fills.
    pub fill: Color32,
    /// Pressed fills, the off track of a toggle.
    pub fill_strong: Color32,
    /// Opaque hairlines and dividers.
    pub separator: Color32,
    /// Translucent control outlines.
    pub border: Color32,
    /// Modal backdrop.
    pub scrim: Color32,
    /// Text: primary, secondary, tertiary (placeholders, disabled glyphs).
    pub ink: Color32,
    pub ink_2: Color32,
    pub ink_3: Color32,
    /// Verse numbers.
    pub verse: Color32,
    /// Red letters (words of Christ).
    pub rubric: Color32,
    /// Selection, links and primary buttons; `on_accent` is text on it.
    pub accent: Color32,
    pub on_accent: Color32,
    /// Selected rows and chips.
    pub accent_soft: Color32,
    /// Selected-verse wash, translucent so highlights show through.
    pub selection: Color32,
    /// Keyboard focus ring.
    pub focus: Color32,
    pub danger: Color32,
    pub success: Color32,
    /// Highlight fills behind text, in [`HighlightColor::ALL`] order.
    pub highlights: [Color32; 6],
    /// Highlight identity colours (dots, filters, colour bars); the same in every theme.
    pub swatches: [Color32; 6],
    pub shadows: Shadows,
}

impl Palette {
    pub fn highlight(&self, c: HighlightColor) -> Color32 {
        self.highlights[HighlightColor::ALL.iter().position(|x| *x == c).unwrap_or(0)]
    }

    pub fn swatch(&self, c: HighlightColor) -> Color32 {
        self.swatches[HighlightColor::ALL.iter().position(|x| *x == c).unwrap_or(0)]
    }
}

pub struct Shadows {
    /// Toolbar capsules, secondary buttons.
    pub control: Elevation,
    /// The floating sidebar and inspector.
    pub panel: Elevation,
    /// Menus, popovers, hover cards.
    pub popover: Elevation,
    /// The ⌘K palette, sheets.
    pub modal: Elevation,
    /// Toggle and slider knobs, the selected segment.
    pub knob: Elevation,
}

/// A design shadow: a hairline ring around the shape plus one soft drop shadow.
#[derive(Clone, Copy, Debug)]
pub struct Elevation {
    pub ring: Color32,
    pub ring_width: f32,
    /// Drop shadow offset (down) and CSS blur radius.
    pub y: f32,
    pub blur: f32,
    pub color: Color32,
}

impl Elevation {
    /// The drop shadow as an egui shape. egui fades a shadow linearly across `blur_width`
    /// centred on its edge, where CSS fades a Gaussian over about the blur radius either side,
    /// so the width is twice the radius.
    pub fn drop_shape(&self, rect: Rect, radius: f32) -> RectShape {
        RectShape::filled(rect.translate(egui::vec2(0.0, self.y)), radius, self.color)
            .with_blur_width(self.blur * 2.0)
    }

    /// The drop shadow alone, for egui frames and popups.
    pub fn shadow(&self) -> Shadow {
        Shadow {
            offset: [0, self.y.round() as i8],
            blur: (self.blur * 2.0).round() as u8,
            spread: 0,
            color: self.color,
        }
    }

    /// Paints the drop shadow and the ring for a shape that is then filled on top.
    pub fn paint(&self, painter: &Painter, rect: Rect, radius: f32) {
        painter.add(self.drop_shape(rect, radius));
        painter.rect_stroke(rect, radius, Stroke::new(self.ring_width, self.ring), StrokeKind::Outside);
    }

    /// Paints the ring only (after the fill, so it stays crisp over it).
    pub fn paint_ring(&self, painter: &Painter, rect: Rect, radius: f32) {
        painter.rect_stroke(rect, radius, Stroke::new(self.ring_width, self.ring), StrokeKind::Outside);
    }
}

/// A style from the design's type scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypeStyle {
    /// Literata (reading) rather than Inter (interface).
    pub reading: bool,
    pub size: f32,
    pub line_height: f32,
    pub weight: u16,
    /// Letter spacing in ems.
    pub tracking: f32,
}

impl TypeStyle {
    pub fn font(&self) -> FontId {
        let name = match (self.reading, self.weight) {
            (true, w) if w < 350 => SERIF_DISPLAY,
            (true, w) if w >= 600 => SERIF_SEMIBOLD,
            (true, _) => SERIF,
            (false, w) if w >= 600 => UI_SEMIBOLD,
            (false, w) if w >= 500 => UI_MEDIUM,
            (false, _) => UI,
        };
        FontId::new(self.size, family(name))
    }

    /// Letter spacing in points.
    pub fn spacing(&self) -> f32 {
        self.tracking * self.size
    }
}

/// Interface metrics in points, from the design tokens.
pub mod metrics {
    use super::{icon, titlebar};

    /// Height of the macOS unified toolbar. AppKit lays a unified toolbar out 52 pt tall and
    /// centres the traffic lights on it; toolbar items centred in the same height share their line.
    pub const TOOLBAR_HEIGHT: f32 = titlebar::HEIGHT;
    /// Toolbar height where the window has its own title bar (Linux, Windows).
    pub const COMPACT_TOOLBAR_HEIGHT: f32 = 44.0;
    /// Toolbar controls: glass capsules of this height, with buttons of these widths.
    pub const CAPSULE_HEIGHT: f32 = titlebar::CAPSULE_HEIGHT;
    pub const BUTTON_WIDTH: f32 = titlebar::BUTTON_WIDTH;
    pub const SIDEBAR_TOGGLE_WIDTH: f32 = titlebar::SIDEBAR_TOGGLE_WIDTH;
    pub const SEARCH_FIELD_WIDTH: f32 = titlebar::SEARCH_FIELD_WIDTH;
    /// Toolbar icons and row/menu icons, and the stroke that gives both the same weight.
    pub const ICON: f32 = icon::TOOLBAR;
    pub const ICON_SM: f32 = icon::ROW;
    pub const ICON_STROKE: f32 = icon::STROKE_20;
    pub const ICON_STROKE_SM: f32 = icon::STROKE_16;
    /// From the traffic lights (or the sidebar) to the first toolbar control, and between groups.
    pub const GROUP_GAP: f32 = titlebar::GROUP_GAP;
    /// Toolbar margins at the window's edges without traffic lights.
    pub const LEADING_INSET: f32 = titlebar::LEADING_INSET_SIDEBAR_HIDDEN;
    pub const TRAILING_INSET: f32 = titlebar::TRAILING_INSET;
    /// The floating sidebar: its width and its inset from the window's edges.
    pub const SIDEBAR_WIDTH: f32 = super::size::SIDEBAR;
    pub const SIDEBAR_INSET: f32 = titlebar::SIDEBAR_INSET;
}

// Font families.
pub const SERIF: &str = "serif";
pub const SERIF_ITALIC: &str = "serif-italic";
pub const SERIF_SEMIBOLD: &str = "serif-semibold";
pub const SERIF_DISPLAY: &str = "serif-display";
pub const SERIF_HEADING: &str = "serif-heading";
pub const UI_SEMIBOLD: &str = "ui-semibold";
pub const UI_MEDIUM: &str = "ui-medium";
pub const UI: &str = "ui";

pub fn family(name: &str) -> FontFamily {
    FontFamily::Name(name.into())
}

pub fn ui_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn ui_medium(size: f32) -> FontId {
    FontId::new(size, family(UI_MEDIUM))
}

pub fn ui_semibold(size: f32) -> FontId {
    FontId::new(size, family(UI_SEMIBOLD))
}

static LITERATA: &[u8] = include_bytes!("../assets/fonts/Literata.ttf");
static LITERATA_ITALIC: &[u8] = include_bytes!("../assets/fonts/Literata-Italic.ttf");
static INTER: &[u8] = include_bytes!("../assets/fonts/Inter.ttf");
static INTER_MEDIUM: &[u8] = include_bytes!("../assets/fonts/Inter-Medium.ttf");
static INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");

fn variable(bytes: &'static [u8], coords: &[(&[u8; 4], f32)]) -> Arc<FontData> {
    let tweak = FontTweak {
        coords: VariationCoords::new(coords.iter().map(|(t, v)| (**t, *v))),
        ..Default::default()
    };
    Arc::new(FontData::from_static(bytes).tweak(tweak))
}

/// Literata (reading) and Inter (interface), both SIL OFL; egui's bundled fonts remain as
/// fallbacks for symbols.
pub fn font_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let fallbacks: Vec<String> = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    // Body text, italics and the interface use static fonts or each variable font's *default*
    // instance: pinning axis values makes egui build a variation instance for every shaped run,
    // which costs more than the rest of shaping. Only headings (few runs) pin weight and optical
    // size; Inter's medium and semibold weights are static instances for that reason.
    let data = [
        ("literata", Arc::new(FontData::from_static(LITERATA))),
        ("literata-italic", Arc::new(FontData::from_static(LITERATA_ITALIC))),
        ("literata-semibold", variable(LITERATA, &[(b"wght", 600.0), (b"opsz", 16.0)])),
        ("literata-display", variable(LITERATA, &[(b"wght", 300.0), (b"opsz", 48.0)])),
        ("literata-heading", variable(LITERATA, &[(b"wght", 400.0), (b"opsz", 28.0)])),
        ("inter", Arc::new(FontData::from_static(INTER))),
        ("inter-medium", Arc::new(FontData::from_static(INTER_MEDIUM))),
        ("inter-semibold", Arc::new(FontData::from_static(INTER_SEMIBOLD))),
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
    fonts.families.insert(family(UI), with_fallbacks("inter"));
    fonts.families.insert(family(UI_MEDIUM), with_fallbacks("inter-medium"));
    fonts.families.insert(family(UI_SEMIBOLD), with_fallbacks("inter-semibold"));
    fonts.families.insert(family(SERIF), with_fallbacks("literata"));
    fonts.families.insert(family(SERIF_ITALIC), with_fallbacks("literata-italic"));
    fonts.families.insert(family(SERIF_SEMIBOLD), with_fallbacks("literata-semibold"));
    fonts.families.insert(family(SERIF_DISPLAY), with_fallbacks("literata-display"));
    fonts.families.insert(family(SERIF_HEADING), with_fallbacks("literata-heading"));
    fonts
}

/// Applies a theme to egui's widgets.
pub fn apply(ctx: &egui::Context, theme: Theme) {
    let p = theme.palette();
    let dark = theme.is_dark();
    ctx.set_theme(if dark { egui::Theme::Dark } else { egui::Theme::Light });
    ctx.style_mut_of(if dark { egui::Theme::Dark } else { egui::Theme::Light }, |style| {
        style.text_styles = [
            (TextStyle::Small, ui_font(text::SMALL.size)),
            (TextStyle::Body, ui_font(text::UI.size)),
            (TextStyle::Button, ui_font(text::UI.size)),
            (TextStyle::Heading, text::UI_TITLE.font()),
            (TextStyle::Monospace, FontId::monospace(12.5)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(space::S2, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        style.spacing.interact_size = egui::vec2(24.0, 28.0);
        style.spacing.window_margin = Margin::same(space::S4 as i8);
        style.spacing.menu_margin = Margin::same(6);
        style.spacing.scroll.bar_width = 7.0;
        style.spacing.scroll.floating = true;
        style.interaction.tooltip_delay = 0.35;
        style.animation_time = motion::FAST_MS / 1000.0;
        style.url_in_tooltip = false;

        let mut v = if dark { Visuals::dark() } else { Visuals::light() };
        v.panel_fill = p.paper;
        v.window_fill = p.surface;
        v.window_stroke = Stroke::new(p.shadows.modal.ring_width, p.shadows.modal.ring);
        v.window_corner_radius = CornerRadius::same(radius::LG as u8);
        v.menu_corner_radius = CornerRadius::same(radius::MD as u8);
        v.window_shadow = p.shadows.modal.shadow();
        v.popup_shadow = p.shadows.popover.shadow();
        v.extreme_bg_color = p.field;
        v.text_edit_bg_color = Some(p.field);
        v.faint_bg_color = p.fill;
        v.code_bg_color = p.fill;
        v.hyperlink_color = p.accent;
        v.selection.bg_fill = p.selection;
        v.selection.stroke = Stroke::new(1.0, p.ink);
        v.text_cursor.stroke = Stroke::new(1.5, p.accent);
        v.override_text_color = Some(p.ink);
        v.weak_text_color = Some(p.ink_2);
        v.indent_has_left_vline = false;
        let corner = CornerRadius::same(radius::SM as u8);
        let w = &mut v.widgets;
        let quiet = Stroke::new(1.0, p.border);
        // `bg_fill` is used for slider rails and checkbox interiors, so it must differ from the
        // window colour; buttons use the (transparent) `weak_bg_fill` instead.
        for (state, fill) in [
            (&mut w.noninteractive, p.paper),
            (&mut w.inactive, p.fill_strong),
            (&mut w.hovered, p.fill),
            (&mut w.active, p.fill_strong),
            (&mut w.open, p.fill),
        ] {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.bg_stroke = quiet;
            state.fg_stroke = Stroke::new(1.0, p.ink);
            state.corner_radius = corner;
            state.expansion = 0.0;
        }
        w.noninteractive.bg_stroke = Stroke::new(1.0, p.separator);
        w.inactive.weak_bg_fill = Color32::TRANSPARENT;
        // Checkbox/radio frames use `bg_stroke`; keep it visible but quiet.
        w.inactive.bg_stroke = Stroke::new(1.0, p.border);
        w.hovered.bg_stroke = Stroke::new(1.0, p.ink_3);
        w.active.bg_stroke = Stroke::new(1.0, p.accent);
        style.visuals = v;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_match_the_design() {
        // Spot checks against assets/design-tokens.json.
        assert_eq!(LIGHT.accent, Color32::from_rgb(0x2D, 0x4A, 0x9E));
        assert_eq!(DARK.paper, Color32::from_rgb(0x16, 0x16, 0x18));
        assert_eq!(SEPIA.ink, Color32::from_rgb(0x3A, 0x2E, 0x22));
        assert_eq!(LIGHT.glass, Color32::from_rgba_unmultiplied(0xFF, 0xFF, 0xFF, 0xE0));
        assert_eq!(LIGHT.highlight(HighlightColor::Orange), Color32::from_rgb(0xFA, 0xDD, 0xC2));
        assert_eq!(SWATCHES[0], Color32::from_rgb(0xEB, 0xC2, 0x4A));
        assert_eq!(traffic_lights::CLOSE, Color32::from_rgb(0xFF, 0x5F, 0x57));
        assert_eq!(text::BODY.size, 20.0);
        assert_eq!(text::BODY.line_height, 32.0);
        assert_eq!(text::LABEL.tracking, 0.12);
        assert_eq!(space::S4, 16.0);
        assert_eq!(radius::LG, 16.0);
        assert_eq!(titlebar::HEIGHT, 52.0);
        assert_eq!(icon::STROKE_16, 1.75);
        assert_eq!(LIGHT.shadows.panel.blur, 16.0);
        assert_eq!(DARK.shadows.modal.ring, Color32::from_rgba_unmultiplied(255, 255, 255, 36));
    }
}
