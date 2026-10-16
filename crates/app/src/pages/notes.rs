//! Study › Notes (frames 16 and 31): the notes in a list beside an editor for the selected one —
//! the verse quoted on its highlight, a format bar, the note as styled Markdown, and the verses
//! it mentions as links.
//!
//! The editor is a text field whose layouter styles Markdown as it's typed: `#` headings, `**bold**`,
//! `*italic*` (in the reading face), `> quotes` and bullets. Markers are hidden except on the
//! caret's line; bullets show as "•" and are stored as "- ". A blank line is a short gap.

use eframe::egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};
use eframe::egui::{self, Align, Color32, FontId, Rect, Sense, Stroke, Ui, pos2, vec2};

use bible_core::VerseRef;

use super::{Empty, Page, PageEnv, text};
use crate::actions::Action;
use crate::icons;
use crate::sidebar::{ListItem, list_item};
use crate::theme::{self, Palette};
use crate::widgets::{self, css_baseline, flow_indented, paint_runs};

const LIST_W: f32 = 336.0;
/// The note text's measure (the design's paragraphs are 560 wide).
const MEASURE: f32 = 560.0;
const BODY: f32 = 15.0;
const LINE: f32 = 24.0;

pub struct State {
    recent: bool,
    selected: Option<VerseRef>,
    editor: Option<Editor>,
    /// The new-note and link-verse popups' reference fields.
    adding: String,
    linking: String,
}

impl Default for State {
    fn default() -> Self {
        Self { recent: true, selected: None, editor: None, adding: String::new(), linking: String::new() }
    }
}

/// The note being edited, as shown (bullets as "•") and as last saved (Markdown).
struct Editor {
    verse: VerseRef,
    text: String,
    saved: String,
    /// When the text last changed (seconds, egui time), to save after a pause.
    changed: Option<f64>,
}

impl Editor {
    fn new(verse: VerseRef, body: &str) -> Self {
        Self { verse, text: to_display(body), saved: body.to_string(), changed: None }
    }
}

