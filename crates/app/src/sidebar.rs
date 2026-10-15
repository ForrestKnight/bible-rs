//! Left sidebar: a floating source list (study collections, then the books with a chapter grid)
//! that turns into search results while a search is active.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use bible_core::books::{BookId, ChapterIdx, FIRST_NT_BOOK, VerseRef};
use bible_core::library::Library;
use bible_core::search::{Query, Scope};
use bible_core::study::StudyStore;
use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{
    self, Align, Align2, Color32, FontId, Key, Painter, Rect, Response, ScrollArea, Sense, Ui, pos2, vec2,
};

use crate::actions::{Action, ExportKind, SidebarTab};
use crate::theme::{self, Palette, Theme, text};
use crate::widgets;

/// Source-list geometry (points): rows inset from the panel's edges, 32 pt rows on 8 pt padding.
const LIST_INSET: f32 = 8.0;
const ROW_HEIGHT: f32 = 32.0;
const ROW_PAD: f32 = 8.0;
const ICON_LANE: f32 = 16.0;
const GAP: f32 = 8.0;
/// Book rows indent their labels to line up with the labels of icon rows.
const BOOK_INDENT: f32 = 32.0;
/// Chapter chips: 24 pt circles, 8 pt apart, seven to a row, indented 24 pt.
const CHIP: f32 = 24.0;
const CHIP_GAP: f32 = 8.0;
const CHIP_INDENT: f32 = 24.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StudyKind {
    Bookmarks,
    Highlights,
    Notes,
}

pub struct Sidebar {
    /// Which testaments are expanded (OT, NT).
    testaments: [bool; 2],
    /// The book whose chapter grid is open.
    expanded: Option<BookId>,
    study_open: Option<StudyKind>,
    pub query: String,
    /// Whether the toolbar's search field has focus (arrow keys then move through results).
    pub field_focused: bool,
    focus: bool,
    scope: Scope,
    results: Arc<Vec<u32>>,
    key: Option<(String, String, Scope, bool)>,
    elapsed: Duration,
    selected: usize,
    scroll_to_selected: bool,
    /// The book last revealed (testament opened, scrolled into view) in the source list.
    revealed: Option<BookId>,
    /// Chapters with highlights or notes, keyed by the study store's generation.
    activity: (u64, HashSet<ChapterIdx>),
}

impl Sidebar {
    pub fn new(_tab: SidebarTab) -> Self {
        Self {
            testaments: [false, false],
            expanded: None,
            study_open: None,
            query: String::new(),
            field_focused: false,
            focus: false,
            scope: Scope::All,
            results: Arc::new(Vec::new()),
            key: None,
            elapsed: Duration::ZERO,
            selected: 0,
            scroll_to_selected: false,
            revealed: None,
            activity: (0, HashSet::new()),
        }
    }

    pub fn focus_search(&mut self, q: Option<String>) {
        self.focus = true;
        if let Some(q) = q {
            self.query = q;
        }
    }

    /// Whether the toolbar's search field should take focus this frame.
    pub fn take_focus(&mut self) -> bool {
        std::mem::take(&mut self.focus)
    }

    /// Expands one of the study collections by name (`bookmarks`, `highlights`, `notes`).
    pub fn open_study(&mut self, name: &str) {
        self.study_open = match name {
            "bookmarks" => Some(StudyKind::Bookmarks),
            "highlights" => Some(StudyKind::Highlights),
            "notes" => Some(StudyKind::Notes),
            _ => None,
        };
    }

