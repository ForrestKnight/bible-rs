//! Left sidebar: book/chapter picker, instant search and study lists.

use std::sync::Arc;
use std::time::Duration;

use bible_core::books::{BookId, ChapterIdx, FIRST_NT_BOOK, VerseRef};
use bible_core::library::Library;
use bible_core::search::{Query, Scope};
use bible_core::study::StudyStore;
use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{self, Align, CornerRadius, FontId, Key, Rect, Response, ScrollArea, Sense, Ui, vec2};

use crate::actions::{Action, ExportKind, SidebarTab};
use crate::theme::{self, Theme};
use crate::widgets;

#[derive(Clone, Copy, PartialEq, Eq)]
enum StudyTab {
    Bookmarks,
    Highlights,
    Notes,
}

pub struct Sidebar {
    expanded: Option<BookId>,
    query: String,
    focus: bool,
    scope: Scope,
    results: Arc<Vec<u32>>,
    key: Option<(String, String, Scope, bool)>,
    elapsed: Duration,
    selected: usize,
    scroll_to_selected: bool,
    study_tab: StudyTab,
}

impl Sidebar {
    pub fn new(_tab: SidebarTab) -> Self {
        Self {
            expanded: None,
            query: String::new(),
            focus: false,
            scope: Scope::All,
            results: Arc::new(Vec::new()),
            key: None,
            elapsed: Duration::ZERO,
            selected: 0,
            scroll_to_selected: false,
            study_tab: StudyTab::Bookmarks,
        }
    }

    pub fn focus_search(&mut self, q: Option<String>) {
        self.focus = true;
        if let Some(q) = q {
            self.query = q;
        }
    }

    pub fn invalidate_search(&mut self) {
        self.key = None;
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut Ui,
        tab: &mut SidebarTab,
        library: &Library,
        study: &StudyStore,
        translation: &str,
        whole_words: &mut bool,
        theme: Theme,
        location: (ChapterIdx, u8),
        actions: &mut Vec<Action>,
    ) {
        ui.add_space(10.0);
        egui::Frame::NONE.inner_margin(egui::Margin::symmetric(12, 0)).show(ui, |ui| {
            let idx = match tab {
                SidebarTab::Books => 0,
                SidebarTab::Search => 1,
                SidebarTab::Study => 2,
            };
            if let Some(i) = widgets::segmented(ui, &["Books", "Search", "Study"], idx) {
                *tab = [SidebarTab::Books, SidebarTab::Search, SidebarTab::Study][i];
                if *tab == SidebarTab::Search {
                    self.focus = true;
                }
            }
        });
        ui.add_space(8.0);
        match tab {
            SidebarTab::Books => self.books(ui, library, translation, theme, location, actions),
            SidebarTab::Search => {
                self.search(ui, library, translation, whole_words, theme, location, actions)
            }
            SidebarTab::Study => self.study(ui, library, study, translation, theme, actions),
        }
    }

