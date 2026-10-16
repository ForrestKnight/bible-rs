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
use crate::widgets::{css_baseline, galley_baseline, line, paint_on_baseline};

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
        if self.page.has_panes() && !self.notes.is_empty(env) {
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
                    Page::Notes => self.notes.show_empty(ui, content, env, actions),
                    Page::Reader => content.top(),
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

/// How an empty state's circle and glyph are tinted.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    /// The study pages: a 12 % accent circle and an accent glyph, drawn heavier.
    Accent,
    /// Errors: a 12 % danger circle and a danger glyph.
    Danger,
    /// A search that found nothing: a `fill` circle and a tertiary glyph.
    Quiet,
}

/// An empty state (`ds-16`): a 56 pt tinted circle with a 28 pt glyph, a Literata 20/28 title,
/// 13/20 body text, then up to two buttons and keycap hints. Everything is centred on one axis:
/// each line of the body, the buttons by what shows of them, and the hints, which wrap into
/// rows when the space is narrower than they are. A ghost button shows only its label, so its
/// padding doesn't count.
pub struct Empty<'a> {
    pub glyph: crate::widgets::IconFn,
    pub tone: Tone,
    pub title: &'a str,
    pub body: &'a str,
    /// Up to two buttons, side by side; `show` returns their responses in this order.
    pub buttons: &'a [crate::widgets::Button<'a>],
    /// Keycaps and what they do.
    pub hints: &'a [(&'a [&'a str], &'a str)],
    /// The body's measure, narrowed to the space there is.
    pub width: f32,
}

/// An [`Empty`] laid out for the width it has.
struct EmptyLayout {
    title: std::sync::Arc<egui::Galley>,
    body: std::sync::Arc<egui::Galley>,
    /// Each hint's width, and the hints on each row with the row's width.
    hint_widths: Vec<f32>,
    hint_rows: Vec<(std::ops::Range<usize>, f32)>,
    height: f32,
}

const HINT_GAP: f32 = 24.0;
const HINT_ROW: f32 = 20.0;
const HINT_ROW_GAP: f32 = 8.0;
const BUTTON_GAP: f32 = 8.0;