fn to_display(md: &str) -> String {
    md.split('\n')
        .map(|l| match l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")) {
            Some(rest) => format!("• {rest}"),
            None => l.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn to_markdown(shown: &str) -> String {
    shown
        .split('\n')
        .map(|l| match l.strip_prefix("• ") {
            Some(rest) => format!("- {rest}"),
            None => l.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The note as plain text for the list: markers dropped, lines joined.
fn plain(md: &str) -> String {
    md.lines()
        .map(|l| l.trim_start_matches(['#', '>', '-', '*', ' ']).replace("**", "").replace('*', ""))
        .filter(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Verse references a note mentions ("Isaiah 9:2", "1 John 2:8"), in order, without repeats.
pub fn mentioned(text: &str) -> Vec<VerseRef> {
    let words: Vec<&str> = text.split(|c: char| c.is_whitespace() || c == '(' || c == ')').collect();
    let mut out: Vec<VerseRef> = Vec::new();
    for (i, w) in words.iter().enumerate() {
        let cv = w.trim_end_matches([',', '.', ';', ':', '—', '–', '!', '?', '”', '"']);
        let Some((c, v)) = cv.split_once(':') else { continue };
        if c.is_empty()
            || !c.chars().all(|ch| ch.is_ascii_digit())
            || !v.starts_with(|ch: char| ch.is_ascii_digit())
        {
            continue;
        }
        // The book: up to three words before ("Song of Songs", "1 John").
        for n in (1..=3usize).rev() {
            if i < n {
                continue;
            }
            let candidate = format!("{} {cv}", words[i - n..i].join(" "));
            if let Some(r) = bible_core::reference::parse(&candidate)
                && r.verse.is_some()
            {
                let v = r.verse_ref();
                if !out.contains(&v) {
                    out.push(v);
                }
                break;
            }
        }
    }
    out
}

impl State {
    /// Draws the page over the main area `area` (list and editor side by side, each scrolling).
    pub fn show(&mut self, ui: &mut Ui, area: Rect, env: &PageEnv<'_>, actions: &mut Vec<Action>) {
        let p = env.settings.theme.palette();
        let t = super::reading_text(env);
        let notes = env.study.notes();
        if notes.is_empty() && self.editor.is_none() {
            let content = Rect::from_min_max(
                pos2(area.left() + super::SIDE, area.top() + env.toolbar + super::TOP),
                pos2(area.right() - super::SIDE, area.bottom()),
            );
            super::title(ui.painter(), p, content.left(), content.top(), "Notes", "Nothing written yet");
            let empty = Empty {
                glyph: icons::note,
                title: "Your notes will live here",
                body: "Select a verse and press N, or right-click it and choose Add Note. Notes stay on this \
                       Mac and export to Markdown.",
                primary: Some("Open John 1"),
                ghost: Some("Import a backup"),
                hints: &[(&["N"], "New note on the selected verse"), (&["⌘", "E"], "Export as Markdown")],
                width: 344.0,
            };
            super::bookmarks::empty_page(ui, p, content, &empty, actions);
            return;
        }

        // Keep the editor on the selected note (the newest when none is).
        let selected = self.selected.or_else(|| {
            let mut n = notes.clone();
            n.sort_by_key(|(_, n)| std::cmp::Reverse(n.updated));
            n.first().map(|(v, _)| *v)
        });
        self.selected = selected;
        if let Some(v) = selected
            && self.editor.as_ref().is_none_or(|e| e.verse != v)
        {
            self.flush(actions);
            let body = env.study.note(v).map(|n| n.body.clone()).unwrap_or_default();
            self.editor = Some(Editor::new(v, &body));
        }

        let list = Rect::from_min_max(area.min, pos2(area.left() + LIST_W, area.bottom()));
        let editor = Rect::from_min_max(pos2(list.right(), area.top()), area.max);
        self.list(ui, p, list, env, t.as_deref(), actions);
        ui.painter().vline(
            list.right() - 0.5,
            (area.top() + env.toolbar)..=area.bottom(),
            Stroke::new(1.0, p.separator),
        );
        self.editor(ui, p, editor, env, t.as_deref(), actions);

        // Save after a pause in typing.
        let now = ui.input(|i| i.time);
        if let Some(e) = &mut self.editor
            && let Some(at) = e.changed
        {
            if now - at > 0.6 {
                self.flush(actions);
            } else {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(650));
            }
        }
    }

    /// Saves the open note if it changed.
    fn flush(&mut self, actions: &mut Vec<Action>) {
        if let Some(e) = &mut self.editor {
            let md = to_markdown(&e.text);
            if md != e.saved {
                actions.push(Action::SaveNote(e.verse, md.clone()));
                e.saved = md;
            }
            e.changed = None;
        }
    }

    fn list(
        &mut self,
        ui: &mut Ui,
        p: &Palette,
        r: Rect,
        env: &PageEnv<'_>,
        t: Option<&bible_core::Translation>,
        actions: &mut Vec<Action>,
    ) {
        let painter = ui.painter().clone();
        let x = r.left() + 32.0;
        let w = LIST_W - 32.0 - 16.0;
        let top = r.top() + env.toolbar + 20.0;
        // Head: the title and a new-note button.
        let g = widgets::line(
            &painter,
            "Notes",
            FontId::new(28.0, theme::family(theme::SERIF_HEADING)),
            p.ink,
            -0.01,
        );
        widgets::paint_on_baseline(&painter, x, css_baseline(top, 36.0, 28.0, true), g, p.ink);
        let plus = Rect::from_min_size(pos2(x + w - 28.0, top + 4.0), vec2(28.0, 28.0));
        let resp = widgets::round_button(ui, p, plus, icons::plus, ui.id().with("new-note"));
        egui::Popup::menu(&resp).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            ui.set_min_width(220.0);
            ui.label(egui::RichText::new("New note on…").color(p.ink_2));
            let field = ui.add(
                egui::TextEdit::singleline(&mut self.adding).hint_text("John 3:16").desired_width(200.0),
            );
            field.request_focus();
            if field.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                && let Some(r) = bible_core::reference::parse(&self.adding).filter(|r| r.verse.is_some())
            {
                self.flush(actions);
                let v = r.verse_ref();
                self.selected = Some(v);
                let body = env.study.note(v).map(|n| n.body.clone()).unwrap_or_default();
                self.editor = Some(Editor::new(v, &body));
                self.adding.clear();
                ui.close();
            }
        });
        // Sub row: count and order.
        let notes = env.study.notes();
        let count = match notes.len() {
            1 => "1 note".to_string(),
            n => format!("{n} notes"),
        };
        text(&painter, x, top + 48.0, 16.0, &count, theme::ui_font(12.0), p.ink_2, false);
        let (_, pick) = widgets::segmented_tight(
            ui,
            p,
            pos2(x + w, top + 40.0 + 16.0),
            ui.id().with("notes-order"),
            &["Recent", "Canonical"],
            usize::from(!self.recent),
        );
        if let Some(i) = pick {
            self.recent = i == 0;
        }

        // The notes, then any new one not yet saved.
        let q = env.query.trim().to_lowercase();
        let verse_text = |v: VerseRef| t.and_then(|t| t.find(v)).map(|v| v.plain_text()).unwrap_or_default();
        let mut items: Vec<(VerseRef, String, i64)> =
            notes.iter().map(|(v, n)| (*v, n.body.clone(), n.updated)).collect();
        if let Some(e) = &self.editor
            && !items.iter().any(|(v, _, _)| *v == e.verse)
        {
            items.push((e.verse, to_markdown(&e.text), bible_core::study::now()));
        }
        if self.recent {
            items.sort_by_key(|(v, _, updated)| (std::cmp::Reverse(*updated), *v));
        } else {
            items.sort_by_key(|(v, _, _)| *v);
        }
        let list = Rect::from_min_max(pos2(x, top + 40.0 + 44.0), pos2(x + w, r.bottom()));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(list));
        child.set_clip_rect(list.intersect(ui.clip_rect()));
        egui::ScrollArea::vertical().id_salt("notes-list").auto_shrink(false).show(&mut child, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            for (v, body, updated) in items {
                let snippet = verse_text(v);
                let note = plain(&body);
                if !q.is_empty()
                    && !v.display().to_lowercase().contains(&q)
                    && !note.to_lowercase().contains(&q)
                    && !snippet.to_lowercase().contains(&q)
                {
                    continue;
                }
                let item = ListItem {
                    reference: v.display(),
                    trailing: crate::time::short_date(updated),
                    bar: env.study.highlight(v).map(|c| p.swatch(c)),
                    note: Some(if note.is_empty() { "New note".into() } else { note }),
                    snippet,
                    selected: self.selected == Some(v),
                };
                if list_item(ui, p, &item).clicked() {
                    self.flush(actions);
                    self.selected = Some(v);
                }
            }
        });
    }

    fn editor(
        &mut self,
        ui: &mut Ui,
        p: &Palette,
        r: Rect,
        env: &PageEnv<'_>,
        t: Option<&bible_core::Translation>,
        actions: &mut Vec<Action>,
    ) {
        let Some(e) = &mut self.editor else { return };
        let v = e.verse;
        let inner = Rect::from_min_max(pos2(r.left() + 40.0, r.top()), pos2(r.right() - 40.0, r.bottom()));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
        child.set_clip_rect(r.intersect(ui.clip_rect()));
        let mut linking = std::mem::take(&mut self.linking);
        let mut remove = false;
        egui::ScrollArea::vertical().id_salt("note-editor").auto_shrink(false).show(&mut child, |ui| {
            let painter = ui.painter().clone();
            let x = inner.left();
            let width = inner.width();
            let top = ui.max_rect().top() + env.toolbar + 20.0;

            // Head: the reference, facts, and actions.
            let g = widgets::line(
                &painter,
                &v.display(),
                FontId::new(28.0, theme::family(theme::SERIF_HEADING)),
                p.ink,
                -0.01,
            );
            widgets::paint_on_baseline(&painter, x, css_baseline(top, 36.0, 28.0, true), g, p.ink);
            let note = env.study.note(v);
            let mut meta = vec![t.map(|t| t.meta.name.clone()).unwrap_or_default()];
            if let Some(n) = note {
                let l = crate::time::local(n.created);
                const MONTHS: [&str; 12] = [
                    "January",
                    "February",
                    "March",
                    "April",
                    "May",
                    "June",
                    "July",
                    "August",
                    "September",
                    "October",
                    "November",
                    "December",
                ];
                meta.push(format!("Created {} {}", l.day, MONTHS[(l.month as usize + 11) % 12]));
                let ago = crate::time::ago(n.updated);
                meta.push(format!("Edited {}", if ago == "Just now" { "just now".into() } else { ago }));
            } else {
                meta.push("New note".into());
            }
            text(&painter, x, top + 38.0, 16.0, &meta.join(" · "), theme::ui_font(12.0), p.ink_3, false);
            let bookmarked = env.study.is_bookmarked(v);
            let buttons: [(widgets::IconFn, &str); 4] = [
                (
                    if bookmarked {
                        |p, r, c| icons::bookmark(p, r, c, true)
                    } else {
                        |p, r, c| icons::bookmark(p, r, c, false)
                    },
                    "bookmark",
                ),
                (icons::share, "share"),
                (icons::trash, "delete"),
                (icons::ellipsis, "more"),
            ];
            for (i, (icon, id)) in buttons.into_iter().enumerate() {
                let b =
                    Rect::from_min_size(pos2(inner.right() - 140.0 + 36.0 * i as f32, top), vec2(32.0, 32.0));
                let resp = widgets::round_button(ui, p, b, icon, ui.id().with(("note-head", id)));
                match id {
                    "bookmark" if resp.clicked() => actions.push(Action::ToggleBookmarkAt(v)),
                    "share" if resp.clicked() => {
                        let verse = t.and_then(|t| t.find(v)).map(|v| v.plain_text()).unwrap_or_default();
                        ui.ctx().copy_text(format!("{} — {verse}\n\n{}", v.display(), to_markdown(&e.text)));
                        actions.push(Action::Toast(format!("Copied the note on {}", v.display())));
                    }
                    "delete" if resp.clicked() => remove = true,
                    "more" => {
                        crate::menu::popup(&resp, p).show(|ui| {
                            ui.set_min_width(180.0);
                            if ui.button("Open in Reader").clicked() {
                                actions.push(Action::Jump {
                                    chapter: v.chapter_idx(),
                                    verse: Some(v.verse),
                                    flash: true,
                                });
                            }
                            if ui.button("Copy as Markdown").clicked() {
                                ui.ctx().copy_text(to_markdown(&e.text));
                            }
                            crate::menu::separator(ui);
                            if ui.button("Delete Note").clicked() {
                                remove = true;
                            }
                        });
                    }
                    _ => {}
                }
            }

            // The verse, quoted on its highlight.
            let y = top + 54.0 + 20.0;
            let colour = env.study.highlight(v);
            let verse = t.and_then(|t| t.find(v)).map(|v| v.plain_text()).unwrap_or_default();
            let serif = FontId::new(20.0, theme::family(theme::SERIF));
            let number = v.verse.to_string();
            let ng = widgets::line(&painter, &number, theme::ui_semibold(11.0), p.verse, 0.0);
            let indent = ng.size().x + 4.0;
            let block = width.min(MEASURE + 40.0) - 19.0;
            let lines = flow_indented(&painter, &verse, &serif, 15.0, block - 4.0, block - indent - 4.0, 6);
            let h = (lines.len().max(1) as f32 * 34.0 - 2.0).max(20.0);
            painter.rect_filled(
                Rect::from_min_size(pos2(x, y), vec2(3.0, h)),
                2.0,
                colour.map_or(p.border, |c| p.swatch(c)),
            );
            let bx = x + 19.0;
            widgets::paint_on_baseline(&painter, bx, css_baseline(y + 4.0, 16.0, 11.0, false), ng, p.verse);
            for (k, l) in lines.iter().enumerate() {
                let lx = if k == 0 { bx + indent } else { bx };
                let ly = y + 34.0 * k as f32;
                if let Some(c) = colour {
                    let lw = widgets::runs_width(&painter, l);
                    painter.rect_filled(
                        Rect::from_min_size(pos2(lx, ly), vec2(lw + 4.0, 32.0)),
                        4.0,
                        p.highlight(c),
                    );
                }
                paint_runs(&painter, lx + 2.0, css_baseline(ly, 32.0, 20.0, true), l, p.ink);
            }

            // Tools: the format bar and the word count.
            let y = y + h + 20.0;
            let id = egui::Id::new(("note-text", v.key()));
            let state = egui::TextEdit::load_state(ui.ctx(), id);
            let caret = state.as_ref().and_then(|s| s.cursor.char_range()).map(|r| r.primary.index.0);
            let active = caret.map(|c| styles_at(&e.text, c)).unwrap_or_default();
            let bar = Rect::from_min_size(pos2(x, y), vec2(193.0, 32.0));
            widgets::capsule(&painter, bar, p);
            let tools: [(Tool, bool); 6] = [
                (Tool::Heading, active.heading),
                (Tool::Bold, active.bold),
                (Tool::Italic, active.italic),
                (Tool::List, active.bullet),
                (Tool::Quote, active.quote),
                (Tool::Link, false),
            ];
            let mut bx = bar.left();
            let mut clicked = None;
            for (i, (tool, on)) in tools.into_iter().enumerate() {
                let b = Rect::from_min_size(pos2(bx, y), vec2(32.0, 32.0));
                let resp = ui.interact(b, ui.id().with(("tool", i)), Sense::click());
                if on {
                    painter.circle_filled(b.center(), 16.0, p.accent_soft);
                } else if let Some(fill) = widgets::hover_fill(ui, &resp, p, false) {
                    painter.circle_filled(b.center(), 16.0, fill);
                }
                let ink = if on { p.accent } else { p.ink };
                let icon = Rect::from_center_size(b.center(), vec2(16.0, 16.0));
                match tool {
                    Tool::Heading => icons::heavy(|| icons::text_size(&painter, icon, ink)),
                    Tool::Bold => {
                        painter.text(b.center(), egui::Align2::CENTER_CENTER, "B", theme::ui_bold(13.0), ink);
                    }
                    Tool::Italic => {
                        let f = FontId::new(14.0, theme::family(theme::SERIF_ITALIC));
                        painter.text(b.center(), egui::Align2::CENTER_CENTER, "I", f, ink);
                    }
                    Tool::List => icons::heavy(|| icons::list(&painter, icon, ink)),
                    Tool::Quote => icons::heavy(|| icons::quote(&painter, icon, ink)),
                    Tool::Link => icons::heavy(|| icons::link(&painter, icon, ink)),
                }
                if resp.clicked() {
                    clicked = Some((tool, resp));
                }
                bx += 32.0;
                if i == 0 {
                    widgets::capsule_divider(&painter, bx, y + 16.0, p);
                    bx += 1.0;
                }
            }
            let words = to_markdown(&e.text)
                .split_whitespace()
                .filter(|w| w.chars().any(char::is_alphanumeric))
                .count();
            let wc = format!("Markdown · {words} {}", if words == 1 { "word" } else { "words" });
            text(&painter, inner.right(), y + 8.0, 16.0, &wc, theme::ui_font(12.0), p.ink_3, true);

            // The note.
            let y = y + 32.0 + 20.0;
            let caret_line = caret.map(|c| e.text.chars().take(c).filter(|ch| *ch == '\n').count());
            let pal = env.settings.theme.palette();
            let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap: f32| {
                let job = markdown_job(ui, buf.as_str(), caret_line, pal, wrap);
                ui.fonts_mut(|f| f.layout_job(job))
            };
            let edit_rect = Rect::from_min_size(pos2(x, y), vec2(width.min(MEASURE), 0.0));
            let resp = ui
                .scope_builder(
                    egui::UiBuilder::new().max_rect(edit_rect.with_max_y(edit_rect.top() + 4000.0)),
                    |ui| {
                        ui.visuals_mut().text_cursor.stroke = Stroke::new(1.5, p.accent);
                        ui.visuals_mut().selection.bg_fill = p.selection;
                        ui.add(
                            egui::TextEdit::multiline(&mut e.text)
                                .id(id)
                                .frame(egui::Frame::NONE)
                                .margin(egui::Margin::ZERO)
                                .desired_width(edit_rect.width())
                                .desired_rows(4)
                                .hint_text(
                                    egui::RichText::new("Write a note…")
                                        .font(theme::ui_font(BODY))
                                        .color(p.ink_3),
                                )
                                .layouter(&mut layouter),
                        )
                    },
                )
                .inner;
            if resp.changed() {
                // "- " or "* " at a line start becomes a bullet (same length: the caret stays put).
                let shown = to_display(&e.text);
                if shown != e.text {
                    e.text = shown;
                }
                e.changed = Some(ui.input(|i| i.time));
            }
            if let Some((tool, button)) = clicked {
                if tool == Tool::Link {
                    linking.clear();
                    egui::Popup::open_id(ui.ctx(), button.id.with("link-popup"));
                } else if let Some(mut s) = state {
                    let range = s
                        .cursor
                        .char_range()
                        .unwrap_or(CCursorRange::one(CCursor::new(e.text.chars().count())));
                    let (text, a, b) = apply(
                        tool,
                        &e.text,
                        range.primary.index.0.min(range.secondary.index.0),
                        range.primary.index.0.max(range.secondary.index.0),
                    );
                    e.text = text;
                    e.changed = Some(ui.input(|i| i.time));
                    s.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(a), CCursor::new(b))));
                    s.store(ui.ctx(), id);
                    ui.memory_mut(|m| m.request_focus(id));
                }
            }

            // Linked verses: what the note mentions, and a way to add one.
            let y = resp.rect.bottom() + 20.0;
            let linked = mentioned(&e.text);
            let mut cx = x;
            let lw = text(&painter, cx, y + 4.0, 16.0, "Linked", theme::ui_semibold(12.0), p.ink_3, false);
            cx += lw + 8.0;
            for r in linked {
                let label = r.display();
                let tw = painter
                    .layout_no_wrap(label.clone(), theme::ui_semibold(12.0), Color32::PLACEHOLDER)
                    .size()
                    .x;
                let chip = Rect::from_min_size(pos2(cx, y), vec2(10.0 + 12.0 + 4.0 + tw + 10.0, 24.0));
                if chip.right() > inner.right() {
                    break;
                }
                let resp = ui.interact(chip, ui.id().with(("linked", r.key())), Sense::click());
                painter.rect_filled(chip, 12.0, if resp.hovered() { p.selection } else { p.accent_soft });
                let icon = Rect::from_min_size(pos2(chip.left() + 10.0, y + 6.0), vec2(12.0, 12.0));
                icons::heavy(|| icons::link(&painter, icon, p.accent));
                text(
                    &painter,
                    icon.right() + 4.0,
                    y + 4.0,
                    16.0,
                    &label,
                    theme::ui_semibold(12.0),
                    p.accent,
                    false,
                );
                if resp.clicked() {
                    actions.push(Action::Jump {
                        chapter: r.chapter_idx(),
                        verse: Some(r.verse),
                        flash: true,
                    });
                }
                cx = chip.right() + 8.0;
            }
            let label = "Link verse";
            let tw =
                painter.layout_no_wrap(label.into(), theme::ui_medium(12.0), Color32::PLACEHOLDER).size().x;
            let chip = Rect::from_min_size(pos2(cx, y), vec2(10.0 + 12.0 + 4.0 + tw + 10.0, 24.0));
            let resp = ui.interact(chip, ui.id().with("link-verse"), Sense::click());
            if resp.hovered() {
                painter.rect_filled(chip, 12.0, p.fill);
            }
            painter.rect_stroke(chip, 12.0, Stroke::new(1.0, p.border), egui::StrokeKind::Inside);
            let icon = Rect::from_min_size(pos2(chip.left() + 10.0, y + 6.0), vec2(12.0, 12.0));
            icons::heavy(|| icons::plus(&painter, icon, p.ink_2));
            text(&painter, icon.right() + 4.0, y + 4.0, 16.0, label, theme::ui_medium(12.0), p.ink_2, false);
            if resp.clicked() {
                linking.clear();
            }
            // Linking inserts the reference at the end of the note, where the chip picks it up.
            egui::Popup::menu(&resp).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(
                |ui| {
                    ui.set_min_width(220.0);
                    ui.label(egui::RichText::new("Link a verse").color(p.ink_2));
                    let field = ui.add(
                        egui::TextEdit::singleline(&mut linking).hint_text("Isaiah 9:2").desired_width(200.0),
                    );
                    field.request_focus();
                    if field.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        && let Some(r) = bible_core::reference::parse(&linking).filter(|r| r.verse.is_some())
                    {
                        let r = r.verse_ref();
                        if !e.text.is_empty() && !e.text.ends_with('\n') {
                            e.text.push('\n');
                        }
                        e.text.push_str(&format!("• {}", r.display()));
                        e.changed = Some(ui.input(|i| i.time));
                        linking.clear();
                        ui.close();
                    }
                },
            );
            ui.allocate_rect(
                Rect::from_min_max(ui.max_rect().min, pos2(inner.right(), y + 24.0 + 40.0)),
                Sense::hover(),
            );
        });
        self.linking = linking;
        if remove {
            actions.push(Action::RemoveNote(v));
            self.editor = None;
            self.selected = None;
        }
    }

    /// Opens a note (demo steps, links from elsewhere).
    pub fn select(&mut self, v: VerseRef) {
        self.selected = Some(v);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Heading,
    Bold,
    Italic,
    List,
    Quote,
    Link,
}

