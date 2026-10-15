//! Right-hand panel for the selected verse: its note, or the verse compared across every
//! installed translation.

use bible_core::VerseRef;
use bible_core::library::{Chapter, Library};
use bible_core::model::Style;
use bible_core::study::StudyStore;
use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{self, FontId, ScrollArea, Ui, vec2};

use crate::actions::Action;
use crate::app::Settings;
use crate::theme;
use crate::widgets;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PanelMode {
    Note,
    Compare,
}

pub struct VersePanel {
    pub verse: VerseRef,
    pub mode: PanelMode,
    note: String,
    saved_note: String,
    focus: bool,
    edited_at: f64,
}

impl VersePanel {
    pub fn new(verse: VerseRef, mode: PanelMode, study: &StudyStore) -> Self {
        let note = study.note(verse).map(|n| n.body.clone()).unwrap_or_default();
        Self { verse, mode, saved_note: note.clone(), note, focus: mode == PanelMode::Note, edited_at: 0.0 }
    }

    fn save(&mut self, study: &mut StudyStore, actions: &mut Vec<Action>) {
        if self.note != self.saved_note {
            match study.set_note(self.verse, &self.note) {
                Ok(()) => self.saved_note = self.note.clone(),
                Err(e) => actions.push(Action::Toast(format!("Couldn't save note: {e:#}"))),
            }
        }
    }

    /// Returns `true` to close the panel.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        library: &Library,
        study: &mut StudyStore,
        settings: &Settings,
        actions: &mut Vec<Action>,
    ) -> bool {
        let p = settings.theme.palette();
        let mut close = false;
        egui::Frame::NONE.inner_margin(egui::Margin::symmetric(14, 12)).show(ui, |ui| {
            ui.horizontal(|ui| {
                let name = library
                    .get(&settings.translation)
                    .map(|e| e.meta.ref_name(self.verse.book).to_string())
                    .unwrap_or_else(|| self.verse.book.ref_name().into());
                ui.label(
                    egui::RichText::new(format!("{name} {}:{}", self.verse.chapter, self.verse.verse))
                        .font(theme::ui_medium(15.0))
                        .color(p.text),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::icon_button(ui, "Close", false, crate::icons::close).clicked() {
                        close = true;
                    }
                });
            });
            ui.add_space(6.0);
            let idx = if self.mode == PanelMode::Note { 0 } else { 1 };
            if let Some(i) = widgets::segmented(ui, &["Note", "Compare"], idx) {
                self.save(study, actions);
                self.mode = if i == 0 { PanelMode::Note } else { PanelMode::Compare };
                self.focus = self.mode == PanelMode::Note;
            }
            ui.add_space(10.0);
            match self.mode {
                PanelMode::Note => self.note_ui(ui, library, study, settings, actions),
                PanelMode::Compare => compare(ui, library, settings, self.verse, actions),
            }
        });
        if close {
            self.save(study, actions);
        }
        close
    }

    fn note_ui(
        &mut self,
        ui: &mut Ui,
        library: &Library,
        study: &mut StudyStore,
        settings: &Settings,
        actions: &mut Vec<Action>,
    ) {
        let p = settings.theme.palette();
        if let Ok(t) = library.load(&settings.translation)
            && let Some(v) = t.find(self.verse)
        {
            let mut job = LayoutJob::single_section(
                v.plain_text(),
                TextFormat {
                    font_id: FontId::new(14.5, theme::family(theme::SERIF_ITALIC)),
                    color: p.muted,
                    ..Default::default()
                },
            );
            job.wrap = TextWrapping {
                max_width: ui.available_width(),
                max_rows: 6,
                overflow_character: Some('…'),
                ..Default::default()
            };
            ui.label(job);
            ui.add_space(8.0);
        }
        let edit = egui::TextEdit::multiline(&mut self.note)
            .hint_text("Write a note on this verse…")
            .font(FontId::new(14.5, theme::family(theme::SERIF)))
            .desired_width(f32::INFINITY)
            .desired_rows(10)
            .margin(vec2(10.0, 8.0));
        let resp = ui.add(edit);
        if self.focus {
            resp.request_focus();
            self.focus = false;
        }
        let now = ui.input(|i| i.time);
        if resp.changed() {
            self.edited_at = now;
        }
        if resp.lost_focus() {
            self.save(study, actions);
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let dirty = self.note != self.saved_note;
            if ui.add_enabled(dirty, egui::Button::new("Save")).clicked() {
                self.save(study, actions);
            }
            if !self.saved_note.is_empty() && ui.button("Delete note").clicked() {
                self.note.clear();
                self.save(study, actions);
            }
            let status = if dirty {
                "Unsaved"
            } else if self.saved_note.is_empty() {
                ""
            } else {
                "Saved"
            };
            ui.label(egui::RichText::new(status).color(p.muted).font(theme::ui_font(11.5)));
        });
        // Autosave a moment after typing stops.
        if self.note != self.saved_note {
            if now - self.edited_at > 1.2 {
                self.save(study, actions);
            } else {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
            }
        }
    }
}