    pub fn invalidate_search(&mut self) {
        self.key = None;
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn searching(&self) -> bool {
        !self.query.trim().is_empty()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut Ui,
        library: &Library,
        study: &StudyStore,
        translation: &str,
        whole_words: &mut bool,
        theme: Theme,
        location: (ChapterIdx, u8),
        actions: &mut Vec<Action>,
    ) {
        if self.searching() {
            self.search(ui, library, translation, whole_words, theme, location, actions);
        } else {
            self.source_list(ui, library, study, translation, theme, location, actions);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn source_list(
        &mut self,
        ui: &mut Ui,
        library: &Library,
        study: &StudyStore,
        translation: &str,
        theme: Theme,
        location: (ChapterIdx, u8),
        actions: &mut Vec<Action>,
    ) {
        let p = theme.palette();
        let meta = library.get(translation).map(|e| e.meta.clone());
        let current_book = location.0.book();
        let reveal = self.revealed != Some(current_book);
        if reveal {
            self.revealed = Some(current_book);
            self.testaments[usize::from(current_book >= FIRST_NT_BOOK)] = true;
            self.expanded = Some(current_book);
        }
        if self.activity.0 != study.generation() {
            let mut set = HashSet::new();
            set.extend(study.highlights().into_iter().map(|(v, _)| v.chapter_idx()));
            set.extend(study.notes().into_iter().map(|(v, _)| v.chapter_idx()));
            self.activity = (study.generation(), set);
        }
        let (nh, nn, nb) = study.counts();
        ScrollArea::vertical().auto_shrink(false).id_salt("source-list").show(ui, |ui| {
            egui::Frame::NONE.inner_margin(egui::Margin { left: 8, right: 8, top: 4, bottom: 8 }).show(
                ui,
                |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    section(ui, p, "Study", true);
                    for (kind, label, n, icon) in [
                        (StudyKind::Bookmarks, "Bookmarks", nb, icon_bookmark as IconFn),
                        (StudyKind::Highlights, "Highlights", nh, crate::icons::highlighter),
                        (StudyKind::Notes, "Notes", nn, crate::icons::note),
                    ] {
                        let open = self.study_open == Some(kind);
                        let r = row(
                            ui,
                            p,
                            &Row { icon: Some(icon), label, count: Some(n.to_string()), ..Row::default() },
                        );
                        if r.clicked() {
                            self.study_open = if open { None } else { Some(kind) };
                        }
                        if open {
                            self.study_items(ui, library, study, translation, theme, kind, actions);
                        }
                    }

                    section(ui, p, "Books", false);
                    for (t, (label, range)) in
                        [("Old Testament", 0..FIRST_NT_BOOK.0), ("New Testament", FIRST_NT_BOOK.0..66)]
                            .into_iter()
                            .enumerate()
                    {
                        let open = self.testaments[t];
                        let r = row(
                            ui,
                            p,
                            &Row {
                                icon: Some(crate::icons::book),
                                label,
                                count: Some(range.len().to_string()),
                                disclosure: Some(open),
                                ..Row::default()
                            },
                        );
                        if r.clicked() {
                            self.testaments[t] = !open;
                        }
                        if !open {
                            continue;
                        }
                        for b in range.map(BookId) {
                            let name = meta
                                .as_ref()
                                .map(|m| m.book_short(b).to_string())
                                .unwrap_or_else(|| b.name().into());
                            let here = b == current_book;
                            let resp = row(
                                ui,
                                p,
                                &Row {
                                    label: &name,
                                    indent: BOOK_INDENT - ROW_PAD,
                                    count: here.then(|| b.chapters().to_string()),
                                    selected: here,
                                    ..Row::default()
                                },
                            );
                            if here && reveal {
                                resp.scroll_to_me(Some(Align::Center));
                            }
                            if resp.clicked() {
                                if b.chapters() == 1 {
                                    actions.push(Action::Jump {
                                        chapter: b.first_chapter(),
                                        verse: None,
                                        flash: false,
                                    });
                                } else {
                                    self.expanded = if self.expanded == Some(b) { None } else { Some(b) };
                                }
                            }
                            if self.expanded == Some(b) && b.chapters() > 1 {
                                chapter_grid(ui, p, b, location.0, &self.activity.1, actions);
                            }
                        }
                    }
                },
            );
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn study_items(
        &mut self,
        ui: &mut Ui,
        library: &Library,
        study: &StudyStore,
        translation: &str,
        theme: Theme,
        kind: StudyKind,
        actions: &mut Vec<Action>,
    ) {
        let p = theme.palette();
        let t = library.load(translation).ok();
        let items: Vec<(VerseRef, Option<Color32>, String)> = match kind {
            StudyKind::Bookmarks => {
                study.bookmarks().into_iter().map(|(v, b)| (v, None, b.label.clone())).collect()
            }
            StudyKind::Highlights => {
                study.highlights().into_iter().map(|(v, c)| (v, Some(p.swatch(c)), String::new())).collect()
            }
            StudyKind::Notes => study.notes().into_iter().map(|(v, n)| (v, None, n.body.clone())).collect(),
        };
        if items.is_empty() {
            let hint = match kind {
                StudyKind::Bookmarks => "No bookmarks yet. Select a verse and press B (or ⌘D).",
                StudyKind::Highlights => "No highlights yet. Select a verse and press 1–6.",
                StudyKind::Notes => "No notes yet. Select a verse and press N.",
            };
            egui::Frame::NONE.inner_margin(egui::Margin::symmetric(12, 8)).show(ui, |ui| {
                ui.label(egui::RichText::new(hint).font(text::SMALL.font()).color(p.ink_2));
            });
            return;
        }
        for (v, swatch, extra) in items {
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 72.0), Sense::click());
            if let Some(fill) = widgets::hover_fill(ui, &resp, p, false) {
                ui.painter().rect_filled(rect, 10.0, fill);
            }
            let inner = rect.shrink2(vec2(12.0, 10.0));
            let mut x = inner.left();
            if let Some(c) = swatch {
                let bar = Rect::from_min_size(pos2(x, inner.top()), vec2(3.0, inner.height()));
                ui.painter().rect_filled(bar, 1.5, c);
                x += 12.0;
            }
            let name = t
                .as_ref()
                .map(|t| t.meta.ref_name(v.book).to_string())
                .unwrap_or_else(|| v.book.ref_name().into());
            ui.painter().text(
                pos2(x, inner.top()),
                Align2::LEFT_TOP,
                format!("{name} {}:{}", v.chapter, v.verse),
                text::UI_STRONG.font(),
                p.ink,
            );
            if let Some(t) = &t {
                ui.painter().text(
                    pos2(inner.right(), inner.top() + 2.0),
                    Align2::RIGHT_TOP,
                    &t.meta.abbrev,
                    theme::ui_medium(11.0),
                    p.ink_3,
                );
            }
            let body = if extra.is_empty() {
                t.as_ref().and_then(|t| t.find(v)).map(|v| v.plain_text()).unwrap_or_default()
            } else {
                extra
            };
            let mut job = LayoutJob::single_section(
                body.replace('\n', " "),
                TextFormat {
                    font_id: FontId::new(text::SNIPPET_SM.size, theme::family(theme::SERIF)),
                    color: p.ink_2,
                    line_height: Some(text::SNIPPET_SM.line_height),
                    ..Default::default()
                },
            );
            job.wrap = TextWrapping {
                max_width: inner.right() - x,
                max_rows: 2,
                break_anywhere: false,
                overflow_character: Some('…'),
            };
            let g = ui.painter().layout_job(job);
            ui.painter().galley(pos2(x, inner.top() + 22.0), g, p.ink_2);
            if resp.clicked() {
                actions.push(Action::Jump { chapter: v.chapter_idx(), verse: Some(v.verse), flash: true });
            }
            resp.context_menu(|ui| {
                let (label, a) = match kind {
                    StudyKind::Bookmarks => ("Remove bookmark", Action::RemoveBookmark(v)),
                    StudyKind::Highlights => ("Remove highlight", Action::RemoveHighlight(v)),
                    StudyKind::Notes => ("Delete note", Action::RemoveNote(v)),
                };
                if ui.button(label).clicked() {
                    actions.push(a);
                    ui.close();
                }
                ui.separator();
                if ui.button("Export as Markdown…").clicked() {
                    actions.push(Action::Export(ExportKind::Markdown));
                    ui.close();
                }
                if ui.button("Export as JSON…").clicked() {
                    actions.push(Action::Export(ExportKind::Json));
                    ui.close();
                }
                if ui.button("Import…").clicked() {
                    actions.push(Action::ImportStudy);
                    ui.close();
                }
            });
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn search(
        &mut self,
        ui: &mut Ui,
        library: &Library,
        translation: &str,
        whole_words: &mut bool,
        theme: Theme,
        location: (ChapterIdx, u8),
        actions: &mut Vec<Action>,
    ) {
        let p = theme.palette();
        let Ok(t) = library.load(translation) else {
            ui.label(
                "Search is available for local translations. Online translations are fetched per chapter.",
            );
            return;
        };
        let mut jump_to: Option<u32> = None;
        // The query lives in the toolbar's field; arrow keys and return drive the results.
        let (down, up, enter) = if self.field_focused {
            ui.input_mut(|i| {
                (
                    i.consume_key(egui::Modifiers::NONE, Key::ArrowDown),
                    i.consume_key(egui::Modifiers::NONE, Key::ArrowUp),
                    i.key_pressed(Key::Enter),
                )
            })
        } else {
            (false, false, false)
        };
        if down && !self.results.is_empty() {
            self.selected = (self.selected + 1).min(self.results.len() - 1);
            self.scroll_to_selected = true;
            jump_to = self.results.get(self.selected).copied();
        }
        if up {
            self.selected = self.selected.saturating_sub(1);
            self.scroll_to_selected = true;
            jump_to = self.results.get(self.selected).copied();
        }
        if enter {
            jump_to = self.results.get(self.selected).copied();
        }
        egui::Frame::NONE.inner_margin(egui::Margin::symmetric(12, 0)).show(ui, |ui| {
            let idx = match self.scope {
                Scope::OldTestament => 1,
                Scope::NewTestament => 2,
                _ => 0,
            };
            if let Some(i) = widgets::segmented(ui, p, &["Bible", "Old Testament", "New Testament"], idx) {
                self.scope = [Scope::All, Scope::OldTestament, Scope::NewTestament][i];
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let book = location.0.book();
                let label = |s: Scope| match s {
                    Scope::Book(b) => b.name().to_string(),
                    _ => "All books".into(),
                };
                egui::ComboBox::from_id_salt("scope-book")
                    .selected_text(label(self.scope))
                    .width(150.0)
                    .icon(widgets::combo_chevron)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.scope, Scope::All, "All books");
                        ui.selectable_value(&mut self.scope, Scope::Book(book), book.name());
                        ui.separator();
                        ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                            for b in BookId::all() {
                                ui.selectable_value(&mut self.scope, Scope::Book(b), b.name());
                            }
                        });
                    });
                ui.checkbox(whole_words, "Whole words");
            });
        });