#[derive(Default)]
struct Active {
    heading: bool,
    bold: bool,
    italic: bool,
    bullet: bool,
    quote: bool,
}

/// The styles in effect at character `c` (for the format bar's on states).
fn styles_at(text: &str, c: usize) -> Active {
    let before: String = text.chars().take(c).collect();
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[line_start..].find('\n').map_or(text.len(), |i| line_start + i);
    let line = &text[line_start..line_end];
    let col = before.len() - line_start;
    let mut a = Active {
        heading: line.starts_with("# ") || line.starts_with("## ") || line.starts_with("### "),
        bullet: line.starts_with("• "),
        quote: line.starts_with("> "),
        ..Default::default()
    };
    for s in inline_spans(line) {
        if s.start <= col && col <= s.end {
            a.bold |= s.bold;
            a.italic |= s.italic;
        }
    }
    a
}

/// Applies a format to the selection `a..b` (chars); returns the text and the new selection.
fn apply(tool: Tool, text: &str, a: usize, b: usize) -> (String, usize, usize) {
    let chars: Vec<char> = text.chars().collect();
    let line_start = chars[..a.min(chars.len())].iter().rposition(|c| *c == '\n').map_or(0, |i| i + 1);
    let prefix = |p: &str| -> (String, usize, usize) {
        let line: String = chars[line_start..].iter().take_while(|c| **c != '\n').collect();
        let n = p.chars().count() as isize;
        let (new_line, delta) = if let Some(rest) = line.strip_prefix(p) {
            (rest.to_string(), -n)
        } else {
            // One block style per line: drop another prefix first.
            let bare = ["# ", "• ", "> "]
                .iter()
                .find_map(|q| line.strip_prefix(q).map(|r| (r, q.chars().count() as isize)));
            match bare {
                Some((r, m)) => (format!("{p}{r}"), n - m),
                None => (format!("{p}{line}"), n),
            }
        };
        let end = line_start + line.chars().count();
        let out: String = chars[..line_start].iter().collect::<String>()
            + &new_line
            + &chars[end..].iter().collect::<String>();
        let shift = |x: usize| ((x as isize + delta).max(line_start as isize)) as usize;
        (out, shift(a), shift(b))
    };
    match tool {
        Tool::Heading => prefix("# "),
        Tool::List => prefix("• "),
        Tool::Quote => prefix("> "),
        Tool::Bold | Tool::Italic => {
            let m = if tool == Tool::Bold { "**" } else { "*" };
            let n = m.len();
            let sel: String = chars[a..b].iter().collect();
            let before: String = chars[..a].iter().collect();
            let after: String = chars[b..].iter().collect();
            if before.ends_with(m) && after.starts_with(m) {
                let out = format!("{}{sel}{}", &before[..before.len() - n], &after[n..]);
                (out, a - n, b - n)
            } else {
                (format!("{before}{m}{sel}{m}{after}"), a + n, b + n)
            }
        }
        Tool::Link => (text.to_string(), a, b),
    }
}

