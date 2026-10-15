//! The verse menu (frame 08a): right-click a verse for the highlight swatches (1–6, 0 clears),
//! note, bookmark, copy, share, compare and parallel.

use bible_core::VerseRef;
use bible_core::study::{HighlightColor, StudyStore};
use eframe::egui::{
    self, Align2, Color32, Key, Modifiers, Order, Painter, Pos2, Rect, Sense, Stroke, pos2, vec2,
};

use crate::actions::Action;
use crate::icons;
use crate::theme::{Palette, text};

const WIDTH: f32 = 264.0;
const PAD: f32 = 6.0;
const ITEM_H: f32 = 28.0;

type IconFn = fn(&Painter, Rect, Color32);

enum Line {
    Item { icon: IconFn, label: String, shortcut: &'static str, action: Option<Action> },
    Separator,
}

pub struct VerseMenu {
    pos: Pos2,
    /// The item under the keyboard (and the pointer).
    highlighted: Option<usize>,
    opened: bool,
}

impl VerseMenu {
    pub fn new(pos: Pos2) -> Self {
        Self { pos, highlighted: None, opened: false }
    }

    /// Starts with the first item highlighted, as when opened from the keyboard.
    pub fn with_first_highlighted(mut self) -> Self {
        self.highlighted = Some(0);
        self
    }

    fn lines(study: &StudyStore, refs: &[VerseRef]) -> Vec<Line> {
        let first = refs[0];
        let item = |icon: IconFn, label: &str, shortcut: &'static str, action: Option<Action>| Line::Item {
            icon,
            label: label.to_string(),
            shortcut,
            action,
        };
        vec![
            item(
                icons::note,
                if study.note(first).is_some() { "Edit Note…" } else { "Add Note…" },
                "N",
                Some(Action::EditSelectedNote),
            ),
            item(
                bookmark,
                if study.is_bookmarked(first) { "Remove Bookmark" } else { "Bookmark" },
                "B",
                Some(Action::ToggleBookmark),
            ),
            Line::Separator,
            item(icons::copy, "Copy", "⌘C", Some(Action::CopyText)),
            item(icons::quote, "Copy with Reference", "⇧⌘C", Some(Action::Copy)),
            item(icons::share, "Share as Image…", "", Some(Action::ShareImage)),
            Line::Separator,
            item(icons::compare, "Compare Translations", "C", Some(Action::Compare)),
            item(icons::columns, "Open in Parallel", "⇧⌘P", Some(Action::ToggleParallel)),
            Line::Separator,
            item(icons::text_cursor, "Select Verse Range", "⇧-click", None),
        ]
    }

