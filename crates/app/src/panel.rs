//! The inspector for the selected verse: the verse compared across every installed translation
//! (frame 07 in the design).

use bible_core::VerseRef;
use bible_core::library::{Chapter, Library};
use bible_core::model::Style;
use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{self, Align2, FontId, Rect, ScrollArea, Sense, Stroke, Ui, pos2, vec2};

use crate::actions::Action;
use crate::app::Settings;
use crate::icons;
use crate::theme::{self, Palette, text};
use crate::widgets;

/// The inspector's width (a 336 pt panel, 8 pt from the window's edge).
pub const INSPECTOR_WIDTH: f32 = 344.0;

pub struct VersePanel {
    pub verse: VerseRef,
    /// The translation picked in the compare list (the one "Add to parallel" adds).
    picked: Option<String>,
}

/// "King James Version" → "King James": the list shows names without their generic last word.
fn short_name(name: &str) -> &str {
    for suffix in [" Version", " Bible", " Translation"] {
        if let Some(s) = name.strip_suffix(suffix) {
            return s;
        }
    }
    name
}

impl VersePanel {
    pub fn new(verse: VerseRef) -> Self {
        Self { verse, picked: None }
    }

    /// Draws the inspector's content in `ui` (its panel already painted). `columns` are the
    /// translations shown side by side. Returns `true` to close.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        library: &Library,
        settings: &Settings,
        columns: &[String],
        actions: &mut Vec<Action>,
    ) -> bool {
        let p = settings.theme.palette();
        let rect = ui.max_rect();
        let name = library
            .get(&settings.translation)
            .map(|e| e.meta.ref_name(self.verse.book).to_string())
            .unwrap_or_else(|| self.verse.book.ref_name().into());
        let reference = format!("{name} {}:{}", self.verse.chapter, self.verse.verse);
        let mut close = false;

        // Header: title and detail, copy and close.
        let header = Rect::from_min_size(rect.min, vec2(rect.width(), 66.0));
        let (title, detail) = ("Compare", format!("{reference} · {} translations", library.entries().len()));
        ui.painter().text(
            pos2(header.left() + 16.0, header.top() + 16.0 + 10.0),
            Align2::LEFT_CENTER,
            title,
            text::UI_TITLE.font(),
            p.ink,
        );
        ui.painter().text(
            pos2(header.left() + 16.0, header.top() + 16.0 + 20.0 + 2.0 + 8.0),
            Align2::LEFT_CENTER,
            detail,
            text::SMALL.font(),
            p.ink_2,
        );
        let b = |x: f32| Rect::from_center_size(pos2(x, header.top() + 16.0 + 17.0), vec2(28.0, 28.0));
        let close_rect = b(header.right() - 12.0 - 14.0);
        if round_button(ui, p, close_rect, "inspector-close", icons::close, "Close").clicked() {
            close = true;
        }
        if round_button(
            ui,
            p,
            b(close_rect.left() - 4.0 - 14.0),
            "inspector-copy",
            icons::copy,
            "Copy every translation",
        )
        .clicked()
        {
            ui.ctx().copy_text(self.compare_text(library, &reference));
            actions.push(Action::Toast(format!("Copied {reference} in every translation")));
        }
        ui.painter().hline(header.x_range(), header.bottom() - 0.5, Stroke::new(1.0, p.separator));

        let body =
            Rect::from_min_max(pos2(rect.left(), header.bottom()), pos2(rect.right(), rect.bottom() - 48.0));
        let mut child = ui.new_child(
            egui::UiBuilder::new().max_rect(body).layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_clip_rect(body.intersect(ui.clip_rect()));
        self.compare(&mut child, library, settings, columns, p);

        // Footer: add the picked translation to the parallel columns.
        let footer = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 48.0), rect.max);
        ui.painter().hline(footer.x_range(), footer.top() + 0.5, Stroke::new(1.0, p.separator));
        {
            let picked = self.picked.clone().unwrap_or_else(|| settings.translation.clone());
            // Adds the picked translation, or offers those not shown yet when it already is.
            let others: Vec<(String, String)> = library
                .entries()
                .iter()
                .filter(|e| !columns.contains(&e.meta.id))
                .map(|e| (e.meta.id.clone(), format!("{}  {}", e.meta.abbrev, e.meta.name)))
                .collect();
            let direct = !columns.contains(&picked) && library.contains(&picked);
            let enabled = direct || !others.is_empty();
            let label = "Add to parallel";
            let font = theme::ui_medium(text::SMALL.size);
            let w = 10.0
                + 16.0
                + 6.0
                + ui.painter().layout_no_wrap(label.into(), font.clone(), p.ink).size().x
                + 10.0;
            let button =
                Rect::from_min_size(pos2(footer.left() + 16.0, footer.center().y - 12.0), vec2(w, 24.0));
            let resp = ui.interact(
                button,
                ui.id().with("add-parallel"),
                if enabled { Sense::click() } else { Sense::hover() },
            );
            let fade = if enabled { 1.0 } else { 0.4 };
            p.shadows.control.paint(ui.painter(), button, 12.0);
            ui.painter().rect_filled(button, 12.0, p.raised);
            if enabled && let Some(fill) = widgets::hover_fill(ui, &resp, p, false) {
                ui.painter().rect_filled(button, 12.0, fill);
            }
            let ink = p.ink.gamma_multiply(fade);
            icons::heavy(|| {
                icons::plus(
                    ui.painter(),
                    Rect::from_center_size(
                        pos2(button.left() + 10.0 + 8.0, button.center().y),
                        vec2(16.0, 16.0),
                    ),
                    ink,
                )
            });
            ui.painter().text(
                pos2(button.left() + 10.0 + 16.0 + 6.0, button.center().y),
                Align2::LEFT_CENTER,
                label,
                font,
                ink,
            );
            if direct && resp.clicked() {
                actions.push(Action::AddParallel(picked));
            } else if enabled {
                crate::menu::popup(&resp, p).show(|ui| {
                    for (id, label) in &others {
                        if ui.button(label).clicked() {
                            actions.push(Action::AddParallel(id.clone()));
                            ui.close();
                        }
                    }
                });
            }
            ui.painter().text(
                pos2(footer.right() - 16.0, footer.center().y),
                Align2::RIGHT_CENTER,
                "C",
                theme::ui_medium(11.0),
                p.ink_3,
            );
        }
        close
    }

    /// Every translation's text of the verse, for the clipboard.
    fn compare_text(&self, library: &Library, reference: &str) -> String {
        let mut out = format!("{reference}\n");
        for e in library.entries() {
            if let Chapter::Ready(t) = library.chapter(&e.meta.id, self.verse.chapter_idx())
                && let Some(v) = t.find(self.verse)
            {
                out.push_str(&format!("\n{}: {}", e.meta.abbrev, v.plain_text()));
            }
        }
        out
    }

    /// One verse in every installed translation: the reading one first is picked, those shown in
    /// parallel are badged.
    fn compare(
        &mut self,
        ui: &mut Ui,
        library: &Library,
        settings: &Settings,
        columns: &[String],
        p: &Palette,
    ) {
        let picked = self.picked.clone().unwrap_or_else(|| settings.translation.clone());
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            egui::Frame::NONE.inner_margin(egui::Margin::same(8)).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                let width = ui.available_width();
                for e in library.entries() {
                    let m = &e.meta;
                    let (body, red) = verse_text(library, &m.id, self.verse, settings.typo.red_letters);
                    let selected = m.id == picked;
                    let font = FontId::new(text::SNIPPET_SM.size, theme::family(theme::SERIF));
                    let mut job = LayoutJob::default();
                    let mut pos = 0;
                    let base = TextFormat {
                        font_id: font,
                        color: p.ink,
                        line_height: Some(20.0),
                        ..Default::default()
                    };
                    for r in red {
                        if r.start >= pos && r.end <= body.len() {
                            job.append(&body[pos..r.start], 0.0, base.clone());
                            job.append(
                                &body[r.start..r.end],
                                0.0,
                                TextFormat { color: p.rubric, ..base.clone() },
                            );
                            pos = r.end;
                        }
                    }
                    job.append(&body[pos..], 0.0, base);
                    job.wrap = TextWrapping {
                        max_width: width - 16.0,
                        max_rows: 4,
                        break_anywhere: false,
                        overflow_character: Some('…'),
                    };
                    let galley = ui.painter().layout_job(job);
                    let h = 10.0 + 16.0 + 4.0 + galley.size().y + 10.0;
                    let (r, resp) = ui.allocate_exact_size(vec2(width, h), Sense::click());
                    if selected {
                        ui.painter().rect_filled(r, 10.0, p.accent_soft);
                    } else if let Some(fill) = widgets::hover_fill(ui, &resp, p, false) {
                        ui.painter().rect_filled(r, 10.0, fill);
                    }
                    let cy = r.top() + 10.0 + 8.0;
                    let abbrev = ui.painter().text(
                        pos2(r.left() + 8.0, cy),
                        Align2::LEFT_CENTER,
                        &m.abbrev,
                        theme::ui_semibold(text::SMALL.size),
                        if selected { p.accent } else { p.ink },
                    );
                    ui.painter().text(
                        pos2(abbrev.right() + 6.0, cy),
                        Align2::LEFT_CENTER,
                        short_name(&m.name),
                        theme::ui_font(11.0),
                        p.ink_3,
                    );
                    if columns.contains(&m.id) {
                        badge(ui, p, pos2(r.right() - 8.0, cy), "In parallel");
                    }
                    ui.painter().galley(pos2(r.left() + 8.0, r.top() + 10.0 + 16.0 + 4.0), galley, p.ink);
                    if resp.clicked() {
                        self.picked = Some(m.id.clone());
                    }
                    if resp.double_clicked() {
                        self.picked = Some(m.id.clone());
                    }
                }
            });
        });
    }
}