        // Run the search when its inputs change (well under a millisecond on the KJV).
        let key = (self.query.clone(), translation.to_string(), self.scope, *whole_words);
        if self.key.as_ref() != Some(&key) {
            let out = bible_core::search::run(&t, &self.query, self.scope, *whole_words);
            self.results = Arc::new(out.verses);
            self.elapsed = out.elapsed;
            self.key = Some(key);
            self.selected = 0;
        }
        let query = Query::parse(&self.query, *whole_words);
        ui.add_space(8.0);
        egui::Frame::NONE.inner_margin(egui::Margin::symmetric(16, 0)).show(ui, |ui| {
            let n = self.results.len();
            let status = format!(
                "{} {} · {:.2} ms · {}",
                group(n),
                if n == 1 { "verse" } else { "verses" },
                self.elapsed.as_secs_f64() * 1000.0,
                t.meta.abbrev
            );
            ui.label(egui::RichText::new(status).font(text::SMALL.font()).color(p.ink_2));
        });
        ui.add_space(4.0);
        let row_h = 76.0;
        let results = self.results.clone();
        let mut sa = ScrollArea::vertical().auto_shrink(false).id_salt("results");
        if self.scroll_to_selected {
            self.scroll_to_selected = false;
            let avail = ui.available_height();
            let target = self.selected as f32 * row_h;
            let cur = ui.ctx().data(|d| d.get_temp::<f32>(egui::Id::new("results-offset"))).unwrap_or(0.0);
            if target < cur || target + row_h > cur + avail {
                sa = sa.vertical_scroll_offset((target - avail / 2.0).max(0.0));
            }
        }
        let out = sa.show_rows(ui, row_h, results.len(), |ui, range| {
            for i in range {
                let vi = results[i];
                let Some(v) = t.verse_view(vi) else { continue };
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), row_h), Sense::click());
                let item = rect.shrink2(vec2(LIST_INSET, 2.0));
                let selected = i == self.selected;
                if selected {
                    ui.painter().rect_filled(item, 10.0, p.accent_soft);
                } else if let Some(fill) = widgets::hover_fill(ui, &resp, p, false) {
                    ui.painter().rect_filled(item, 10.0, fill);
                }
                let inner = item.shrink2(vec2(12.0, 10.0));
                let reference =
                    format!("{} {}:{}", t.meta.ref_name(v.chapter.book()), v.chapter.chapter(), v.number);
                ui.painter().text(
                    inner.left_top(),
                    Align2::LEFT_TOP,
                    reference,
                    text::UI_STRONG.font(),
                    if selected { p.accent } else { p.ink },
                );
                ui.painter().text(
                    inner.right_top() + vec2(0.0, 2.0),
                    Align2::RIGHT_TOP,
                    &t.meta.abbrev,
                    theme::ui_medium(11.0),
                    p.ink_3,
                );
                let text = v.plain_text();
                let job = snippet_job(&text, &query, inner.width(), theme);
                let g = ui.painter().layout_job(job);
                ui.painter().galley(inner.left_top() + vec2(0.0, 22.0), g, p.ink_2);
                if resp.clicked() {
                    self.selected = i;
                    jump_to = Some(vi);
                }
            }
        });
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("results-offset"), out.state.offset.y));
        if let Some(vi) = jump_to
            && let Some(v) = t.verse_view(vi)
        {
            actions.push(Action::Jump { chapter: v.chapter, verse: Some(v.number), flash: true });
        }
    }
}

