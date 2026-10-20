//! The design's colours (`crates/app/assets/design-tokens.json`, the Mac app's token file) for
//! the phone apps, so both draw from the same palette.

use serde_json::Value;

use crate::HighlightColor;

const TOKENS: &str = include_str!("../../app/assets/design-tokens.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ThemeName {
    Light,
    Sepia,
    Dark,
}

impl ThemeName {
    fn key(self) -> &'static str {
        match self {
            ThemeName::Light => "light",
            ThemeName::Sepia => "sepia",
            ThemeName::Dark => "dark",
        }
    }
}

/// A theme's colour roles as `0xRRGGBBAA`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Palette {
    pub paper: u32,
    pub surface: u32,
    pub glass: u32,
    pub fill: u32,
    pub fill_strong: u32,
    pub separator: u32,
    pub border: u32,
    pub ink: u32,
    pub ink_2: u32,
    pub ink_3: u32,
    pub verse: u32,
    pub rubric: u32,
    pub accent: u32,
    pub on_accent: u32,
    pub accent_soft: u32,
    pub selection: u32,
    pub danger: u32,
    /// Highlight fills under the text, in [`HighlightColor`] order.
    pub highlights: Vec<u32>,
    /// The highlight colours as swatches (the same in every theme).
    pub swatches: Vec<u32>,
}

const HIGHLIGHTS: [(HighlightColor, &str); 6] = [
    (HighlightColor::Yellow, "yellow"),
    (HighlightColor::Green, "green"),
    (HighlightColor::Blue, "blue"),
    (HighlightColor::Pink, "pink"),
    (HighlightColor::Purple, "purple"),
    (HighlightColor::Orange, "orange"),
];

/// A theme's palette from the design tokens.
#[uniffi::export]
pub fn palette(theme: ThemeName) -> Palette {
    let tokens: Value = serde_json::from_str(TOKENS).expect("design-tokens.json is valid JSON");
    let roles = &tokens["color"][theme.key()];
    let role = |name: &str| hex(&roles[name]);
    let highlight = &tokens["color"]["highlight"];
    Palette {
        paper: role("paper"),
        surface: role("surface"),
        glass: role("glass"),
        fill: role("fill"),
        fill_strong: role("fill_strong"),
        separator: role("separator"),
        border: role("border"),
        ink: role("ink"),
        ink_2: role("ink_2"),
        ink_3: role("ink_3"),
        verse: role("verse"),
        rubric: role("rubric"),
        accent: role("accent"),
        on_accent: role("on_accent"),
        accent_soft: role("accent_soft"),
        selection: role("selection"),
        danger: role("danger"),
        highlights: HIGHLIGHTS.iter().map(|(_, name)| hex(&highlight[name][theme.key()])).collect(),
        swatches: HIGHLIGHTS.iter().map(|(_, name)| hex(&highlight[name]["swatch"])).collect(),
    }
}

/// `#RRGGBB` or `#RRGGBBAA` → `0xRRGGBBAA`. A missing token is a build-breaking mistake, which
/// the tests catch.
fn hex(v: &Value) -> u32 {
    let s = v.as_str().and_then(|s| s.strip_prefix('#')).unwrap_or_default();
    let n = u32::from_str_radix(s, 16).unwrap_or(0);
    match s.len() {
        6 => n << 8 | 0xFF,
        8 => n,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_has_every_colour() {
        for theme in [ThemeName::Light, ThemeName::Sepia, ThemeName::Dark] {
            let p = palette(theme);
            let all = [
                p.paper,
                p.surface,
                p.glass,
                p.fill,
                p.fill_strong,
                p.separator,
                p.border,
                p.ink,
                p.ink_2,
                p.ink_3,
                p.verse,
                p.rubric,
                p.accent,
                p.on_accent,
                p.accent_soft,
                p.selection,
                p.danger,
            ];
            assert!(all.iter().chain(&p.highlights).chain(&p.swatches).all(|&c| c != 0), "{theme:?}");
            assert_eq!((p.highlights.len(), p.swatches.len()), (6, 6));
        }
        assert_eq!(palette(ThemeName::Light).paper, 0xFFFFFFFF);
        assert_eq!(palette(ThemeName::Dark).ink, 0xE6E3DCFF);
        assert_eq!(palette(ThemeName::Sepia).selection, 0x3C549024);
        assert_eq!(palette(ThemeName::Light).highlights[0], 0xFBE7A1FF);
    }
}