impl Empty<'_> {
    /// Its height when centred in `room` points of width.
    pub fn height(&self, painter: &Painter, room: f32) -> f32 {
        self.layout(painter, room).height
    }

    fn layout(&self, painter: &Painter, room: f32) -> EmptyLayout {
        // Lines centred on the galley's x = 0; the title wraps to two lines at most.
        let centred = |text: &str, font: FontId, line_height: f32, width: f32, max_rows: usize| {
            let mut job =
                egui::text::LayoutJob::simple(text.into(), font, Color32::PLACEHOLDER, width.max(1.0));
            job.halign = egui::Align::Center;
            job.sections[0].format.line_height = Some(line_height);
            job.wrap.max_rows = max_rows;
            painter.layout_job(job)
        };
        let title = centred(self.title, FontId::new(20.0, theme::family(theme::SERIF)), 28.0, room, 2);
        let body = centred(self.body, theme::ui_font(13.0), 20.0, self.width.min(room), usize::MAX);
        let hint_widths: Vec<f32> = self
            .hints
            .iter()
            .map(|(keys, what)| {
                let caps: f32 = keys.iter().map(|k| crate::widgets::keycap_width(painter, k) + 4.0).sum();
                let t = painter
                    .layout_no_wrap((*what).into(), theme::ui_font(12.0), Color32::PLACEHOLDER)
                    .size()
                    .x;
                caps + 4.0 + t
            })
            .collect();
        // As many hints on a row as fit in the room, each row centred.
        let mut hint_rows: Vec<(std::ops::Range<usize>, f32)> = Vec::new();
        for (i, &w) in hint_widths.iter().enumerate() {
            match hint_rows.last_mut() {
                Some((r, rw)) if *rw + HINT_GAP + w <= room => {
                    r.end = i + 1;
                    *rw += HINT_GAP + w;
                }
                _ => hint_rows.push((i..i + 1, w)),
            }
        }
        let buttons = if self.buttons.is_empty() { 0.0 } else { 12.0 + 4.0 + 32.0 };
        let hints = if hint_rows.is_empty() {
            0.0
        } else {
            24.0 + hint_rows.len() as f32 * (HINT_ROW + HINT_ROW_GAP) - HINT_ROW_GAP
        };
        let height = 56.0 + 12.0 + title.size().y + 4.0 + body.size().y + buttons + hints;
        EmptyLayout { title, body, hint_widths, hint_rows, height }
    }

    /// Draws it centred on `cx` from `top`, in `room` points of width; returns the buttons'
    /// responses.
    pub fn show(&self, ui: &mut Ui, p: &Palette, cx: f32, top: f32, room: f32) -> Vec<egui::Response> {
        let painter = ui.painter().clone();
        let l = self.layout(&painter, room);
        let c = pos2(cx, top + 28.0);
        let (circle, ink) = match self.tone {
            Tone::Accent => (p.accent.gamma_multiply(0.12), p.accent),
            Tone::Danger => (p.danger.gamma_multiply(0.12), p.danger),
            Tone::Quiet => (p.fill, p.ink_3),
        };
        painter.circle_filled(c, 28.0, circle);
        let glyph = Rect::from_center_size(c, vec2(28.0, 28.0));
        if self.tone == Tone::Accent {
            crate::icons::heavy(|| (self.glyph)(&painter, glyph, ink));
        } else {
            (self.glyph)(&painter, glyph, ink);
        }
        // A centred galley's lines are centred on its own x = 0, not on the middle of its measure.
        let mut y = top + 56.0 + 12.0;
        let first_baseline = css_baseline(y, 28.0, 20.0, true) - galley_baseline(&l.title);
        y += l.title.size().y + 4.0;
        painter.galley(pos2(cx - l.title.rect.center().x, first_baseline), l.title, p.ink);
        let h = l.body.size().y;
        painter.galley(pos2(cx - l.body.rect.center().x, y), l.body, p.ink_2);
        y += h;
        let mut responses = Vec::new();
        if !self.buttons.is_empty() {
            y += 12.0 + 4.0;
            let widths: Vec<f32> = self.buttons.iter().map(|b| b.width(&painter)).collect();
            let total = widths.iter().sum::<f32>() + BUTTON_GAP * (widths.len() as f32 - 1.0);
            // Centre what shows. A ghost button at either end is only its label until hovered.
            let lead = self.buttons.first().map_or(0.0, |b| b.rest_inset());
            let trail = self.buttons.last().map_or(0.0, |b| b.rest_inset());
            let mut x = cx - (total - lead - trail) / 2.0 - lead;
            for (i, (b, w)) in self.buttons.iter().zip(widths).enumerate() {
                responses.push(b.show_left(ui, p, x, y + 16.0, ui.id().with(("empty-button", i))).1);
                x += w + BUTTON_GAP;
            }
            y += 32.0;
        }
        if !l.hint_rows.is_empty() {
            y += 24.0;
            for (row, row_w) in &l.hint_rows {
                let cy = y + HINT_ROW / 2.0;
                let mut x = cx - row_w / 2.0;
                for i in row.clone() {
                    let (keys, what) = self.hints[i];
                    let start = x;
                    for k in keys {
                        x = crate::widgets::keycap(&painter, p, x, cy, k).right() + 4.0;
                    }
                    x += 4.0;
                    painter.text(pos2(x, cy), egui::Align2::LEFT_CENTER, what, theme::ui_font(12.0), p.ink_3);
                    x = start + l.hint_widths[i] + HINT_GAP;
                }
                y += HINT_ROW + HINT_ROW_GAP;
            }
        }
        responses
    }
}

/// A page's visible height under the toolbar, in the page's scrolling coordinates (so a block
/// placed in it scrolls with the page). `content` is the page's content rectangle.
pub fn under_toolbar(ui: &Ui, content: Rect) -> egui::Rangef {
    egui::Rangef::new(content.top() - TOP, ui.max_rect().bottom())
}

/// The top of a block `h` tall, optically centred in `area`. It sits a little above the middle,
/// with 45 % of the leftover height above it, and never above `min_top`.
pub fn optical_top(area: egui::Rangef, h: f32, min_top: f32) -> f32 {
    (area.min + (area.span() - h).max(0.0) * 0.45).max(min_top)
}