type IconFn = fn(&Painter, Rect, Color32);

fn icon_bookmark(p: &Painter, r: Rect, c: Color32) {
    crate::icons::bookmark(p, r, c, false);
}

/// A source-list row.
#[derive(Default)]
struct Row<'a> {
    icon: Option<IconFn>,
    label: &'a str,
    /// Extra space before the label (book rows have no icon but line up with icon rows).
    indent: f32,
    count: Option<String>,
    /// A disclosure chevron: `Some(open)`.
    disclosure: Option<bool>,
    selected: bool,
}

fn row(ui: &mut Ui, p: &Palette, r: &Row<'_>) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    if r.selected {
        ui.painter().rect_filled(rect, theme::radius::SM, p.accent_soft);
    } else if let Some(fill) = widgets::hover_fill(ui, &resp, p, false) {
        ui.painter().rect_filled(rect, theme::radius::SM, fill);
    }
    let painter = ui.painter();
    let cy = rect.center().y;
    let mut x = rect.left() + ROW_PAD + r.indent;
    if let Some(icon) = r.icon {
        icon(
            painter,
            Rect::from_min_size(pos2(x, cy - ICON_LANE / 2.0), vec2(ICON_LANE, ICON_LANE)),
            p.accent,
        );
        x += ICON_LANE + GAP;
    }
    let (font, color) =
        if r.selected { (text::UI_STRONG.font(), p.accent) } else { (text::UI.font(), p.ink) };
    painter.text(pos2(x, cy), Align2::LEFT_CENTER, r.label, font, color);
    let mut right = rect.right() - ROW_PAD;
    if let Some(open) = r.disclosure {
        let lane =
            Rect::from_min_size(pos2(right - ICON_LANE, cy - ICON_LANE / 2.0), vec2(ICON_LANE, ICON_LANE));
        let chevron = Rect::from_center_size(lane.center(), vec2(12.0, 12.0));
        if open {
            crate::icons::chevron_down(painter, chevron, p.ink_3);
        } else {
            crate::icons::disclosure(painter, chevron, p.ink_3);
        }
        right = lane.left() - GAP;
    }
    if let Some(count) = &r.count {
        let (font, color) = if r.selected {
            (theme::ui_medium(text::SMALL.size), p.accent)
        } else {
            (text::SMALL.font(), p.ink_3)
        };
        painter.text(pos2(right, cy), Align2::RIGHT_CENTER, count, font, color);
    }
    resp
}

