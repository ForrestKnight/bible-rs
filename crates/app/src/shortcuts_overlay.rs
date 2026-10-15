//! The keyboard shortcuts overlay (frame 29, `?`): every shortcut in three columns with keycaps,
//! a filter, and a way to change them in Settings.

use std::collections::BTreeMap;

use eframe::egui::{self, Order, Rect, Sense, Stroke, pos2, vec2};

use crate::actions::{Action, SettingsTab};
use crate::icons;
use crate::keymap;
use crate::pages::text;
use crate::theme::{self, Palette};
use crate::widgets::{self, Button, Kind};

const WIDTH: f32 = 960.0;
const COLUMN: f32 = 280.0;
const ROW: f32 = 32.0;

#[derive(Default)]
pub struct ShortcutsOverlay {
    filter: String,
}

impl ShortcutsOverlay {
    /// Draws the overlay; returns true when it should close.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        p: &Palette,
        overrides: &BTreeMap<String, String>,
        actions: &mut Vec<Action>,
    ) -> bool {
        let mut close = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        let screen = ctx.content_rect();
        egui::Area::new(egui::Id::new("shortcuts-scrim")).order(Order::Middle).fixed_pos(screen.min).show(
            ctx,
            |ui| {
                let (r, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter().rect_filled(r, 0.0, p.scrim);
                close |= resp.clicked();
            },
        );
        let q = self.filter.trim().to_lowercase();
        let columns: Vec<(&str, Vec<&keymap::Shortcut>)> = keymap::GROUPS
            .iter()
            .map(|g| {
                let rows = keymap::ALL
                    .iter()
                    .filter(|s| s.group == *g && (q.is_empty() || s.label.to_lowercase().contains(&q)))
                    .collect();
                (*g, rows)
            })
            .collect();
        let tallest = columns.iter().map(|(_, r)| r.len()).max().unwrap_or(0).max(1);
        let height = 60.0 + 20.0 + 21.0 + ROW * tallest as f32 + 8.0 + 44.0;
        let width = WIDTH.min(screen.width() - 48.0);
        let rect = Rect::from_min_size(
            pos2((screen.center().x - width / 2.0).round(), screen.top() + 72.0),
            vec2(width, height),
        );
        egui::Area::new(egui::Id::new("shortcuts-overlay"))
            .order(Order::Foreground)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                ui.allocate_rect(rect, Sense::hover());
                let painter = ui.painter().clone();
                p.shadows.modal.paint(&painter, rect, 16.0);
                painter.rect_filled(rect, 16.0, p.surface);

                // Head: title, filter, close.
                text(
                    &painter,
                    rect.left() + 24.0,
                    rect.top() + 20.0,
                    20.0,
                    "Keyboard Shortcuts",
                    theme::ui_semibold(15.0),
                    p.ink,
                    false,
                );
                let close_r = Rect::from_min_size(
                    pos2(rect.right() - 16.0 - 28.0, rect.top() + 16.0),
                    vec2(28.0, 28.0),
                );
                if widgets::round_button(ui, p, close_r, icons::close, ui.id().with("close-shortcuts"))
                    .clicked()
                {
                    close = true;
                }
                let field = Rect::from_min_size(
                    pos2(close_r.left() - 12.0 - 200.0, rect.top() + 14.0),
                    vec2(200.0, 32.0),
                );
                widgets::text_field(
                    ui,
                    p,
                    field,
                    &mut self.filter,
                    "Filter",
                    Some(icons::search),
                    false,
                    false,
                    egui::Id::new("shortcuts-filter"),
                );
                painter.hline(rect.x_range(), rect.top() + 59.5, Stroke::new(1.0, p.separator));

                // Three columns.
                let mut x = rect.left() + 24.0;
                for (group, rows) in &columns {
                    let top = rect.top() + 60.0 + 20.0;
                    let g = widgets::line(
                        &painter,
                        &group.to_uppercase(),
                        theme::ui_semibold(11.0),
                        p.accent,
                        0.12,
                    );
                    widgets::paint_on_baseline(
                        &painter,
                        x,
                        widgets::css_baseline(top, 16.0, 11.0, false),
                        g,
                        p.accent,
                    );
                    let mut y = top + 21.0;
                    for s in rows {
                        text(&painter, x, y + 6.0, 20.0, s.label, theme::ui_font(13.0), p.ink, false);
                        let caps = keymap::keycaps(s, overrides);
                        let widths: Vec<f32> =
                            caps.iter().map(|c| widgets::keycap_width(&painter, c)).collect();
                        let mut kx = x + COLUMN
                            - widths.iter().sum::<f32>()
                            - 4.0 * (widths.len().max(1) as f32 - 1.0);
                        for (c, w) in caps.iter().zip(widths) {
                            widgets::keycap(&painter, p, kx, y + 16.0, c);
                            kx += w + 4.0;
                        }
                        y += ROW;
                    }
                    x += COLUMN + 32.0;
                }
                if columns.iter().all(|(_, r)| r.is_empty()) {
                    text(
                        &painter,
                        rect.left() + 24.0,
                        rect.top() + 80.0,
                        20.0,
                        "No shortcut matches.",
                        theme::ui_font(13.0),
                        p.ink_3,
                        false,
                    );
                }

                // Foot.
                let foot = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 44.0), rect.max);
                painter.rect_filled(foot, egui::CornerRadius { nw: 0, ne: 0, sw: 16, se: 16 }, p.fill);
                painter.hline(foot.x_range(), foot.top() + 0.5, Stroke::new(1.0, p.separator));
                text(
                    &painter,
                    foot.left() + 24.0,
                    foot.top() + 14.0,
                    16.0,
                    "Press ? any time to see this.",
                    theme::ui_font(12.0),
                    p.ink_3,
                    false,
                );
                let (_, resp) = Button::new(Kind::Ghost, "Customize in Settings")
                    .small()
                    .trailing_icon(icons::arrow_right)
                    .show_right(
                        ui,
                        p,
                        foot.right() - 16.0,
                        foot.center().y,
                        ui.id().with("customize-shortcuts"),
                    );
                if resp.clicked() {
                    actions.push(Action::OpenSettings(SettingsTab::Shortcuts));
                    close = true;
                }
            });
        close
    }
}