/// An inline span within a line (byte offsets of its content, markers excluded).
struct Span {
    start: usize,
    end: usize,
    bold: bool,
    italic: bool,
    /// Byte ranges of its markers.
    markers: [std::ops::Range<usize>; 2],
}

/// `**bold**` and `*italic*` spans in a line; a marker without a partner stays literal.
fn inline_spans(line: &str) -> Vec<Span> {
    let mut out = Vec::new();
    let b = line.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'*' {
            let bold = i + 1 < b.len() && b[i + 1] == b'*';
            let m = if bold { 2 } else { 1 };
            let open = i + m;
            let close = if bold { line[open..].find("**") } else { line[open..].find('*') };
            if let Some(c) = close
                && c > 0
            {
                let end = open + c;
                out.push(Span { start: open, end, bold, italic: !bold, markers: [i..open, end..end + m] });
                i = end + m;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// The layout of the note: Markdown styled, markers hidden except on the caret's line.
fn markdown_job(ui: &egui::Ui, text: &str, caret_line: Option<usize>, p: &Palette, wrap: f32) -> LayoutJob {
    let inter = |size: f32, family: &str, color: Color32, lh: f32| TextFormat {
        font_id: FontId::new(size, theme::family(family)),
        color,
        line_height: Some(lh),
        valign: Align::BOTTOM,
        ..Default::default()
    };
    let body = inter(BODY, theme::UI, p.ink, LINE);
    // Literata italic sits lower than Inter on the same line height; a taller line box lifts it.
    let italic_probe = inter(BODY, theme::SERIF_ITALIC, p.ink, LINE);
    let lift = ui.fonts_mut(|f| {
        crate::reader::layout::baseline(f, &italic_probe) - crate::reader::layout::baseline(f, &body)
    });
    let italic = TextFormat { line_height: Some(LINE + lift), ..italic_probe };
    let hidden = TextFormat {
        font_id: FontId::new(0.5, theme::family(theme::UI)),
        color: Color32::TRANSPARENT,
        line_height: Some(LINE),
        valign: Align::BOTTOM,
        ..Default::default()
    };
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap;
    let lines: Vec<&str> = text.split('\n').collect();
    for (n, line) in lines.iter().enumerate() {
        let last = n + 1 == lines.len();
        let on_caret = caret_line == Some(n);
        let marker = |job: &mut LayoutJob, s: &str, size_like: &TextFormat| {
            if on_caret {
                job.append(s, 0.0, TextFormat { color: p.ink_3, ..size_like.clone() });
            } else {
                job.append(s, 0.0, hidden.clone());
            }
        };
        if line.is_empty() {
            // A blank line: an 8 pt gap.
            let gap = TextFormat { line_height: Some(8.0), ..hidden.clone() };
            job.append(if last { "" } else { "\n" }, 0.0, gap);
            continue;
        }
        let (base, rest) = if let Some(level) = ["### ", "## ", "# "].iter().position(|h| line.starts_with(h))
        {
            let m = ["### ", "## ", "# "][level];
            let f = inter(17.0, theme::UI_SEMIBOLD, p.ink, LINE);
            marker(&mut job, m, &f);
            (f, &line[m.len()..])
        } else if let Some(rest) = line.strip_prefix("• ") {
            job.append("• ", 0.0, TextFormat { color: p.ink_3, ..body.clone() });
            (body.clone(), rest)
        } else if let Some(rest) = line.strip_prefix("> ") {
            let f = TextFormat { color: p.ink_2, ..italic.clone() };
            marker(&mut job, "> ", &body);
            (f, rest)
        } else {
            (body.clone(), *line)
        };
        // Inline spans.
        let mut at = 0;
        for s in inline_spans(rest) {
            if s.markers[0].start > at {
                job.append(&rest[at..s.markers[0].start], 0.0, base.clone());
            }
            marker(&mut job, &rest[s.markers[0].clone()], &base);
            let f = if s.italic {
                TextFormat { color: base.color, ..italic.clone() }
            } else {
                TextFormat {
                    font_id: FontId::new(base.font_id.size, theme::family(theme::UI_BOLD)),
                    ..base.clone()
                }
            };
            job.append(&rest[s.start..s.end], 0.0, f);
            marker(&mut job, &rest[s.markers[1].clone()], &base);
            at = s.markers[1].end;
        }
        if at < rest.len() {
            job.append(&rest[at..], 0.0, base.clone());
        }
        if !last {
            job.append("\n", 0.0, base.clone());
        }
    }
    job
}

impl Page {
    /// Whether the page draws its own panes rather than one scrolling column.
    pub fn has_panes(self) -> bool {
        self == Page::Notes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bullets_round_trip_and_references_are_found() {
        let md = "# Title\n- one\n* two\nplain - not a bullet";
        assert_eq!(to_display(md), "# Title\n• one\n• two\nplain - not a bullet");
        assert_eq!(to_markdown(&to_display(md)), "# Title\n- one\n- two\nplain - not a bullet");
        let refs = mentioned("Compare Isaiah 9:2 and 1 John 2:8, also (John 8:12).");
        let shown: Vec<String> = refs.iter().map(|r| r.display()).collect();
        assert_eq!(shown, ["Isaiah 9:2", "1 John 2:8", "John 8:12"]);
    }

    #[test]
    fn formats_wrap_and_unwrap() {
        let (t, a, b) = apply(Tool::Bold, "say hi", 4, 6);
        assert_eq!((t.as_str(), a, b), ("say **hi**", 6, 8));
        let (t, _, _) = apply(Tool::Bold, &t, 6, 8);
        assert_eq!(t, "say hi");
        let (t, _, _) = apply(Tool::Heading, "a\nb", 2, 2);
        assert_eq!(t, "a\n# b");
        let (t, _, _) = apply(Tool::List, &t, 3, 3);
        assert_eq!(t, "a\n• b");
        assert!(styles_at("x *it* y", 4).italic);
        assert!(!styles_at("x *it* y", 7).italic);
    }
}