/// A section header: 11 pt semibold tertiary text, 8 pt below the previous row (16 for later
/// sections).
fn section(ui: &mut Ui, p: &Palette, label: &str, first: bool) {
    let (top, h) = if first { (8.0, 28.0) } else { (16.0, 36.0) };
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    let y = rect.top() + top + (h - top) / 2.0;
    ui.painter().text(
        pos2(rect.left() + ROW_PAD, y),
        Align2::LEFT_CENTER,
        label,
        text::CAPTION.font(),
        p.ink_3,
    );
}

/// The open book's chapters: 24 pt chips, the current one solid accent; a dot marks chapters
/// with highlights or notes.
fn chapter_grid(
    ui: &mut Ui,
    p: &Palette,
    book: BookId,
    current: ChapterIdx,
    activity: &HashSet<ChapterIdx>,
    actions: &mut Vec<Action>,
) {
    let n = book.chapters() as usize;
    let cols = 7;
    let rows = n.div_ceil(cols);
    let h = 8.0 + rows as f32 * CHIP + (rows.saturating_sub(1)) as f32 * 4.0 + 8.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    for i in 0..n {
        let (r, c) = (i / cols, i % cols);
        let chip = Rect::from_min_size(
            pos2(
                rect.left() + CHIP_INDENT + c as f32 * (CHIP + CHIP_GAP),
                rect.top() + 8.0 + r as f32 * (CHIP + 4.0),
            ),
            vec2(CHIP, CHIP),
        );
        let idx = ChapterIdx::new(book, (i + 1) as u8).unwrap();
        let resp = ui.interact(chip, ui.id().with(("chapter", book.0, i)), Sense::click());
        let here = idx == current;
        if here {
            ui.painter().circle_filled(chip.center(), CHIP / 2.0, p.accent);
        } else if let Some(fill) = widgets::hover_fill(ui, &resp, p, false) {
            ui.painter().circle_filled(chip.center(), CHIP / 2.0, fill);
        }
        let (font, color) = if here {
            (theme::ui_semibold(text::SMALL.size), p.on_accent)
        } else {
            (text::SMALL.font(), p.ink_2)
        };
        ui.painter().text(chip.center(), Align2::CENTER_CENTER, (i + 1).to_string(), font, color);
        if !here && activity.contains(&idx) {
            ui.painter().circle_filled(pos2(chip.center().x, chip.bottom() + 1.0), 2.0, p.accent);
        }
        if resp.clicked() {
            actions.push(Action::Jump { chapter: idx, verse: None, flash: false });
        }
    }
}

