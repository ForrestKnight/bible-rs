//! Pages that take the reader's place in the main area (the design's Flows frames): the
//! translation library and the study collections. The sidebar's rows open them; the toolbar
//! then holds back/forward and a field that filters the page.

pub mod bookmarks;
pub mod highlights;
pub mod history;
pub mod notes;
pub mod plans;
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
    pub bookmarks: bookmarks::State,
    pub highlights: highlights::State,
    pub notes: notes::State,
    pub history: history::State,
    pub plans: plans::State,
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
        if self.page.has_panes() {
            self.notes.show(&mut child, rect, env, actions);
            self.scrolled = false;
            return;
        }
        let out = ScrollArea::vertical().id_salt(("page", self.page as u8)).auto_shrink(false).show(
            &mut child,
            |ui| {
                let width = ui.available_width();
                let left = ui.max_rect().left() + SIDE;
                let top = ui.max_rect().top() + env.toolbar + TOP;
                let content = Rect::from_min_size(pos2(left, top), vec2(width - 2.0 * SIDE, 0.0));
                let bottom = match self.page {
                    Page::Translations => self.translations.show(ui, content, env, actions),
                    Page::Bookmarks => self.bookmarks.show(ui, content, env, actions),
                    Page::Highlights => self.highlights.show(ui, content, env, actions),
                    Page::History => self.history.show(ui, content, env, actions),
                    Page::Plans => self.plans.show(ui, content, env, actions),
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

/// The reading translation, for the text of study items (bookmarks and the like are kept per
/// verse, not per translation); the default bundled one when it isn't local.
pub fn reading_text(env: &PageEnv<'_>) -> Option<std::sync::Arc<bible_core::Translation>> {
    env.library
        .load(&env.settings.translation)
        .ok()
        .or_else(|| env.library.load(bible_core::bundled::DEFAULT_ID).ok())
}

/// An upper-case group label: 11/16 semibold, +0.12 em, tertiary ink.
pub fn group_label(painter: &Painter, p: &Palette, x: f32, top: f32, s: &str) {
    let g = line(painter, &s.to_uppercase(), theme::ui_semibold(11.0), p.ink_3, 0.12);
    paint_on_baseline(painter, x, css_baseline(top, 16.0, 11.0, false), g, p.ink_3);
}

/// An empty state (`ds-16`): a 56 pt tinted circle with a 28 pt glyph, a Literata 20/28 title,
/// 13/20 body text, then up to two buttons and keycap hints, centred on `cx` from `top`.
pub struct Empty<'a> {
    pub glyph: crate::widgets::IconFn,
    pub title: &'a str,
    pub body: &'a str,
    pub primary: Option<&'a str>,
    pub ghost: Option<&'a str>,
    /// Keycaps and what they do.
    pub hints: &'a [(&'a [&'a str], &'a str)],
    pub width: f32,
}

impl Empty<'_> {
    pub fn height(&self, painter: &Painter) -> f32 {
        let body = self.body_galley(painter, Color32::PLACEHOLDER).size().y;
        let buttons = if self.primary.is_some() || self.ghost.is_some() { 12.0 + 4.0 + 32.0 } else { 0.0 };
        let hints = if self.hints.is_empty() { 0.0 } else { 24.0 + 20.0 };
        56.0 + 12.0 + 28.0 + 4.0 + body + buttons + hints
    }

    fn body_galley(&self, painter: &Painter, color: Color32) -> std::sync::Arc<egui::Galley> {
        let mut job =
            egui::text::LayoutJob::simple(self.body.into(), theme::ui_font(13.0), color, self.width);
        job.halign = egui::Align::Center;
        job.sections[0].format.line_height = Some(20.0);
        painter.layout_job(job)
    }

    /// Draws it; returns the primary and ghost buttons' responses.
    pub fn show(
        &self,
        ui: &mut Ui,
        p: &Palette,
        cx: f32,
        top: f32,
    ) -> (Option<egui::Response>, Option<egui::Response>) {
        let painter = ui.painter().clone();
        let c = pos2(cx, top + 28.0);
        painter.circle_filled(c, 28.0, p.accent.gamma_multiply(0.12));
        crate::icons::heavy(|| (self.glyph)(&painter, Rect::from_center_size(c, vec2(28.0, 28.0)), p.accent));
        let mut y = top + 56.0 + 12.0;
        let g = line(&painter, self.title, FontId::new(20.0, theme::family(theme::SERIF)), p.ink, 0.0);
        let gx = cx - g.size().x / 2.0;
        paint_on_baseline(&painter, gx, css_baseline(y, 28.0, 20.0, true), g, p.ink);
        y += 28.0 + 4.0;
        let body = self.body_galley(&painter, p.ink_2);
        let h = body.size().y;
        painter.galley(pos2(cx - self.width / 2.0, y), body, p.ink_2);
        y += h;
        let (mut primary, mut ghost) = (None, None);
        if self.primary.is_some() || self.ghost.is_some() {
            y += 12.0 + 4.0;
            let a = self.primary.map(|l| crate::widgets::Button::new(crate::widgets::Kind::Primary, l));
            let b = self.ghost.map(|l| crate::widgets::Button::new(crate::widgets::Kind::Ghost, l));
            let wa = a.map_or(0.0, |b| b.width(&painter));
            let wb = b.map_or(0.0, |b| b.width(&painter));
            let gap = if a.is_some() && b.is_some() { 8.0 } else { 0.0 };
            let mut x = cx - (wa + gap + wb) / 2.0;
            if let Some(a) = a {
                primary = Some(a.show_left(ui, p, x, y + 16.0, ui.id().with("empty-primary")).1);
                x += wa + gap;
            }
            if let Some(b) = b {
                ghost = Some(b.show_left(ui, p, x, y + 16.0, ui.id().with("empty-ghost")).1);
            }
            y += 32.0;
        }
        if !self.hints.is_empty() {
            y += 24.0;
            let cy = y + 10.0;
            let gap = 24.0;
            let widths: Vec<f32> = self
                .hints
                .iter()
                .map(|(keys, what)| {
                    let caps: f32 =
                        keys.iter().map(|k| crate::widgets::keycap_width(&painter, k) + 4.0).sum();
                    let t = painter
                        .layout_no_wrap((*what).into(), theme::ui_font(12.0), Color32::PLACEHOLDER)
                        .size()
                        .x;
                    caps + 4.0 + t
                })
                .collect();
            let total: f32 = widths.iter().sum::<f32>() + gap * (widths.len() as f32 - 1.0);
            let mut x = cx - total / 2.0;
            for ((keys, what), w) in self.hints.iter().zip(widths) {
                let start = x;
                for k in *keys {
                    x = crate::widgets::keycap(&painter, p, x, cy, k).right() + 4.0;
                }
                x += 4.0;
                painter.text(pos2(x, cy), egui::Align2::LEFT_CENTER, *what, theme::ui_font(12.0), p.ink_3);
                x = start + w + gap;
            }
        }
        (primary, ghost)
    }
}