    fn books(
        &mut self,
        ui: &mut Ui,
        library: &Library,
        translation: &str,
        theme: Theme,
        location: (ChapterIdx, u8),
        actions: &mut Vec<Action>,
    ) {
        let p = theme.palette();
        let meta = library.get(translation).map(|e| e.meta.clone());
        let current_book = location.0.book();
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            egui::Frame::NONE.inner_margin(egui::Margin::symmetric(8, 0)).show(ui, |ui| {
                for (title, range) in
                    [("Old Testament", 0..FIRST_NT_BOOK.0), ("New Testament", FIRST_NT_BOOK.0..66)]
                {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.add_space(6.0);
                        widgets::caption(ui, title);
                    });
                    ui.add_space(2.0);
                    for b in range.map(BookId) {
                        let name = meta
                            .as_ref()
                            .map(|m| m.book_short(b).to_string())
                            .unwrap_or_else(|| b.name().into());
                        let open = self.expanded == Some(b);
                        let (rect, resp) =
                            ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::click());
                        if resp.hovered() || open {
                            ui.painter().rect_filled(rect, CornerRadius::same(6), p.hover);
                        }
                        let color = if b == current_book { p.accent } else { p.text };
                        ui.painter().text(
                            rect.left_center() + vec2(10.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            &name,
                            if b == current_book { theme::ui_medium(13.5) } else { theme::ui_font(13.5) },
                            color,
                        );
                        ui.painter().text(
                            rect.right_center() - vec2(10.0, 0.0),
                            egui::Align2::RIGHT_CENTER,
                            b.chapters().to_string(),
                            theme::ui_font(11.5),
                            p.faint,
                        );
                        if resp.clicked() {
                            if b.chapters() == 1 {
                                actions.push(Action::Jump {
                                    chapter: b.first_chapter(),
                                    verse: None,
                                    flash: false,
                                });
                            } else {
                                self.expanded = if open { None } else { Some(b) };
                            }
                        }
                        if open {
                            chapter_grid(ui, b, location.0, theme, actions);
                            ui.add_space(4.0);
                        }
                    }
                }
                ui.add_space(12.0);
            });
        });
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
        egui::Frame::NONE.inner_margin(egui::Margin::symmetric(12, 0)).show(ui, |ui| {
            let edit = egui::TextEdit::singleline(&mut self.query)
                .hint_text("Search the Bible…  (\"exact phrase\", -exclude)")
                .desired_width(f32::INFINITY)
                .margin(vec2(8.0, 6.0));
            let resp = ui.add(edit);
            if self.focus {
                resp.request_focus();
                self.focus = false;
            }
            if resp.has_focus() {
                let (down, up, enter, esc) = ui.input_mut(|i| {
                    (
                        i.consume_key(egui::Modifiers::NONE, Key::ArrowDown),
                        i.consume_key(egui::Modifiers::NONE, Key::ArrowUp),
                        i.key_pressed(Key::Enter),
                        i.consume_key(egui::Modifiers::NONE, Key::Escape),
                    )
                });
                if down && !self.results.is_empty() {
                    self.selected = (self.selected + 1).min(self.results.len() - 1);
                    self.scroll_to_selected = true;
                }
                if up {
                    self.selected = self.selected.saturating_sub(1);
                    self.scroll_to_selected = true;
                }
                if enter {
                    jump_to = self.results.get(self.selected).copied();
                }
                if esc {
                    resp.surrender_focus();
                }
            }
            if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                jump_to = self.results.get(self.selected).copied();
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let book = location.0.book();
                let label = |s: Scope| match s {
                    Scope::All => "Whole Bible".to_string(),
                    Scope::OldTestament => "Old Testament".into(),
                    Scope::NewTestament => "New Testament".into(),
                    Scope::Book(b) => b.name().into(),
                };
                egui::ComboBox::from_id_salt("scope").selected_text(label(self.scope)).width(150.0).show_ui(
                    ui,
                    |ui| {
                        for s in [Scope::All, Scope::OldTestament, Scope::NewTestament, Scope::Book(book)] {
                            ui.selectable_value(&mut self.scope, s, label(s));
                        }
                        ui.separator();
                        ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                            for b in BookId::all() {
                                ui.selectable_value(&mut self.scope, Scope::Book(b), b.name());
                            }
                        });
                    },
                );
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
        ui.add_space(6.0);
        egui::Frame::NONE.inner_margin(egui::Margin::symmetric(14, 0)).show(ui, |ui| {
            let status = if self.query.trim().is_empty() {
                "Type to search as you go".to_string()
            } else {
                let n = self.results.len();
                format!(
                    "{} {} · {:.2} ms · {}",
                    group(n),
                    if n == 1 { "verse" } else { "verses" },
                    self.elapsed.as_secs_f64() * 1000.0,
                    t.meta.abbrev
                )
            };
            ui.label(egui::RichText::new(status).font(theme::ui_font(11.5)).color(p.muted));
        });
        ui.add_space(4.0);
        let row_h = 66.0;
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
                if i == self.selected || resp.hovered() {
                    ui.painter().rect_filled(rect.shrink2(vec2(6.0, 2.0)), CornerRadius::same(8), p.hover);
                }
                let inner = rect.shrink2(vec2(18.0, 8.0));
                let reference =
                    format!("{} {}:{}", t.meta.ref_name(v.chapter.book()), v.chapter.chapter(), v.number);
                ui.painter().text(
                    inner.left_top(),
                    egui::Align2::LEFT_TOP,
                    reference,
                    theme::ui_medium(12.0),
                    p.accent,
                );
                let text = v.plain_text();
                let job = snippet_job(&text, &query, inner.width(), theme);
                let g = ui.painter().layout_job(job);
                ui.painter().galley(inner.left_top() + vec2(0.0, 18.0), g, p.text);
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

    fn study(
        &mut self,
        ui: &mut Ui,
        library: &Library,
        study: &StudyStore,
        translation: &str,
        theme: Theme,
        actions: &mut Vec<Action>,
    ) {
        let p = theme.palette();
        let t = library.load(translation).ok();
        let (nh, nn, nb) = study.counts();
        egui::Frame::NONE.inner_margin(egui::Margin::symmetric(12, 0)).show(ui, |ui| {
            let labels = [format!("Bookmarks {nb}"), format!("Highlights {nh}"), format!("Notes {nn}")];
            let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
            let idx = match self.study_tab {
                StudyTab::Bookmarks => 0,
                StudyTab::Highlights => 1,
                StudyTab::Notes => 2,
            };
            if let Some(i) = widgets::segmented(ui, &refs, idx) {
                self.study_tab = [StudyTab::Bookmarks, StudyTab::Highlights, StudyTab::Notes][i];
            }
        });
        ui.add_space(6.0);
        let items: Vec<(VerseRef, Option<egui::Color32>, String)> = match self.study_tab {
            StudyTab::Bookmarks => {
                study.bookmarks().into_iter().map(|(v, b)| (v, None, b.label.clone())).collect()
            }
            StudyTab::Highlights => {
                study.highlights().into_iter().map(|(v, c)| (v, Some(p.swatch(c)), String::new())).collect()
            }
            StudyTab::Notes => study.notes().into_iter().map(|(v, n)| (v, None, n.body.clone())).collect(),
        };
        let footer_h = 44.0;
        let list_h = (ui.available_height() - footer_h).max(80.0);
        ScrollArea::vertical().auto_shrink(false).max_height(list_h).id_salt("study-list").show(ui, |ui| {
            if items.is_empty() {
                egui::Frame::NONE.inner_margin(egui::Margin::symmetric(16, 12)).show(ui, |ui| {
                    let hint = match self.study_tab {
                        StudyTab::Bookmarks => "No bookmarks yet. Select a verse and press B (or ⌘D).",
                        StudyTab::Highlights => "No highlights yet. Select a verse and press 1–5.",
                        StudyTab::Notes => "No notes yet. Select a verse and press N.",
                    };
                    ui.label(egui::RichText::new(hint).color(p.muted));
                });
            }
            for (v, swatch, extra) in items {
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 62.0), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(rect.shrink2(vec2(6.0, 2.0)), CornerRadius::same(8), p.hover);
                }
                let inner = rect.shrink2(vec2(18.0, 8.0));
                let mut x = inner.left();
                if let Some(c) = swatch {
                    ui.painter().circle_filled(egui::pos2(x + 5.0, inner.top() + 8.0), 5.0, c);
                    x += 16.0;
                }
                let name = t
                    .as_ref()
                    .map(|t| t.meta.ref_name(v.book).to_string())
                    .unwrap_or_else(|| v.book.ref_name().into());
                ui.painter().text(
                    egui::pos2(x, inner.top()),
                    egui::Align2::LEFT_TOP,
                    format!("{name} {}:{}", v.chapter, v.verse),
                    theme::ui_medium(12.0),
                    p.accent,
                );
                let body = if extra.is_empty() {
                    t.as_ref().and_then(|t| t.find(v)).map(|v| v.plain_text()).unwrap_or_default()
                } else {
                    extra
                };
                let mut job = LayoutJob::single_section(
                    body.replace('\n', " "),
                    TextFormat {
                        font_id: FontId::new(13.0, theme::family(theme::SERIF)),
                        color: p.text,
                        ..Default::default()
                    },
                );
                job.wrap = TextWrapping {
                    max_width: inner.width(),
                    max_rows: 2,
                    break_anywhere: false,
                    overflow_character: Some('…'),
                };
                let g = ui.painter().layout_job(job);
                ui.painter().galley(inner.left_top() + vec2(0.0, 18.0), g, p.text);
                // Remove button on hover.
                let del = Rect::from_center_size(inner.right_top() + vec2(-6.0, 8.0), vec2(18.0, 18.0));
                if resp.hovered() {
                    crate::icons::close(ui.painter(), del, p.muted);
                }
                let pointer_on_del = ui.input(|i| i.pointer.hover_pos()).is_some_and(|pt| del.contains(pt));
                if resp.clicked() {
                    if pointer_on_del {
                        actions.push(match self.study_tab {
                            StudyTab::Bookmarks => Action::RemoveBookmark(v),
                            StudyTab::Highlights => Action::RemoveHighlight(v),
                            StudyTab::Notes => Action::RemoveNote(v),
                        });
                    } else {
                        actions.push(Action::Jump {
                            chapter: v.chapter_idx(),
                            verse: Some(v.verse),
                            flash: true,
                        });
                    }
                }
            }
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            if ui.button("Export Markdown…").clicked() {
                actions.push(Action::Export(ExportKind::Markdown));
            }
            if ui.button("JSON…").clicked() {
                actions.push(Action::Export(ExportKind::Json));
            }
            if ui.button("Import…").on_hover_text("Merge a bible-rs JSON export").clicked() {
                actions.push(Action::ImportStudy);
            }
        });
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
        font_id: FontId::new(13.5, theme::family(theme::SERIF)),
        color: p.text,
        ..Default::default()
    };
    let mut hit = base.clone();
    hit.background = p.search_hit;
    // Merge hits separated only by spaces so "love one another" reads as one highlight.
    let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
    for r in query.match_ranges(text) {
        match ranges.last_mut() {
            Some(last) if text[last.end..r.start].chars().all(|c| c == ' ') => last.end = r.end,
            _ => ranges.push(r),
        }
    }
    // Start the snippet a little before the first hit so it is visible in two lines.
    let start = ranges.first().map_or(0, |r| {
        if r.start > 90 {
            let mut s = r.start - 60;
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

fn chapter_grid(ui: &mut Ui, book: BookId, current: ChapterIdx, theme: Theme, actions: &mut Vec<Action>) {
    let p = theme.palette();
    let n = book.chapters() as usize;
    let cols = ((ui.available_width() - 16.0) / 38.0).floor().max(4.0) as usize;
    let size = vec2(34.0, 28.0);
    for row in 0..n.div_ceil(cols) {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.spacing_mut().item_spacing.x = 4.0;
            for k in 0..cols {
                let ch = row * cols + k + 1;
                if ch > n {
                    break;
                }
                let idx = ChapterIdx::new(book, ch as u8).unwrap();
                let (r, resp) = ui.allocate_exact_size(size, Sense::click());
                let here = idx == current;
                if here {
                    ui.painter().rect_filled(r, CornerRadius::same(6), p.accent);
                } else if resp.hovered() {
                    ui.painter().rect_filled(r, CornerRadius::same(6), p.hover);
                }
                ui.painter().text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    ch.to_string(),
                    theme::ui_font(12.5),
                    if here { p.page } else { p.text },
                );
                if resp.clicked() {
                    actions.push(Action::Jump { chapter: idx, verse: None, flash: false });
                }
            }
        });
        ui.add_space(2.0);
    }
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
                    ui.label(egui::RichText::new(b.name()).font(theme::ui_medium(14.0)));
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
