//! Pages that take the reader's place in the main area (the design's Flows frames): the
//! translation library and the study collections. The sidebar's rows open them; the toolbar
//! then holds back/forward and a field that filters the page.

pub mod translations;

use eframe::egui::{self, Color32, FontId, Painter, Rect, ScrollArea, Ui, pos2, vec2};

use bible_core::library::Library;
use bible_core::study::StudyStore;

use crate::actions::Action;
use crate::app::Settings;
use crate::theme::{self, Palette};
use crate::widgets::{css_baseline, line, paint_on_baseline};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Page {
    #[default]
    Reader,
    Translations,
    Bookmarks,
    Highlights,
    Notes,
    History,
    Plans,
}

impl Page {
    /// The toolbar field's placeholder on this page.
    pub fn placeholder(self) -> &'static str {
        match self {
            Page::Reader => "Search",
            Page::Translations => "Filter translations",
            Page::Bookmarks => "Search bookmarks",
            Page::Highlights => "Search highlights",
            Page::Notes => "Search notes",
            Page::History => "Search history",
            Page::Plans => "Search plans",
        }
    }

    pub fn from_name(name: &str) -> Option<Page> {
        Some(match name {
            "reader" => Page::Reader,
            "translations" => Page::Translations,
            "bookmarks" => Page::Bookmarks,
            "highlights" => Page::Highlights,
            "notes" => Page::Notes,
            "history" => Page::History,
            "plans" => Page::Plans,
            _ => return None,
        })
    }
}

/// What a page reads from the app each frame.
pub struct PageEnv<'a> {
    pub library: &'a Library,
    pub study: &'a StudyStore,
    pub settings: &'a Settings,
    /// The toolbar field's text.
    pub query: &'a str,
    /// The toolbar's height (content starts 24 pt below it).
    pub toolbar: f32,
}

/// The open page and each page's own state.
#[derive(Default)]
pub struct Pages {
    pub page: Page,
    pub query: String,
    pub translations: translations::State,
    /// Whether the page is scrolled (the toolbar then takes its material).
    pub scrolled: bool,
}

impl Pages {
    pub fn open(&mut self, page: Page) {
        if self.page != page {
            self.query.clear();
        }
        self.page = page;
    }

    pub fn reader(&self) -> bool {
        self.page == Page::Reader
    }

    /// Draws the open page over `rect` (the main area, from the window's top: content scrolls
    /// under the toolbar).
    pub fn show(&mut self, ui: &mut Ui, rect: Rect, env: &PageEnv<'_>, actions: &mut Vec<Action>) {
        let p = env.settings.theme.palette();
        ui.painter().rect_filled(rect, 0.0, p.paper);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        child.set_clip_rect(rect);
        let out = ScrollArea::vertical().id_salt(("page", self.page as u8)).auto_shrink(false).show(
            &mut child,
            |ui| {
                let width = ui.available_width();
                let left = ui.max_rect().left() + SIDE;
                let top = ui.max_rect().top() + env.toolbar + TOP;
                let content = Rect::from_min_size(pos2(left, top), vec2(width - 2.0 * SIDE, 0.0));
                let bottom = match self.page {
                    Page::Translations => self.translations.show(ui, content, env, actions),
                    _ => content.top(),
                };
                ui.allocate_rect(
                    Rect::from_min_max(ui.max_rect().min, pos2(ui.max_rect().right(), bottom + 40.0)),
                    egui::Sense::hover(),
                );
            },
        );
        self.scrolled = out.state.offset.y > 0.5;
    }
}

/// Page margins: 40 pt at the sides, 24 pt under the toolbar.
pub const SIDE: f32 = 40.0;
pub const TOP: f32 = 24.0;

/// A page's title (Literata 28/36) and the subtitle under it (13/20, secondary ink).
pub fn title(painter: &Painter, p: &Palette, left: f32, top: f32, title: &str, subtitle: &str) {
    let font = FontId::new(28.0, theme::family(theme::SERIF_HEADING));
    let g = line(painter, title, font, p.ink, -0.01);
    paint_on_baseline(painter, left, css_baseline(top, 36.0, 28.0, true), g, p.ink);
    let g = line(painter, subtitle, theme::ui_font(13.0), p.ink_2, 0.0);
    paint_on_baseline(painter, left, css_baseline(top + 38.0, 20.0, 13.0, false), g, p.ink_2);
}

/// Text on a line box: `size` pt Inter (`font`) with its CSS baseline in a `lh` box at `top`,
/// left-aligned at `x` (or right-aligned there when `right`). Returns the text's width.
#[allow(clippy::too_many_arguments)]
pub fn text(
    painter: &Painter,
    x: f32,
    top: f32,
    lh: f32,
    s: &str,
    font: FontId,
    color: Color32,
    right: bool,
) -> f32 {
    let size = font.size;
    let reading = matches!(&font.family, egui::FontFamily::Name(n) if n.starts_with("serif"));
    let g = line(painter, s, font, color, 0.0);
    let w = g.size().x;
    let x = if right { x - w } else { x };
    paint_on_baseline(painter, x, css_baseline(top, lh, size, reading), g, color);
    w
}

/// `s` cut to fit `width` with an ellipsis.
pub fn elide(painter: &Painter, s: &str, font: &FontId, width: f32) -> String {
    let fits =
        |t: &str| painter.layout_no_wrap(t.into(), font.clone(), Color32::PLACEHOLDER).size().x <= width;
    if fits(s) {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let (mut lo, mut hi) = (0, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let t: String = chars[..mid].iter().collect::<String>() + "…";
        if fits(&t) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    chars[..lo].iter().collect::<String>().trim_end().to_string() + "…"
}