fn group(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A two-line result snippet with search hits highlighted.
pub fn snippet_job(text: &str, query: &Query, width: f32, theme: Theme) -> LayoutJob {
    let p = theme.palette();
    let base = TextFormat {
        font_id: FontId::new(text::SNIPPET_SM.size, theme::family(theme::SERIF)),
        color: p.ink_2,
        line_height: Some(text::SNIPPET_SM.line_height),
        ..Default::default()
    };
    let mut hit = base.clone();
    hit.background = p.highlights[0];
    hit.color = p.ink;
    // Merge hits separated only by spaces so "love one another" reads as one highlight.
    let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
    for r in query.match_ranges(text) {
        match ranges.last_mut() {
            Some(last) if text[last.end..r.start].chars().all(|c| c == ' ') => last.end = r.end,
            _ => ranges.push(r),
        }
    }
    // Start the snippet a few words before the first hit so it is visible in two lines.
    let start = ranges.first().map_or(0, |r| {
        if r.start > 50 {
            let mut s = r.start - 30;
            while !text.is_char_boundary(s) {
                s -= 1;
            }
            text[..s].rfind(' ').map_or(s, |i| i + 1)
        } else {
            0
        }
    });
    let mut job = LayoutJob::default();
    if start > 0 {
        job.append("…", 0.0, base.clone());
    }
    let mut pos = start;
    for r in ranges.iter().filter(|r| r.start >= start) {
        if r.start > pos {
            job.append(&text[pos..r.start], 0.0, base.clone());
        }
        job.append(&text[r.start..r.end], 0.0, hit.clone());
        pos = r.end;
    }
    if pos < text.len() {
        job.append(&text[pos..], 0.0, base);
    }
    job.wrap =
        TextWrapping {
            max_width: width, max_rows: 2, break_anywhere: false, overflow_character: Some('…')
        };
    job.halign = Align::LEFT;
    job
}

/// Popup under the toolbar's location button: books, then chapters.
pub fn book_picker_popup(
    _ui: &mut Ui,
    anchor: &Response,
    library: &Library,
    translation: &str,
    actions: &mut Vec<Action>,
) {
    let id = egui::Id::new("book-picker-book");
    let meta = library.get(translation).map(|e| e.meta.clone());
    egui::Popup::menu(anchor).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
        let chosen: Option<BookId> = ui.data(|d| d.get_temp(id));
        match chosen {
            None => {
                ui.set_width(560.0);
                ui.columns(2, |cols| {
                    for (ci, (title, range)) in
                        [("Old Testament", 0..FIRST_NT_BOOK.0), ("New Testament", FIRST_NT_BOOK.0..66)]
                            .into_iter()
                            .enumerate()
                    {
                        let ui = &mut cols[ci];
                        widgets::caption(ui, title);
                        egui::Grid::new(title).num_columns(3).spacing(vec2(4.0, 2.0)).show(ui, |ui| {
                            for (k, b) in range.map(BookId).enumerate() {
                                let name = meta
                                    .as_ref()
                                    .map(|m| m.book_short(b).to_string())
                                    .unwrap_or_else(|| b.name().into());
                                let short: String = if name.chars().count() > 12 {
                                    b.info().abbrev.to_string()
                                } else {
                                    name
                                };
                                if ui
                                    .add(egui::Button::new(short).frame(false).min_size(vec2(84.0, 22.0)))
                                    .clicked()
                                {
                                    if b.chapters() == 1 {
                                        actions.push(Action::Jump {
                                            chapter: b.first_chapter(),
                                            verse: None,
                                            flash: false,
                                        });
                                        ui.close();
                                    } else {
                                        ui.data_mut(|d| d.insert_temp(id, b));
                                    }
                                }
                                if k % 3 == 2 {
                                    ui.end_row();
                                }
                            }
                        });
                    }
                });
            }
            Some(b) => {
                ui.set_width(380.0);
                ui.horizontal(|ui| {
                    if ui.button("‹ Books").clicked() {
                        ui.data_mut(|d| d.remove::<BookId>(id));
                    }
                    ui.label(egui::RichText::new(b.name()).font(text::UI_TITLE.font()));
                });
                ui.add_space(4.0);
                let n = b.chapters() as usize;
                let cols = 10;
                egui::Grid::new("chapters").num_columns(cols).spacing(vec2(3.0, 3.0)).show(ui, |ui| {
                    for ch in 1..=n {
                        if ui.add(egui::Button::new(ch.to_string()).min_size(vec2(32.0, 26.0))).clicked() {
                            actions.push(Action::Jump {
                                chapter: ChapterIdx::new(b, ch as u8).unwrap(),
                                verse: None,
                                flash: false,
                            });
                            ui.data_mut(|d| d.remove::<BookId>(id));
                            ui.close();
                        }
                        if ch % cols == 0 {
                            ui.end_row();
                        }
                    }
                });
            }
        }
    });
}