/// A 28 pt round icon button with a 16 pt glyph in secondary ink.
fn round_button(
    ui: &mut Ui,
    p: &Palette,
    r: Rect,
    id: &str,
    icon: fn(&egui::Painter, Rect, egui::Color32),
    tip: &str,
) -> egui::Response {
    let resp = ui.interact(r, ui.id().with(id), Sense::click());
    if let Some(fill) = widgets::hover_fill(ui, &resp, p, false) {
        ui.painter().circle_filled(r.center(), 14.0, fill);
    }
    icons::heavy(|| icon(ui.painter(), Rect::from_center_size(r.center(), vec2(16.0, 16.0)), p.ink_2));
    resp.on_hover_text(tip)
}

/// An accent badge (`ds-15`): 20 pt high, radius 10, 11 pt semibold, right-aligned at `right`.
fn badge(ui: &Ui, p: &Palette, right_center: egui::Pos2, label: &str) {
    let g = ui.painter().layout_no_wrap(label.into(), theme::ui_semibold(11.0), p.accent);
    let r = Rect::from_min_size(
        pos2(right_center.x - g.size().x - 14.0, right_center.y - 10.0),
        vec2(g.size().x + 14.0, 20.0),
    );
    ui.painter().rect_filled(r, 10.0, p.accent_soft);
    let size = g.size();
    ui.painter().galley(pos2(r.left() + 7.0, r.center().y - size.y / 2.0), g, p.accent);
}

/// A verse's text in one translation, with its red-letter ranges.
fn verse_text(
    library: &Library,
    id: &str,
    verse: VerseRef,
    red_letters: bool,
) -> (String, Vec<std::ops::Range<usize>>) {
    match library.chapter(id, verse.chapter_idx()) {
        Chapter::Ready(t) => match t.find(verse) {
            Some(v) => {
                let text = v.plain_text();
                // Red-letter ranges, mapped onto the plain text (marks only ever add spaces).
                let red = if red_letters {
                    v.spans
                        .iter()
                        .filter(|s| s.style == Style::WordsOfJesus)
                        .map(|s| shift(&v, s.start as usize)..shift(&v, s.end as usize))
                        .filter(|r| r.end <= text.len())
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
    }
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
