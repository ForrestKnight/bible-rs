//! ⌘K: jump to a reference ("jn 3:16", "Rom 8", "1co13"), search, switch translation or run a
//! command, all from the keyboard.

use bible_core::library::Library;
use bible_core::reference;
use eframe::egui::{self, Align2, CornerRadius, Frame, Key, Margin, Modifiers, Order, Sense, Stroke, vec2};

use crate::actions::{Action, ExportKind, SettingsTab};
use crate::app::{History, Settings};
use crate::theme::{self, Theme};

#[derive(Default)]
pub struct CommandPalette {
    query: String,
    selected: usize,
    opened: bool,
}

struct Item {
    title: String,
    detail: String,
    action: Action,
}

fn commands(settings: &Settings) -> Vec<(&'static str, Action)> {
    vec![
        ("Toggle parallel translations", Action::ToggleParallel),
        ("Theme: Light", Action::SetTheme(Theme::Light)),
        ("Theme: Sepia", Action::SetTheme(Theme::Sepia)),
        ("Theme: Dark", Action::SetTheme(Theme::Dark)),
        (
            if settings.typo.mode == crate::reader::layout::Mode::Paragraph {
                "Layout: verse per line"
            } else {
                "Layout: paragraphs"
            },
            Action::ToggleMode,
        ),
        (
            if settings.typo.red_letters { "Red letters: off" } else { "Red letters: on" },
            Action::ToggleRedLetters,
        ),
        ("Bigger text", Action::FontSize(1.0)),
        ("Smaller text", Action::FontSize(-1.0)),
        ("Toggle sidebar", Action::ToggleSidebar),
        ("Export notes as Markdown…", Action::Export(ExportKind::Markdown)),
        ("Export notes as JSON…", Action::Export(ExportKind::Json)),
        ("Import a translation…", Action::ImportTranslation),
        ("Manage translations & API keys", Action::OpenSettings(SettingsTab::Translations)),
        ("Settings", Action::OpenSettings(SettingsTab::Reading)),
        ("Keyboard shortcuts", Action::ShowShortcuts),
    ]
}

impl CommandPalette {
    fn items(&self, library: &Library, history: &History, settings: &Settings) -> Vec<Item> {
        let q = self.query.trim();
        let mut items = Vec::new();
        let meta = library.get(&settings.translation).map(|e| e.meta.clone());
        let book_name = |b: bible_core::BookId| {
            meta.as_ref().map(|m| m.ref_name(b).to_string()).unwrap_or_else(|| b.ref_name().into())
        };
        if q.is_empty() {
            for (c, v) in history.recent().take(8) {
                items.push(Item {
                    title: format!("{} {}:{}", book_name(c.book()), c.chapter(), v),
                    detail: "Recent".into(),
                    action: Action::Jump { chapter: *c, verse: Some(*v), flash: true },
                });
            }
        } else {
            for r in reference::suggest(q, 6) {
                let mut title = format!("{} {}", book_name(r.book), r.chapter);
                if let Some(v) = r.verse {
                    title.push_str(&format!(":{v}"));
                    if let Some(e) = r.verse_end.filter(|&e| e > v) {
                        title.push_str(&format!("–{e}"));
                    }
                }
                items.push(Item {
                    title,
                    detail: "Go to".into(),
                    action: Action::Jump {
                        chapter: r.chapter_idx(),
                        verse: r.verse,
                        flash: r.verse.is_some(),
                    },
                });
            }
            items.push(Item {
                title: format!("Search for “{q}”"),
                detail: "Full text".into(),
                action: Action::OpenSearch(Some(q.to_string())),
            });
            let ql = q.to_lowercase();
            for e in library.entries() {
                let m = &e.meta;
                if m.abbrev.to_lowercase().contains(&ql) || m.name.to_lowercase().contains(&ql) {
                    items.push(Item {
                        title: format!("Read {} — {}", m.abbrev, m.name),
                        detail: "Translation".into(),
                        action: Action::SetTranslation(m.id.clone()),
                    });
                }
            }
            for (title, action) in commands(settings) {
                if title.to_lowercase().contains(&ql) {
                    items.push(Item { title: title.into(), detail: "Command".into(), action });
                }
            }
        }
        items
    }

