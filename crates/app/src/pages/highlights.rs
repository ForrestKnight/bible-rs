//! Study › Highlights (frame 18): cards with each verse on its highlight colour, filtered by
//! colour, book and testament, in two columns.

use eframe::egui::{self, Color32, FontId, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use bible_core::study::HighlightColor;
use bible_core::{BookId, VerseRef};

use super::{Empty, PageEnv, Tone, text};
use crate::actions::Action;
use crate::icons;
use crate::theme;
use crate::widgets::{self, Button, Kind, css_baseline, flow, paint_runs};

const CARD: f32 = 446.0;
const GAP_X: f32 = 20.0;
const GAP_Y: f32 = 16.0;
/// Verse lines: Literata 16 on 26 pt rows, each on a 24 pt highlight run.
const ROW: f32 = 26.0;

#[derive(Default)]
pub struct State {
    color: Option<HighlightColor>,
    /// 0 all, 1 Old Testament, 2 New Testament.
    testament: usize,
    book: Option<BookId>,
}

impl State {
    pub fn show(&mut self, ui: &mut Ui, content: Rect, env: &PageEnv<'_>, actions: &mut Vec<Action>) -> f32 {
        let p = env.settings.theme.palette();
        let painter = ui.painter().clone();
        let (left, right, top) = (content.left(), content.right(), content.top());
        let t = super::reading_text(env);
        let abbrev = t.as_ref().map(|t| t.meta.abbrev.clone()).unwrap_or_default();
        let book_name = |b: BookId| {
            t.as_ref().map(|t| t.meta.book_short(b).to_string()).unwrap_or_else(|| b.name().into())
        };

        let all = env.study.highlights_dated();
        let mut books: Vec<BookId> = all.iter().map(|(v, _, _)| v.book).collect();
        books.sort();
        books.dedup();
        let subtitle = match (all.len(), books.len()) {
            (0, _) => "Nothing highlighted yet".to_string(),
            (1, _) => "1 highlight".to_string(),
            (n, 1) => format!("{n} highlights in 1 book"),
            (n, b) => format!("{n} highlights in {b} books"),
        };
        super::title(&painter, p, left, top, "Highlights", &subtitle);
        if all.is_empty() {
            let empty = Empty {
                glyph: icons::highlighter,
                tone: Tone::Accent,
                title: "Your highlights will live here",
                body: "Select a verse and press 1–6 to mark it in a colour, or right-click it and pick one. \
                       0 clears it.",
                buttons: &[Button::new(Kind::Primary, "Open John 1")],
                hints: &[(&["1–6"], "Highlight the selected verse")],
                width: 360.0,
            };
            return super::bookmarks::empty_page(ui, p, content, &empty, actions);
        }

        // Header actions: a book menu, and the testament.
        let cy = top + 26.0 + 16.0;
        let (seg, pick) = widgets::segmented_tight(
            ui,
            p,
            pos2(right, cy),
            ui.id().with("testament"),
            &["All", "OT", "NT"],
            self.testament,
        );
        if let Some(i) = pick {
            self.testament = i;
            self.book = None;
        }
        let select = Rect::from_min_size(pos2(seg.left() - 8.0 - 160.0, cy - 16.0), vec2(160.0, 32.0));
        let value = self.book.map(book_name).unwrap_or_else(|| "All books".into());
        let resp = widgets::select_field(ui, p, select, &value, Some(icons::book), ui.id().with("book"));
        crate::menu::popup(&resp, p).show(|ui| {
            ui.set_min_width(180.0);
            if ui.selectable_label(self.book.is_none(), "All books").clicked() {
                self.book = None;
            }
            crate::menu::separator(ui);
            for b in &books {
                if ui.selectable_label(self.book == Some(*b), book_name(*b)).clicked() {
                    self.book = Some(*b);
                    self.testament = 0;
                }
            }
        });

        // What's shown, before the colour filter (so the tabs count what they'd show).
        let q = env.query.trim().to_lowercase();
        let verse_text =
            |v: VerseRef| t.as_ref().and_then(|t| t.find(v)).map(|v| v.plain_text()).unwrap_or_default();
        let scoped: Vec<(VerseRef, HighlightColor, i64, String)> = all
            .iter()
            .filter(|(v, _, _)| match self.testament {
                1 => v.book.0 < bible_core::books::FIRST_NT_BOOK.0,
                2 => v.book.0 >= bible_core::books::FIRST_NT_BOOK.0,
                _ => true,
            })
            .filter(|(v, _, _)| self.book.is_none_or(|b| v.book == b))
            .map(|(v, c, t)| (*v, *c, *t, verse_text(*v)))
            .filter(|(v, _, _, text)| {
                q.is_empty() || v.display().to_lowercase().contains(&q) || text.to_lowercase().contains(&q)
            })
            .collect();

        // Colour tabs.
        let y = top + 58.0 + 20.0;
        let mut x = left;
        let mut tabs: Vec<(Option<HighlightColor>, String, usize)> = vec![(None, "All".into(), scoped.len())];
        for c in HighlightColor::ALL {
            tabs.push((
                Some(c),
                env.settings.highlight_name(c),
                scoped.iter().filter(|(_, hc, _, _)| *hc == c).count(),
            ));
        }
        for (c, label, n) in tabs {
            let n = n.to_string();
            let on = self.color == c;
            let w = widgets::pill_tab_width(&painter, &label, Some(&n), on, c.is_some());
            let r = Rect::from_min_size(pos2(x, y), vec2(w, 28.0));
            let dot = c.map(|c| p.swatch(c));
            if widgets::pill_tab(ui, p, r, &label, Some(&n), dot, on, ui.id().with(("colour", &label)))
                .clicked()
            {
                self.color = c;
            }
            x = r.right() + 4.0;
        }

        // Cards in columns, each going to the shorter one.
        let width = right - left;
        let columns = if width >= 600.0 { 2 } else { 1 };
        let card_w = if columns == 2 { CARD.min((width - GAP_X) / 2.0) } else { width.min(CARD + 120.0) };
        let body = y + 28.0 + 20.0;
        let mut bottoms = vec![body; columns];
        let serif = FontId::new(16.0, theme::family(theme::SERIF));
        for (v, color, created, verse) in
            scoped.iter().filter(|(_, c, _, _)| self.color.is_none_or(|s| s == *c))
        {
            let lines = flow(&painter, verse, &serif, 12.0, card_w - 32.0 - 4.0, 3);
            let h = 16.0 + 16.0 + 10.0 + ROW * lines.len().max(1) as f32 + 16.0;
            let col = (0..columns).min_by(|a, b| bottoms[*a].total_cmp(&bottoms[*b])).unwrap_or(0);
            let card = Rect::from_min_size(
                pos2(left + col as f32 * (card_w + GAP_X), bottoms[col]),
                vec2(card_w, h),
            );
            bottoms[col] = card.bottom() + GAP_Y;
            let resp = ui.interact(card, ui.id().with(("highlight", v.key())), Sense::click());
            painter.add(
                egui::epaint::RectShape::filled(
                    card.translate(vec2(0.0, 1.0)),
                    14.0,
                    Color32::from_black_alpha(10),
                )
                .with_blur_width(4.0),
            );
            painter.rect_filled(card, 14.0, p.surface);
            let ring = if resp.hovered() { p.ink_3 } else { p.border };
            painter.rect_stroke(card, 14.0, Stroke::new(0.5, ring), StrokeKind::Outside);
            let cx = card.left() + 16.0;
            painter.circle_filled(pos2(cx + 5.0, card.top() + 24.0), 5.0, p.swatch(*color));
            let w = text(
                &painter,
                cx + 18.0,
                card.top() + 16.0,
                16.0,
                &v.display(),
                theme::ui_semibold(13.0),
                p.ink,
                false,
            );
            text(
                &painter,
                cx + 18.0 + w + 8.0,
                card.top() + 16.0,
                16.0,
                &abbrev,
                theme::ui_medium(11.0),
                p.ink_3,
                false,
            );
            let date = crate::time::short_date(*created);
            text(
                &painter,
                card.right() - 16.0,
                card.top() + 16.0,
                16.0,
                &date,
                theme::ui_font(12.0),
                p.ink_3,
                true,
            );
            for (k, l) in lines.iter().enumerate() {
                let row_top = card.top() + 42.0 + ROW * k as f32;
                let lw = widgets::runs_width(&painter, l);
                let run = Rect::from_min_size(pos2(cx, row_top + 1.0), vec2(lw + 4.0, 24.0));
                painter.rect_filled(run, 3.0, p.highlight(*color));
                paint_runs(&painter, cx + 2.0, css_baseline(run.top(), 24.0, 16.0, true), l, p.ink);
            }
            if resp.clicked() {
                actions.push(Action::Jump { chapter: v.chapter_idx(), verse: Some(v.verse), flash: true });
            }
            resp.context_menu(|ui| {
                if ui.button("Remove Highlight").clicked() {
                    actions.push(Action::RemoveHighlight(*v));
                }
            });
        }
        bottoms.into_iter().fold(body, f32::max)
    }
}
