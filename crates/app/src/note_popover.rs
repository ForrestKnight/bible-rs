//! The note popover (frame 08b): a verse's note, written in a popover anchored to its margin
//! marker — reference and highlight colour, the verse quoted, the editor, then delete, the saved
//! state and Done.

use bible_core::VerseRef;
use bible_core::library::Library;
use bible_core::study::{HighlightColor, StudyStore};
use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{
    self, Align2, Color32, FontId, Key, Modifiers, Order, Pos2, Rect, Sense, Stroke, pos2, vec2,
};

use crate::actions::Action;
use crate::icons;
use crate::theme::{self, Palette, text};

const WIDTH: f32 = 344.0;
const PAD: f32 = 16.0;
/// The beak: 18 × 9, its tip 173 pt from the popover's left edge (moved to meet the marker).
const BEAK: egui::Vec2 = vec2(18.0, 9.0);

pub struct NotePopover {
    pub verse: VerseRef,
    note: String,
    saved: String,
    /// When the note was last saved (Unix seconds), and last edited (egui time).
    saved_at: Option<i64>,
    edited_at: f64,
    focus: bool,
}

impl NotePopover {
    pub fn new(verse: VerseRef, study: &StudyStore) -> Self {
        let existing = study.note(verse);
        let note = existing.map(|n| n.body.clone()).unwrap_or_default();
        let saved_at = existing.map(|n| n.updated);
        Self { verse, saved: note.clone(), note, saved_at, edited_at: 0.0, focus: true }
    }

    pub fn save(&mut self, study: &mut StudyStore, actions: &mut Vec<Action>) {
        if self.note != self.saved {
            match study.set_note(self.verse, &self.note) {
                Ok(()) => {
                    self.saved = self.note.clone();
                    self.saved_at = Some(bible_core::study::now());
                }
                Err(e) => actions.push(Action::Toast(format!("Couldn't save note: {e:#}"))),
            }
        }
    }