    /// Returns `true` when the palette should close.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        library: &Library,
        history: &History,
        settings: &Settings,
        actions: &mut Vec<Action>,
    ) -> bool {
        let p = settings.theme.palette();
        let items = self.items(library, history, settings);
        if self.selected >= items.len() {
            self.selected = items.len().saturating_sub(1);
        }
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
        if down && !items.is_empty() {
            self.selected = (self.selected + 1) % items.len();
        }
        if up && !items.is_empty() {
            self.selected = (self.selected + items.len() - 1) % items.len();
        }
        if enter {
            if let Some(it) = items.get(self.selected) {
                actions.push(it.action.clone());
            }
            return true;
        }
        let screen = ctx.content_rect();
        let mut close = false;
        // Dim the page; clicking outside closes.
        egui::Area::new(egui::Id::new("palette-backdrop")).order(Order::Middle).fixed_pos(screen.min).show(
            ctx,
            |ui| {
                let (r, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter().rect_filled(
                    r,
                    0.0,
                    egui::Color32::from_black_alpha(if settings.theme.is_dark() { 90 } else { 40 }),
                );
                if resp.clicked() {
                    close = true;
                }
            },
        );
        let width = 580.0f32.min(screen.width() - 40.0);
        egui::Area::new(egui::Id::new("palette"))
            .order(Order::Foreground)
            .fixed_pos(egui::pos2(screen.center().x, screen.top() + 84.0))
            .pivot(Align2::CENTER_TOP)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(p.surface)
                    .stroke(Stroke::new(1.0, p.separator))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(10))
                    .shadow(ui.visuals().window_shadow)
                    .show(ui, |ui| {
                        ui.set_width(width);
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(vec2(22.0, 30.0), Sense::hover());
                            crate::icons::search(ui.painter(), r.shrink(3.0), p.ink_2);
                            let edit = egui::TextEdit::singleline(&mut self.query)
                                .hint_text("Go to “jn 3:16”, “Rom 8”, search, or type a command…")
                                .font(theme::ui_font(16.0))
                                .frame(egui::Frame::NONE)
                                .desired_width(f32::INFINITY);
                            let resp = ui.add(edit);
                            if !self.opened {
                                resp.request_focus();
                                self.opened = true;
                            }
                            if resp.changed() {
                                self.selected = 0;
                            }
                        });
                        if !items.is_empty() {
                            ui.add_space(4.0);
                            ui.separator();
                        }
                        for (i, it) in items.iter().enumerate().take(14) {
                            let (r, resp) = ui.allocate_exact_size(vec2(width, 32.0), Sense::click());
                            if i == self.selected {
                                ui.painter().rect_filled(r, CornerRadius::same(7), p.fill);
                            }
                            if resp.hovered() && ui.input(|i| i.pointer.delta().length() > 0.0) {
                                self.selected = i;
                            }
                            ui.painter().text(
                                r.left_center() + vec2(10.0, 0.0),
                                Align2::LEFT_CENTER,
                                &it.title,
                                if i == self.selected {
                                    theme::ui_medium(14.0)
                                } else {
                                    theme::ui_font(14.0)
                                },
                                p.ink,
                            );
                            ui.painter().text(
                                r.right_center() - vec2(10.0, 0.0),
                                Align2::RIGHT_CENTER,
                                &it.detail,
                                theme::ui_font(11.5),
                                p.ink_2,
                            );
                            if resp.clicked() {
                                actions.push(it.action.clone());
                                close = true;
                            }
                        }
                    });
            });
        close
    }
}