    /// Returns `true` when the menu should close.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        p: &Palette,
        study: &StudyStore,
        refs: &[VerseRef],
        actions: &mut Vec<Action>,
    ) -> bool {
        if refs.is_empty() {
            return true;
        }
        let lines = Self::lines(study, refs);
        let enabled: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| matches!(l, Line::Item { action: Some(_), .. }))
            .map(|(i, _)| i)
            .collect();
        let (down, up, enter, esc) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::ArrowDown),
                i.consume_key(Modifiers::NONE, Key::ArrowUp),
                i.consume_key(Modifiers::NONE, Key::Enter),
                i.consume_key(Modifiers::NONE, Key::Escape),
            )
        });
        if esc {
            return true;
        }
        let at = self.highlighted.and_then(|h| enabled.iter().position(|i| *i == h));
        if down {
            self.highlighted = Some(enabled[at.map_or(0, |a| (a + 1) % enabled.len())]);
        }
        if up {
            self.highlighted =
                Some(enabled[at.map_or(enabled.len() - 1, |a| (a + enabled.len() - 1) % enabled.len())]);
        }
        if enter && let Some(Line::Item { action: Some(a), .. }) = self.highlighted.and_then(|h| lines.get(h))
        {
            actions.push(a.clone());
            return true;
        }

        let height = PAD
            + 24.0
            + 34.0
            + lines.iter().map(|l| if matches!(l, Line::Separator) { 9.0 } else { ITEM_H }).sum::<f32>()
            + 9.0
            + PAD;
        let screen = ctx.content_rect();
        let pos = pos2(
            self.pos.x.min(screen.right() - WIDTH - 8.0).max(screen.left() + 8.0),
            self.pos.y.min(screen.bottom() - height - 8.0).max(screen.top() + 8.0),
        );
        let current =
            refs.iter().map(|v| study.highlight(*v)).reduce(|a, b| if a == b { a } else { None }).flatten();
        let mut close = false;
        let area = egui::Area::new(egui::Id::new("verse-menu")).order(Order::Foreground).fixed_pos(pos).show(
            ctx,
            |ui| {
                let rect = Rect::from_min_size(pos, vec2(WIDTH, height));
                p.shadows.popover.paint(ui.painter(), rect, 12.0);
                ui.painter().rect_filled(rect, 12.0, p.surface);
                let (_, _) = ui.allocate_exact_size(rect.size(), Sense::hover());
                let left = rect.left() + PAD;
                let inner_w = WIDTH - 2.0 * PAD;
                let mut y = rect.top() + PAD;
                // "Highlight", then the swatches and clear.
                ui.painter().text(
                    pos2(left + 10.0, y + 12.0),
                    Align2::LEFT_CENTER,
                    "Highlight",
                    text::CAPTION.font(),
                    p.ink_3,
                );
                y += 24.0;
                let cy = y + 6.0 + 10.0;
                let mut x = left + 10.0;
                for (k, c) in HighlightColor::ALL.into_iter().enumerate() {
                    let center = pos2(x + 10.0, cy);
                    let resp = ui.interact(
                        Rect::from_center_size(center, vec2(24.0, 24.0)),
                        ui.id().with(("swatch", k)),
                        Sense::click(),
                    );
                    if current == Some(c) {
                        ui.painter().circle_stroke(center, 10.0 + 2.75, Stroke::new(1.5, p.ink));
                    } else if resp.hovered() {
                        ui.painter().circle_filled(center, 10.0 + 3.0, p.fill_strong);
                    }
                    ui.painter().circle_filled(center, 10.0, p.swatch(c));
                    ui.painter().circle_stroke(center, 9.75, Stroke::new(0.5, Color32::from_black_alpha(46)));
                    if current == Some(c) {
                        let s = Stroke::new(1.75, Color32::WHITE);
                        ui.painter().line_segment([center + vec2(-4.0, 0.25), center + vec2(-1.25, 3.0)], s);
                        ui.painter().line_segment([center + vec2(-1.25, 3.0), center + vec2(4.0, -2.75)], s);
                    }
                    if resp.on_hover_text(format!("{} ({})", c.name(), k + 1)).clicked() {
                        actions.push(Action::Highlight(if current == Some(c) { None } else { Some(c) }));
                        close = true;
                    }
                    x += 20.0 + 10.0;
                }
                let clear = Rect::from_min_size(pos2(x, cy - 10.0), vec2(20.0, 20.0));
                let resp = ui.interact(clear.expand(2.0), ui.id().with("swatch-clear"), Sense::click());
                if resp.hovered() {
                    ui.painter().circle_filled(clear.center(), 13.0, p.fill_strong);
                }
                icons::slash_circle(ui.painter(), clear, p.ink_3);
                if resp.on_hover_text("Remove highlight (0)").clicked() {
                    actions.push(Action::Highlight(None));
                    close = true;
                }
                y += 34.0;
                separator(ui.painter(), p, left, inner_w, y);
                y += 9.0;

                for (i, line) in lines.iter().enumerate() {
                    match line {
                        Line::Separator => {
                            separator(ui.painter(), p, left, inner_w, y);
                            y += 9.0;
                        }
                        Line::Item { icon, label, shortcut, action } => {
                            let r = Rect::from_min_size(pos2(left, y), vec2(inner_w, ITEM_H));
                            let resp = ui.interact(r, ui.id().with(("menu-item", i)), Sense::click());
                            if action.is_some()
                                && resp.hovered()
                                && ui.input(|inp| inp.pointer.delta() != egui::Vec2::ZERO)
                            {
                                self.highlighted = Some(i);
                            }
                            let lit = action.is_some() && self.highlighted == Some(i);
                            if lit {
                                ui.painter().rect_filled(r, 6.0, p.accent);
                                if resp.is_pointer_button_down_on() {
                                    ui.painter().rect_filled(r, 6.0, Color32::from_black_alpha(18));
                                }
                            }
                            let (ink, ink2, ink3) = match (action.is_some(), lit) {
                                (false, _) => (p.ink_3, p.ink_3, p.ink_3),
                                (true, true) => (p.on_accent, p.on_accent, p.on_accent),
                                (true, false) => (p.ink, p.ink_2, p.ink_3),
                            };
                            let cy = r.center().y;
                            icons::heavy(|| {
                                icon(
                                    ui.painter(),
                                    Rect::from_center_size(pos2(r.left() + 8.0 + 8.0, cy), vec2(16.0, 16.0)),
                                    ink2,
                                )
                            });
                            ui.painter().text(
                                pos2(r.left() + 8.0 + 16.0 + 8.0, cy),
                                Align2::LEFT_CENTER,
                                label,
                                text::UI.font(),
                                ink,
                            );
                            ui.painter().text(
                                pos2(r.right() - 10.0, cy),
                                Align2::RIGHT_CENTER,
                                *shortcut,
                                text::UI.font(),
                                ink3,
                            );
                            if let Some(a) = action
                                && resp.clicked()
                            {
                                actions.push(a.clone());
                                close = true;
                            }
                            y += ITEM_H;
                        }
                    }
                }
            },
        );
        if !self.opened {
            self.opened = true;
        } else if area.response.clicked_elsewhere() {
            close = true;
        }
        close
    }
}

fn bookmark(p: &Painter, r: Rect, c: Color32) {
    icons::bookmark(p, r, c, false);
}

fn separator(painter: &Painter, p: &Palette, left: f32, width: f32, y: f32) {
    painter.hline(left + 8.0..=left + width - 8.0, y + 4.5, Stroke::new(1.0, p.separator));
}