    /// Draws the popover under `marker` (the verse's note marker, in window points). Returns
    /// `true` when it should close.
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        p: &Palette,
        library: &Library,
        study: &mut StudyStore,
        translation: &str,
        marker: Rect,
        actions: &mut Vec<Action>,
    ) -> bool {
        let now = ctx.input(|i| i.time);
        let esc = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        let done_key = ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter));
        if esc || done_key {
            self.save(study, actions);
            return true;
        }
        let meta = library.get(translation).map(|e| e.meta.clone());
        let reference = format!(
            "{} {}:{}",
            meta.as_ref().map(|m| m.ref_name(self.verse.book).to_string()).unwrap_or_else(|| self
                .verse
                .book
                .ref_name()
                .into()),
            self.verse.chapter,
            self.verse.verse
        );
        let abbrev = meta.as_ref().map(|m| m.abbrev.clone()).unwrap_or_default();
        let verse_text = library
            .load(translation)
            .ok()
            .and_then(|t| t.find(self.verse).map(|v| v.plain_text()))
            .unwrap_or_default();
        let colour = study.highlight(self.verse);

        let screen = ctx.content_rect();
        let tip_x = marker.center().x;
        let left = (tip_x - 173.0).clamp(screen.left() + 8.0, screen.right() - WIDTH - 8.0);
        let top = marker.bottom() + 5.0 + BEAK.y;
        let inner_w = WIDTH - 2.0 * PAD;

        // Lay out the variable parts first to size the popover.
        let quote = {
            let mut job = LayoutJob::single_section(
                verse_text,
                TextFormat {
                    font_id: FontId::new(text::SNIPPET_SM.size, theme::family(theme::SERIF_ITALIC)),
                    color: p.ink_2,
                    line_height: Some(20.0),
                    ..Default::default()
                },
            );
            job.wrap = TextWrapping {
                max_width: inner_w - 12.0,
                max_rows: 2,
                break_anywhere: false,
                overflow_character: Some('…'),
            };
            ctx.fonts_mut(|f| f.layout_job(job))
        };
        let editor_h = 100.0;
        let height = PAD + 16.0 + 12.0 + quote.size().y + 12.0 + editor_h + 12.0 + 28.0 + PAD;
        let rect = Rect::from_min_size(pos2(left, top), vec2(WIDTH, height));
        let mut close = false;
        let area = egui::Area::new(egui::Id::new("note-popover"))
            .order(Order::Foreground)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                let (_, _) = ui.allocate_exact_size(rect.size(), Sense::hover());
                let painter = ui.painter().clone();
                let painter = &painter;
                p.shadows.popover.paint(painter, rect, theme::radius::LG);
                painter.rect_filled(rect, theme::radius::LG, p.surface);
                // The beak, in the surface colour with a hairline.
                let tip = pos2(tip_x.clamp(rect.left() + 24.0, rect.right() - 24.0), rect.top() - BEAK.y);
                let l = pos2(tip.x - BEAK.x / 2.0, rect.top() + 0.5);
                let r = pos2(tip.x + BEAK.x / 2.0, rect.top() + 0.5);
                painter.add(egui::Shape::convex_polygon(vec![l, tip, r], p.surface, Stroke::NONE));
                painter.add(egui::Shape::line(vec![l, tip, r], Stroke::new(0.75, p.border)));
                let x = rect.left() + PAD;
                let mut y = rect.top() + PAD;

                // Header: note glyph, reference, translation, the highlight swatches.
                let cy = y + 8.0;
                icons::heavy(|| {
                    icons::note(
                        painter,
                        Rect::from_center_size(pos2(x + 8.0, cy), vec2(16.0, 16.0)),
                        p.accent,
                    )
                });
                let r_ref = painter.text(
                    pos2(x + 16.0 + 6.0, cy),
                    Align2::LEFT_CENTER,
                    &reference,
                    text::UI_STRONG.font(),
                    p.ink,
                );
                painter.text(
                    pos2(r_ref.right() + 6.0, cy),
                    Align2::LEFT_CENTER,
                    &abbrev,
                    theme::ui_medium(11.0),
                    p.ink_3,
                );
                let mut sx = rect.right() - PAD - 6.0 * 14.0 - 5.0 * 6.0;
                for c in HighlightColor::ALL {
                    let center = pos2(sx + 7.0, cy);
                    let resp = ui.interact(
                        Rect::from_center_size(center, vec2(18.0, 18.0)),
                        ui.id().with(("note-swatch", c.name())),
                        Sense::click(),
                    );
                    let on = colour == Some(c);
                    if on {
                        ui.painter().circle_filled(center, 7.0 + 3.5, p.ink);
                        ui.painter().circle_filled(center, 7.0 + 2.0, p.surface);
                    } else if resp.hovered() {
                        ui.painter().circle_filled(center, 7.0 + 3.0, p.fill_strong);
                    }
                    ui.painter().circle_filled(center, 7.0, p.swatch(c));
                    ui.painter().circle_stroke(center, 6.75, Stroke::new(0.5, Color32::from_black_alpha(46)));
                    if on {
                        let s = Stroke::new(1.5, Color32::from_rgb(0x1C, 0x1D, 0x21));
                        let k = 12.0 / 20.0;
                        let o = center - vec2(6.0, 6.0);
                        ui.painter().add(egui::Shape::line(
                            vec![
                                o + vec2(4.75, 10.5) * k,
                                o + vec2(8.25, 14.0) * k,
                                o + vec2(15.25, 6.0) * k,
                            ],
                            s,
                        ));
                    }
                    if resp.on_hover_text(c.name()).clicked() {
                        actions.push(Action::Highlight(if on { None } else { Some(c) }));
                    }
                    sx += 14.0 + 6.0;
                }
                y += 16.0 + 12.0;

                // The verse, quoted beside a rule in its highlight colour.
                let qh = quote.size().y;
                let rule = colour.map_or(p.accent, |c| p.swatch(c));
                ui.painter().rect_filled(Rect::from_min_size(pos2(x, y), vec2(2.0, qh)), 1.0, rule);
                ui.painter().galley(pos2(x + 2.0 + 10.0, y), quote, p.ink_2);
                y += qh + 12.0;

                // The editor.
                let editor = Rect::from_min_size(pos2(x, y), vec2(inner_w, editor_h));
                let id = egui::Id::new(("note-editor", self.verse.key()));
                if ui.memory(|m| m.has_focus(id)) {
                    ui.painter().rect_filled(editor.expand(3.0), 13.0, p.focus);
                }
                ui.painter().rect_filled(editor, 10.0, p.field);
                let focused = ui.memory(|m| m.has_focus(id));
                ui.painter().rect_stroke(
                    editor,
                    10.0,
                    Stroke::new(1.0, if focused { p.accent } else { p.border }),
                    egui::StrokeKind::Inside,
                );
                let inner = editor.shrink2(vec2(12.0, 10.0));
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
                child.visuals_mut().text_cursor.stroke = Stroke::new(1.5, p.accent);
                child.visuals_mut().weak_text_color = Some(p.ink_3);
                // Inter 14/20, wrapped to the editor.
                let ink = p.ink;
                let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, width: f32| {
                    let mut job = LayoutJob::single_section(
                        buf.as_str().to_string(),
                        TextFormat {
                            font_id: FontId::new(14.0, theme::family(theme::UI)),
                            color: ink,
                            line_height: Some(20.0),
                            ..Default::default()
                        },
                    );
                    job.wrap.max_width = width;
                    ui.fonts_mut(|f| f.layout_job(job))
                };
                let resp = child.add_sized(
                    inner.size(),
                    egui::TextEdit::multiline(&mut self.note)
                        .id(id)
                        .hint_text("Write a note…")
                        .font(FontId::new(14.0, theme::family(theme::UI)))
                        .text_color(p.ink)
                        .frame(egui::Frame::NONE)
                        .margin(egui::Margin::ZERO)
                        .layouter(&mut layouter),
                );
                if std::mem::take(&mut self.focus) {
                    resp.request_focus();
                }
                if resp.changed() {
                    self.edited_at = now;
                }
                y += editor_h + 12.0;

                // Footer: delete, saved state, Done.
                let cy = y + 14.0;
                let trash = Rect::from_center_size(pos2(x + 14.0, cy), vec2(28.0, 28.0));
                let tr = ui.interact(trash, ui.id().with("note-delete"), Sense::click());
                if let Some(fill) = crate::widgets::hover_fill(ui, &tr, p, false) {
                    ui.painter().circle_filled(trash.center(), 14.0, fill);
                }
                icons::heavy(|| {
                    icons::trash(
                        ui.painter(),
                        Rect::from_center_size(trash.center(), vec2(16.0, 16.0)),
                        p.danger,
                    )
                });
                if tr.on_hover_text("Delete note").clicked() {
                    actions.push(Action::RemoveNote(self.verse));
                    close = true;
                }
                let status = if self.note != self.saved {
                    "Editing…".to_string()
                } else if self.saved.is_empty() {
                    String::new()
                } else {
                    match self.saved_at.map(|t| bible_core::study::now() - t) {
                        Some(ago) if ago < 60 => "Saved · just now".into(),
                        Some(ago) if ago < 3600 => format!("Saved · {} min ago", ago / 60),
                        Some(_) => format!("Saved · {}", crate::time::ago(self.saved_at.unwrap_or(0))),
                        None => "Saved".into(),
                    }
                };
                ui.painter().text(
                    pos2(trash.right() + 8.0, cy),
                    Align2::LEFT_CENTER,
                    status,
                    text::SMALL.font(),
                    p.ink_3,
                );
                let font = theme::ui_semibold(text::SMALL.size);
                let w = ui.painter().layout_no_wrap("Done".into(), font.clone(), p.on_accent).size().x + 20.0;
                let done = Rect::from_min_size(pos2(rect.right() - PAD - w, cy - 12.0), vec2(w, 24.0));
                let dr = ui.interact(done, ui.id().with("note-done"), Sense::click());
                ui.painter().rect_filled(done, 12.0, p.accent);
                if dr.hovered() {
                    ui.painter().rect_filled(
                        done,
                        12.0,
                        Color32::from_black_alpha(if dr.is_pointer_button_down_on() { 36 } else { 18 }),
                    );
                }
                ui.painter().text(done.center(), Align2::CENTER_CENTER, "Done", font, p.on_accent);
                if dr.on_hover_text(crate::keymap::shown("Done (⌘↩)").as_ref()).clicked() {
                    close = true;
                }
            });
        // Autosave a moment after typing stops.
        if self.note != self.saved {
            if now - self.edited_at > 1.2 {
                self.save(study, actions);
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(300));
            }
        }
        if area.response.clicked_elsewhere() {
            close = true;
        }
        if close {
            self.save(study, actions);
        }
        close
    }
}

/// Where a popover's beak points when its verse has no marker on screen yet.
pub fn fallback_anchor(screen: Rect) -> Rect {
    Rect::from_center_size(Pos2::new(screen.center().x + 200.0, screen.top() + 200.0), vec2(20.0, 20.0))
}
