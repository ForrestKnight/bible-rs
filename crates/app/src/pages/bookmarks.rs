//! Study › Bookmarks (frame 17): every bookmarked verse in full, grouped by book (or newest
//! first), with go-to and remove on hover.

use eframe::egui::{FontId, Rect, Sense, Stroke, Ui, pos2, vec2};

use bible_core::VerseRef;

use super::{Empty, Page, PageEnv, Tone, group_label, text};
use crate::actions::{Action, ExportKind};
use crate::icons;
use crate::theme;
use crate::widgets::{self, Button, Kind, css_baseline, flow, paint_runs};

/// Verse text lines: Literata 17 on 28 pt lines, two at most.
const LINE: f32 = 28.0;
const TEXT_WIDTH: f32 = 620.0;

#[derive(Default)]
pub struct State {
    /// Newest first rather than in canonical order.
    recent: bool,
}

impl State {
    pub fn show(&mut self, ui: &mut Ui, content: Rect, env: &PageEnv<'_>, actions: &mut Vec<Action>) -> f32 {
        let p = env.settings.theme.palette();
        let painter = ui.painter().clone();
        let (left, right, top) = (content.left(), content.right(), content.top());
        let t = super::reading_text(env);
        let abbrev = t.as_ref().map(|t| t.meta.abbrev.clone()).unwrap_or_default();
        let verse_text =
            |v: VerseRef| t.as_ref().and_then(|t| t.find(v)).map(|v| v.plain_text()).unwrap_or_default();

        let all = env.study.bookmarks();
        let mut books: Vec<_> = all.iter().map(|(v, _)| v.book).collect();
        books.dedup();
        let subtitle = match (all.len(), books.len()) {
            (0, _) => "Nothing bookmarked yet".to_string(),
            (1, _) => "1 verse".to_string(),
            (n, 1) => format!("{n} verses in 1 book"),
            (n, b) => format!("{n} verses in {b} books"),
        };
        super::title(&painter, p, left, top, "Bookmarks", &subtitle);
        if all.is_empty() {
            let empty = Empty {
                glyph: |p, r, c| icons::bookmark(p, r, c, false),
                tone: Tone::Accent,
                title: "Your bookmarks will live here",
                body: "Select a verse and press B, or right-click it and choose Bookmark. Bookmarks stay on \
                       this Mac and export with your notes.",
                buttons: &[Button::new(Kind::Primary, "Open John 1")],
                hints: &[(&["B"], "Bookmark the selected verse")],
                width: 360.0,
            };
            return empty_page(ui, p, content, &empty, actions);
        }

        let cy = top + 26.0 + 16.0;
        let (export, resp) = Button::new(Kind::Secondary, "Export").icon(icons::upload).show_right(
            ui,
            p,
            right,
            cy,
            ui.id().with("export"),
        );
        if resp.clicked() {
            actions.push(Action::Export(ExportKind::Markdown));
        }
        let (_, pick) = widgets::segmented_tight(
            ui,
            p,
            pos2(export.left() - 8.0, cy),
            ui.id().with("order"),
            &["Canonical", "Recent"],
            usize::from(self.recent),
        );
        if let Some(i) = pick {
            self.recent = i == 1;
        }

        // The rows, filtered by the toolbar field.
        let q = env.query.trim().to_lowercase();
        let mut rows: Vec<(VerseRef, i64, String)> = all
            .iter()
            .map(|(v, b)| (*v, b.created, verse_text(*v)))
            .filter(|(v, _, text)| {
                q.is_empty() || v.display().to_lowercase().contains(&q) || text.to_lowercase().contains(&q)
            })
            .collect();
        if self.recent {
            rows.sort_by_key(|(v, created, _)| (std::cmp::Reverse(*created), *v));
        }
        let serif = FontId::new(17.0, theme::family(theme::SERIF));
        let width = TEXT_WIDTH.min(right - left - 44.0 - 16.0 - 80.0);
        let mut y = top + 58.0 + 20.0;
        let mut group = None;
        for (v, created, verse) in rows {
            if !self.recent && group != Some(v.book) {
                group = Some(v.book);
                let name = t
                    .as_ref()
                    .map(|t| t.meta.book_short(v.book).to_string())
                    .unwrap_or_else(|| v.book.name().into());
                group_label(&painter, p, left, y + 12.0, &name);
                y += 32.0;
            }
            let lines = flow(&painter, &verse, &serif, 13.0, width, 2);
            let h = 12.0 + 22.0 + LINE * lines.len().max(1) as f32 + 12.0;
            let row = Rect::from_min_size(pos2(left, y), vec2(right - left, h));
            let resp = ui.interact(row, ui.id().with(("bookmark", v.key())), Sense::click());
            let hovered = ui.rect_contains_pointer(row);
            if hovered {
                let fill = if resp.is_pointer_button_down_on() { p.fill_strong } else { p.fill };
                painter.rect_filled(row, 12.0, fill);
            } else {
                painter.hline(row.x_range(), row.bottom() - 0.5, Stroke::new(1.0, p.separator));
            }
            let glyph = Rect::from_min_size(pos2(left + 14.0, y + 18.0), vec2(16.0, 16.0));
            icons::heavy(|| icons::bookmark(&painter, glyph, p.accent, true));
            let x = left + 44.0;
            let w = text(&painter, x, y + 12.0, 20.0, &v.display(), theme::ui_semibold(13.0), p.ink, false);
            text(&painter, x + w + 8.0, y + 14.0, 16.0, &abbrev, theme::ui_medium(11.0), p.ink_3, false);
            for (k, l) in lines.iter().enumerate() {
                let base = css_baseline(y + 34.0 + LINE * k as f32, 24.0, 17.0, true);
                paint_runs(&painter, x, base, l, p.ink);
            }
            let mut acted = false;
            if hovered {
                let go = Rect::from_min_size(pos2(right - 16.0 - 58.0, y + 12.0), vec2(28.0, 28.0));
                let id = ui.id().with(("bookmark-go", v.key()));
                if widgets::round_button(ui, p, go, icons::arrow_right, id).clicked() {
                    acted = true;
                    open(v, actions);
                }
                let bin = go.translate(vec2(30.0, 0.0));
                let id = ui.id().with(("bookmark-remove", v.key()));
                if widgets::round_button(ui, p, bin, icons::trash, id).clicked() {
                    acted = true;
                    actions.push(Action::RemoveBookmark(v));
                }
            } else {
                let date = crate::time::short_date(created);
                text(&painter, right - 16.0, y + 16.0, 16.0, &date, theme::ui_font(12.0), p.ink_3, true);
            }
            if resp.clicked() && !acted {
                open(v, actions);
            }
            y += h;
        }
        y
    }
}

fn open(v: VerseRef, actions: &mut Vec<Action>) {
    actions.push(Action::Jump { chapter: v.chapter_idx(), verse: Some(v.verse), flash: true });
}

/// An empty page: the state optically centred in the page under the toolbar, below the header.
/// Its first button opens John 1, its second imports a backup.
pub(super) fn empty_page(
    ui: &mut Ui,
    p: &theme::Palette,
    content: Rect,
    empty: &Empty<'_>,
    actions: &mut Vec<Action>,
) -> f32 {
    let room = content.width();
    let h = empty.height(ui.painter(), room);
    let top = super::optical_top(super::under_toolbar(ui, content), h, content.top() + 58.0 + 20.0);
    let buttons = empty.show(ui, p, content.center().x, top, room);
    if buttons.first().is_some_and(|r| r.clicked()) {
        let john = bible_core::ChapterIdx::new(bible_core::BookId(42), 1).expect("John 1");
        actions.push(Action::Jump { chapter: john, verse: None, flash: false });
        actions.push(Action::OpenPage(Page::Reader));
    }
    if buttons.get(1).is_some_and(|r| r.clicked()) {
        actions.push(Action::ImportStudy);
    }
    top + h
}