/// One verse in every installed translation.
fn compare(ui: &mut Ui, library: &Library, settings: &Settings, verse: VerseRef, actions: &mut Vec<Action>) {
    let p = settings.theme.palette();
    let width = ui.available_width();
    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        for e in library.entries() {
            let m = &e.meta;
            let (text, styles): (String, Vec<(std::ops::Range<usize>, bool)>) =
                match library.chapter(&m.id, verse.chapter_idx()) {
                    Chapter::Ready(t) => match t.find(verse) {
                        Some(v) => {
                            let text = v.plain_text();
                            // Red-letter ranges, mapped onto the plain text (marks only ever add spaces).
                            let red: Vec<_> = if settings.typo.red_letters {
                                v.spans
                                    .iter()
                                    .filter(|s| s.style == Style::WordsOfJesus)
                                    .map(|s| (shift(&v, s.start as usize)..shift(&v, s.end as usize), true))
                                    .collect()
                            } else {
                                Vec::new()
                            };
                            (text, red)
                        }
                        None => ("— (not in this translation)".into(), Vec::new()),
                    },
                    Chapter::Pending => ("Loading…".into(), Vec::new()),
                    Chapter::NoKey => ("Add an API key in Settings → Translations.".into(), Vec::new()),
                    Chapter::Failed(err) => (format!("Couldn't load: {err}"), Vec::new()),
                    Chapter::Missing => ("— (not in this translation)".into(), Vec::new()),
                };
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&m.abbrev).font(theme::ui_medium(12.0)).color(p.accent));
                if m.id == settings.translation {
                    ui.label(egui::RichText::new("reading").font(theme::ui_font(10.5)).color(p.faint));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("Read").font(theme::ui_font(11.5)))
                                .frame(false),
                        )
                        .clicked()
                    {
                        actions.push(Action::SetTranslation(m.id.clone()));
                    }
                });
            });
            let base = TextFormat {
                font_id: FontId::new(15.0, theme::family(theme::SERIF)),
                color: p.text,
                ..Default::default()
            };
            let mut job = LayoutJob::default();
            let mut pos = 0;
            for (r, _) in &styles {
                if r.start > pos && r.start <= text.len() {
                    job.append(&text[pos..r.start], 0.0, base.clone());
                }
                let end = r.end.min(text.len());
                if r.start < end && r.start >= pos {
                    let mut red = base.clone();
                    red.color = p.red_letter;
                    job.append(&text[r.start..end], 0.0, red);
                    pos = end;
                }
            }
            if pos < text.len() {
                job.append(&text[pos..], 0.0, base);
            }
            job.wrap.max_width = width - 4.0;
            ui.label(job);
            let copyright = e.copyright();
            if !e.is_local() && !copyright.is_empty() {
                let mut job = LayoutJob::single_section(
                    copyright,
                    TextFormat {
                        font_id: FontId::new(10.5, theme::family(theme::UI_MEDIUM)),
                        color: p.faint,
                        ..Default::default()
                    },
                );
                job.wrap = TextWrapping {
                    max_width: width - 4.0,
                    max_rows: 3,
                    overflow_character: Some('…'),
                    ..Default::default()
                };
                ui.label(job);
            }
            ui.add_space(10.0);
            let r = ui.available_rect_before_wrap();
            ui.painter().line_segment([r.left_top(), r.right_top()], egui::Stroke::new(1.0, p.border));
            ui.add_space(10.0);
        }
    });
}

/// Maps a verse-text byte offset onto `plain_text()` (which inserts a space at some breaks).
fn shift(v: &bible_core::model::VerseView<'_>, off: usize) -> usize {
    let mut extra = 0;
    let mut last = 0usize;
    for m in v.marks.iter().filter(|m| m.kind.breaks_line() && m.offset > 0) {
        let at = m.offset as usize;
        if at > last && at < v.text.len() {
            if at <= off && !v.text[..at].ends_with(' ') {
                extra += 1;
            }
            last = at;
        }
    }
    off + extra
}
