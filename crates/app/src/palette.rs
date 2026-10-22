//! ⌘K: jump to a reference ("jn 3:16", "Rom 8", "1co13"), return to a recent place, search,
//! switch translation or run a command, all from the keyboard (`ds-11` in the design).

use bible_core::library::Library;
use bible_core::reference;
use bible_core::study::{ReadingSession, StudyStore};
use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{
    self, Align, Align2, Color32, FontId, Key, Modifiers, Order, Painter, Rect, ScrollArea, Sense, Stroke,
    Ui, pos2, vec2,
};

use crate::actions::{Action, ExportKind, SettingsTab};
use crate::app::Settings;
use crate::icons;
use crate::theme::{self, Palette, Theme, text};
use crate::widgets;

const WIDTH: f32 = 600.0;
const TOP: f32 = 96.0;
const QUERY_H: f32 = 56.0;
const FOOTER_H: f32 = 40.0;

#[derive(Default)]
pub struct CommandPalette {
    query: String,
    selected: usize,
    opened: bool,
    /// Recent reading, loaded when the palette opens.
    recent: Option<Vec<ReadingSession>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    GoTo,
    Recent,
    Translations,
    Commands,
}

impl Section {
    fn title(self) -> &'static str {
        match self {
            Section::GoTo => "Go to",
            Section::Recent => "Recent",
            Section::Translations => "Translations",
            Section::Commands => "Commands",
        }
    }
}

type IconFn = fn(&Painter, Rect, Color32);

struct Item {
    section: Section,
    icon: IconFn,
    title: String,
    /// Characters of the title that match the query (drawn in the accent).
    matched: Vec<bool>,
    /// A second line: a verse preview (serif) or a detail.
    subtitle: Option<(String, bool)>,
    trailing: Option<String>,
    keys: Vec<&'static str>,
    action: Action,
    /// What ⌘↵ does (open in parallel).
    alternate: Option<Action>,
}

impl Item {
    fn height(&self) -> f32 {
        if self.subtitle.is_some() { 48.0 } else { 40.0 }
    }
}

fn command_list(settings: &Settings) -> Vec<(&'static str, IconFn, Vec<&'static str>, Action)> {
    vec![
        ("Toggle parallel translations", icons::columns, vec!["⇧", "⌘", "P"], Action::ToggleParallel),
        ("Theme: Light", sun_icon, vec![], Action::SetTheme(Theme::Light)),
        ("Theme: Sepia", sun_icon, vec![], Action::SetTheme(Theme::Sepia)),
        ("Theme: Dark", sun_icon, vec![], Action::SetTheme(Theme::Dark)),
        (
            if settings.typo.mode == crate::reader::layout::Mode::Paragraph {
                "Layout: verse per line"
            } else {
                "Layout: paragraphs"
            },
            icons::list,
            vec!["⌘", "L"],
            Action::ToggleMode,
        ),
        (
            if settings.typo.red_letters { "Red letters: off" } else { "Red letters: on" },
            icons::text_size,
            vec!["⌘", "R"],
            Action::ToggleRedLetters,
        ),
        ("Bigger text", icons::text_size, vec!["⌘", "+"], Action::FontSize(1.0)),
        ("Smaller text", icons::text_size, vec!["⌘", "−"], Action::FontSize(-1.0)),
        ("Toggle sidebar", icons::sidebar, vec!["⌘", "B"], Action::ToggleSidebar),
        ("Export notes as Markdown…", icons::note, vec!["⌘", "E"], Action::Export(ExportKind::Markdown)),
        ("Export notes as JSON…", icons::note, vec!["⇧", "⌘", "E"], Action::Export(ExportKind::Json)),
        ("Import a translation…", icons::book, vec!["⌘", "I"], Action::ImportTranslation),
        (
            "Manage translations & API keys",
            icons::book,
            vec![],
            Action::OpenSettings(SettingsTab::Translations),
        ),
        ("Settings", icons::sidebar, vec!["⌘", ","], Action::OpenSettings(SettingsTab::Reading)),
        ("Keyboard shortcuts", icons::list, vec!["⌘", "/"], Action::ShowShortcuts),
    ]
}

fn sun_icon(p: &Painter, r: Rect, c: Color32) {
    icons::text_size(p, r, c);
}

/// Which characters of `title` match `query`: the query's letters in order through the book name
/// (fuzzy, as "jn" in John), and the numbers after it when the query has any.
fn matches(title: &str, query: &str) -> Vec<bool> {
    let letters: Vec<char> =
        query.chars().filter(|c| c.is_alphabetic()).flat_map(char::to_lowercase).collect();
    let has_numbers = query.chars().any(|c| c.is_ascii_digit());
    let chars: Vec<char> = title.chars().collect();
    // The book name ends where the chapter number begins (after a space, past any leading "1 ").
    let name_end = chars
        .iter()
        .enumerate()
        .skip(1)
        .find(|(i, c)| c.is_ascii_digit() && chars[i - 1] == ' ')
        .map_or(chars.len(), |(i, _)| i - 1);
    let mut out = vec![false; chars.len()];
    let mut next = letters.iter().peekable();
    for (i, c) in chars.iter().enumerate().take(name_end) {
        if let Some(want) = next.peek()
            && c.to_lowercase().next() == Some(**want)
        {
            out[i] = true;
            next.next();
        }
    }
    if has_numbers {
        for m in out.iter_mut().skip(name_end) {
            *m = true;
        }
    }
    out
}

impl CommandPalette {
    pub fn with_query(query: &str) -> Self {
        Self { query: query.to_string(), ..Self::default() }
    }

    fn items(&self, library: &Library, settings: &Settings) -> Vec<Item> {
        let q = self.query.trim();
        let mut items = Vec::new();
        let entry = library.get(&settings.translation);
        let meta = entry.map(|e| e.meta.clone());
        let abbrev = meta.as_ref().map(|m| m.abbrev.clone()).unwrap_or_default();
        let name = |b: bible_core::BookId| {
            meta.as_ref().map(|m| m.ref_name(b).to_string()).unwrap_or_else(|| b.ref_name().into())
        };
        let translation = library.load(&settings.translation).ok();
        let mut first_ref = None;
        if !q.is_empty() {
            // Only references that exist in the translation being read (Jonah has no 3:16).
            let refs: Vec<_> = reference::suggest(q, 8)
                .into_iter()
                .filter(|r| match (&translation, r.verse) {
                    (Some(t), Some(_)) => t.find(r.verse_ref()).is_some(),
                    (Some(t), None) => t.verses_in(r.chapter_idx()).next().is_some(),
                    (None, _) => true,
                })
                .collect();
            first_ref = refs.first().copied();
            for r in refs.iter().take(3) {
                let title = match r.verse {
                    Some(v) => match r.verse_end.filter(|&e| e > v) {
                        Some(e) => format!("{} {}:{v}–{e}", name(r.book), r.chapter),
                        None => format!("{} {}:{v}", name(r.book), r.chapter),
                    },
                    None => format!("{} {}", name(r.book), r.chapter),
                };
                let (icon, subtitle): (IconFn, _) = match r.verse {
                    Some(_) => (
                        icons::book,
                        translation
                            .as_ref()
                            .and_then(|t| t.find(r.verse_ref()))
                            .map(|v| (v.plain_text(), true)),
                    ),
                    None => (icons::list, chapter_detail(translation.as_deref(), r.chapter_idx())),
                };
                let jump =
                    Action::Jump { chapter: r.chapter_idx(), verse: r.verse, flash: r.verse.is_some() };
                items.push(Item {
                    section: Section::GoTo,
                    icon,
                    matched: matches(&title, q),
                    title,
                    subtitle,
                    trailing: Some(abbrev.clone()),
                    keys: Vec::new(),
                    alternate: Some(Action::OpenInParallel(Box::new(jump.clone()))),
                    action: jump,
                });
            }
            // The first reference's chapter, when it named a verse.
            if let Some(r) = first_ref.filter(|r| r.verse.is_some()) {
                let title = format!("{} {}", name(r.book), r.chapter);
                let jump = Action::Jump { chapter: r.chapter_idx(), verse: None, flash: false };
                items.push(Item {
                    section: Section::GoTo,
                    icon: icons::list,
                    matched: matches(&title, q),
                    title,
                    subtitle: chapter_detail(translation.as_deref(), r.chapter_idx()),
                    trailing: Some(abbrev.clone()),
                    keys: Vec::new(),
                    alternate: Some(Action::OpenInParallel(Box::new(jump.clone()))),
                    action: jump,
                });
            }
        }

        // Recent reading: where each recent chapter was left.
        let mut seen = Vec::new();
        let limit = if q.is_empty() { 6 } else { 2 };
        for s in self.recent.iter().flatten() {
            let Some(c) = s.chapter_idx() else { continue };
            if seen.contains(&c) {
                continue;
            }
            seen.push(c);
            if seen.len() > limit {
                break;
            }
            let t_abbrev = library
                .get(&s.translation)
                .map(|e| e.meta.abbrev.clone())
                .unwrap_or_else(|| s.translation.clone());
            let (title, detail, verse) = if s.finished() || s.furthest <= 1 {
                (format!("{} {}", name(c.book()), c.chapter()), "finished".to_string(), None)
            } else {
                (
                    format!("{} {}:{}", name(c.book()), c.chapter(), s.furthest),
                    format!("stopped at verse {}", s.furthest),
                    Some(s.furthest),
                )
            };
            let detail = if s.furthest <= 1 && !s.finished() { "started".to_string() } else { detail };
            let jump = Action::Jump { chapter: c, verse, flash: false };
            items.push(Item {
                section: Section::Recent,
                icon: icons::history,
                matched: Vec::new(),
                title,
                subtitle: Some((format!("{t_abbrev} · {detail}"), false)),
                trailing: Some(crate::time::ago(s.started)),
                keys: Vec::new(),
                alternate: Some(Action::OpenInParallel(Box::new(jump.clone()))),
                action: jump,
            });
        }

        if !q.is_empty() {
            let ql = q.to_lowercase();
            for e in library.entries() {
                let m = &e.meta;
                if m.id != settings.translation
                    && (m.abbrev.to_lowercase().contains(&ql) || m.name.to_lowercase().contains(&ql))
                {
                    items.push(Item {
                        section: Section::Translations,
                        icon: icons::book,
                        matched: Vec::new(),
                        title: format!("Read {}", m.abbrev),
                        subtitle: Some((m.name.clone(), false)),
                        trailing: None,
                        keys: Vec::new(),
                        action: Action::SetTranslation(m.id.clone()),
                        alternate: Some(Action::AddParallel(m.id.clone())),
                    });
                }
            }
            items.push(command(
                icons::search,
                format!("Search for “{q}”"),
                vec!["⌘", "F"],
                Action::OpenSearch(Some(q.to_string())),
            ));
            if let Some(r) = first_ref {
                if let Some(v) = r.verse {
                    let title = format!("Compare {} {}:{v} across translations", name(r.book), r.chapter);
                    items.push(command(
                        icons::compare,
                        title,
                        vec!["C"],
                        Action::CompareVerse(r.verse_ref()),
                    ));
                }
                let title = format!("Open {} {} in parallel", name(r.book), r.chapter);
                let jump =
                    Action::Jump { chapter: r.chapter_idx(), verse: r.verse, flash: r.verse.is_some() };
                items.push(command(
                    icons::columns,
                    title,
                    vec!["⇧", "⌘", "P"],
                    Action::OpenInParallel(Box::new(jump)),
                ));
            }
        }
        let ql = q.to_lowercase();
        for (title, icon, keys, action) in command_list(settings) {
            if q.is_empty() || title.to_lowercase().contains(&ql) {
                items.push(command(icon, title.to_string(), keys, action));
            }
        }
        items
    }

    /// Returns `true` when the palette should close.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        library: &Library,
        study: &StudyStore,
        settings: &Settings,
        actions: &mut Vec<Action>,
    ) -> bool {
        let p = settings.theme.palette();
        if self.recent.is_none() {
            self.recent = Some(study.reading_since(bible_core::study::now() - 30 * 86_400));
        }
        let items = self.items(library, settings);
        if self.selected >= items.len() {
            self.selected = items.len().saturating_sub(1);
        }
        let (down, up, enter, alt_enter, esc) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::ArrowDown),
                i.consume_key(Modifiers::NONE, Key::ArrowUp),
                i.consume_key(Modifiers::NONE, Key::Enter),
                i.consume_key(Modifiers::COMMAND, Key::Enter),
                i.consume_key(Modifiers::NONE, Key::Escape),
            )
        });
        if esc {
            return true;
        }
        let mut scroll_to_selected = false;
        if down && !items.is_empty() {
            self.selected = (self.selected + 1) % items.len();
            scroll_to_selected = true;
        }
        if up && !items.is_empty() {
            self.selected = (self.selected + items.len() - 1) % items.len();
            scroll_to_selected = true;
        }
        if enter || alt_enter {
            if let Some(it) = items.get(self.selected) {
                let a = if alt_enter {
                    it.alternate.clone().unwrap_or(it.action.clone())
                } else {
                    it.action.clone()
                };
                actions.push(a);
            }
            return true;
        }
        let screen = ctx.content_rect();
        let mut close = false;
        // Dim the window; clicking outside closes.
        egui::Area::new(egui::Id::new("palette-backdrop")).order(Order::Middle).fixed_pos(screen.min).show(
            ctx,
            |ui| {
                let (r, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter().rect_filled(r, 0.0, p.scrim);
                if resp.clicked() {
                    close = true;
                }
            },
        );
        let width = WIDTH.min(screen.width() - 40.0);
        let left = (screen.center().x - width / 2.0).round();
        let max_results = (screen.height() - TOP - QUERY_H - 1.0 - FOOTER_H - 48.0).max(120.0);
        egui::Area::new(egui::Id::new("palette"))
            .order(Order::Foreground)
            .fixed_pos(pos2(left, screen.top() + TOP))
            .show(ctx, |ui| {
                ui.set_width(width);
                let content_h: f32 = 4.0 + 8.0 + rows_height(&items);
                let results_h = content_h.min(max_results);
                let total = egui::Rect::from_min_size(
                    pos2(left, screen.top() + TOP),
                    vec2(width, QUERY_H + 1.0 + results_h + FOOTER_H),
                );
                let radius = theme::radius::LG;
                p.shadows.modal.paint(ui.painter(), total, radius);
                ui.painter().rect_filled(total, radius, p.surface);
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

                // Query: magnifier, the text, an esc keycap.
                let (row, _) = ui.allocate_exact_size(vec2(width, QUERY_H), Sense::hover());
                let glass =
                    Rect::from_center_size(pos2(row.left() + 16.0 + 10.0, row.center().y), vec2(20.0, 20.0));
                icons::search(ui.painter(), glass, p.ink_2);
                let esc_w = widgets::keycap_width(ui.painter(), "esc");
                widgets::keycap(ui.painter(), p, row.right() - 16.0 - esc_w, row.center().y, "esc");
                let edit_rect = Rect::from_min_max(
                    pos2(glass.right() + 12.0, row.top()),
                    pos2(row.right() - 16.0 - esc_w - 12.0, row.bottom()),
                );
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(edit_rect));
                child.visuals_mut().text_cursor.stroke = Stroke::new(1.5, p.accent);
                let font = FontId::new(17.0, theme::family(theme::UI));
                let resp = child.add_sized(
                    edit_rect.size(),
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text("Go to a reference, search, or run a command")
                        .font(font)
                        .text_color(p.ink)
                        .frame(egui::Frame::NONE)
                        .margin(egui::Margin::ZERO)
                        .vertical_align(Align::Center),
                );
                if !self.opened {
                    resp.request_focus();
                    self.opened = true;
                }
                if resp.changed() {
                    self.selected = 0;
                }
                let (div, _) = ui.allocate_exact_size(vec2(width, 1.0), Sense::hover());
                ui.painter().rect_filled(div, 0.0, p.separator);

                // Results.
                let mut sa = ScrollArea::vertical()
                    .max_height(results_h)
                    .auto_shrink([false, true])
                    .id_salt("palette-results");
                if scroll_to_selected {
                    let y = 4.0 + rows_height(&items[..self.selected]);
                    let h = items.get(self.selected).map_or(40.0, Item::height);
                    let cur =
                        ui.ctx().data(|d| d.get_temp::<f32>(egui::Id::new("palette-offset"))).unwrap_or(0.0);
                    if y < cur {
                        sa = sa.vertical_scroll_offset(y - 28.0);
                    } else if y + h > cur + results_h {
                        sa = sa.vertical_scroll_offset(y + h + 8.0 - results_h);
                    }
                }
                let out = sa.show(ui, |ui| {
                    ui.set_width(width);
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    ui.add_space(4.0);
                    let mut section = None;
                    for (i, it) in items.iter().enumerate() {
                        if section != Some(it.section) {
                            section = Some(it.section);
                            let (h, _) = ui.allocate_exact_size(vec2(width, 24.0), Sense::hover());
                            ui.painter().text(
                                pos2(h.left() + 8.0 + 10.0, h.top() + 4.0 + 10.0),
                                Align2::LEFT_CENTER,
                                it.section.title(),
                                text::CAPTION.font(),
                                p.ink_3,
                            );
                            ui.add_space(2.0);
                        }
                        let (outer, resp) = ui.allocate_exact_size(vec2(width, it.height()), Sense::click());
                        let r = outer.shrink2(vec2(8.0, 0.0));
                        paint_row(ui, p, r, it, i == self.selected, resp.hovered());
                        if resp.clicked() {
                            actions.push(it.action.clone());
                            close = true;
                        }
                        ui.add_space(2.0);
                    }
                    ui.add_space(6.0);
                });
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("palette-offset"), out.state.offset.y));

                // Footer: key hints.
                let (foot, _) = ui.allocate_exact_size(vec2(width, FOOTER_H), Sense::hover());
                let painter = ui.painter().with_clip_rect(total);
                painter.rect_filled(
                    foot,
                    egui::CornerRadius { nw: 0, ne: 0, sw: radius as u8, se: radius as u8 },
                    p.fill,
                );
                painter.hline(foot.x_range(), foot.top() + 0.5, Stroke::new(1.0, p.separator));
                let mut x = foot.left() + 16.0;
                for (key, label) in [("↑↓", "Move"), ("↵", "Open"), ("⌘↵", "Open in parallel")] {
                    let k = widgets::keycap(&painter, p, x, foot.center().y, key);
                    let g = painter.layout_no_wrap(label.into(), text::SMALL.font(), p.ink_3);
                    let w = g.size().x;
                    painter.galley(pos2(k.right() + 6.0, foot.center().y - g.size().y / 2.0), g, p.ink_3);
                    x = k.right() + 6.0 + w + 16.0;
                }
                painter.text(
                    pos2(foot.right() - 16.0, foot.center().y),
                    Align2::RIGHT_CENTER,
                    "Type a reference, book or command",
                    text::SMALL.font(),
                    p.ink_3,
                );
            });
        close
    }
}

fn command(icon: IconFn, title: String, keys: Vec<&'static str>, action: Action) -> Item {
    Item {
        section: Section::Commands,
        icon,
        matched: Vec::new(),
        title,
        subtitle: None,
        trailing: None,
        keys,
        action,
        alternate: None,
    }
}

fn chapter_detail(t: Option<&bible_core::Translation>, c: bible_core::ChapterIdx) -> Option<(String, bool)> {
    let n = t?.verses_in(c).map(|v| v.number_end.max(v.number)).max()?;
    Some((format!("Chapter · {n} verses"), false))
}

/// Total height of rows and their section headers.
fn rows_height(items: &[Item]) -> f32 {
    let mut h = 0.0;
    let mut section = None;
    for it in items {
        if section != Some(it.section) {
            section = Some(it.section);
            h += 24.0 + 2.0;
        }
        h += it.height() + 2.0;
    }
    h
}

/// One result (`ds-11`): a 28 pt icon tile, the title (fuzzy matches in the accent) over a detail
/// or verse preview, then a translation or time, keycaps, and ↵ on the keyboard selection.
fn paint_row(ui: &Ui, p: &Palette, r: Rect, it: &Item, selected: bool, hovered: bool) {
    let painter = ui.painter();
    if selected {
        painter.rect_filled(r, 10.0, p.accent_soft);
    } else if hovered {
        painter.rect_filled(r, 10.0, p.fill);
    }
    let tile = Rect::from_min_size(pos2(r.left() + 12.0, r.center().y - 14.0), vec2(28.0, 28.0));
    painter.rect_filled(tile, 8.0, if selected { p.raised } else { p.fill });
    icons::heavy(|| {
        (it.icon)(
            painter,
            Rect::from_center_size(tile.center(), vec2(16.0, 16.0)),
            if selected { p.accent } else { p.ink_2 },
        )
    });

    // Trailing items, right to left.
    let mut right = r.right() - 12.0;
    let cy = r.center().y;
    if selected && it.keys.is_empty() {
        let w = widgets::keycap_width(painter, "↵");
        widgets::keycap(painter, p, right - w, cy, "↵");
        right -= w + 8.0;
    }
    for key in crate::keymap::shown_caps(&it.keys).iter().rev() {
        let w = widgets::keycap_width(painter, key);
        widgets::keycap(painter, p, right - w, cy, key);
        right -= w + 4.0;
    }
    if let Some(t) = &it.trailing {
        let g = painter.layout_no_wrap(t.clone(), theme::ui_medium(11.0), p.ink_3);
        let w = g.size().x;
        painter.galley(pos2(right - w, cy - g.size().y / 2.0), g, p.ink_3);
        right -= w + 8.0;
    }

    let x = tile.right() + 12.0;
    let width = (right - x - 8.0).max(40.0);
    let title_y = if it.subtitle.is_some() { cy - 17.0 } else { cy - 8.0 };
    painter.galley(pos2(x, title_y), title_job(painter, p, it, width), p.ink);
    if let Some((sub, serif)) = &it.subtitle {
        let font = if *serif { FontId::new(13.0, theme::family(theme::SERIF)) } else { text::UI.font() };
        let mut job = LayoutJob::single_section(
            sub.replace('\n', " "),
            TextFormat { font_id: font, color: p.ink_2, line_height: Some(16.0), ..Default::default() },
        );
        job.wrap = TextWrapping {
            max_width: width,
            max_rows: 1,
            break_anywhere: false,
            overflow_character: Some('…'),
        };
        painter.galley(pos2(x, title_y + 18.0), painter.layout_job(job), p.ink_2);
    }
}

fn title_job(painter: &Painter, p: &Palette, it: &Item, width: f32) -> std::sync::Arc<egui::Galley> {
    let strong = TextFormat {
        font_id: text::UI_STRONG.font(),
        color: p.ink,
        line_height: Some(16.0),
        ..Default::default()
    };
    let mut job = LayoutJob::default();
    if it.matched.iter().any(|m| *m) {
        let hit = TextFormat { color: p.accent, ..strong.clone() };
        let rest = TextFormat { font_id: theme::ui_medium(13.0), ..strong };
        let mut buf = String::new();
        let mut cur: Option<bool> = None;
        for (c, m) in it.title.chars().zip(it.matched.iter().copied().chain(std::iter::repeat(false))) {
            if cur.is_some_and(|x| x != m) {
                job.append(
                    &std::mem::take(&mut buf),
                    0.0,
                    if cur == Some(true) { hit.clone() } else { rest.clone() },
                );
            }
            cur = Some(m);
            buf.push(c);
        }
        job.append(&buf, 0.0, if cur == Some(true) { hit } else { rest });
    } else {
        job.append(&it.title, 0.0, strong);
    }
    job.wrap =
        TextWrapping {
            max_width: width, max_rows: 1, break_anywhere: false, overflow_character: Some('…')
        };
    painter.layout_job(job)
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn fuzzy_titles() {
        let m = |t: &str, q: &str| -> String {
            t.chars().zip(matches(t, q)).map(|(c, m)| if m { c } else { '.' }).collect()
        };
        assert_eq!(m("John 3:16", "jn 3:16"), "J..n 3:16");
        assert_eq!(m("1 John 3:16", "jn 3:16"), "..J..n 3:16");
        assert_eq!(m("Joshua 3:16", "jn 3:16"), "J..... 3:16");
        assert_eq!(m("John 3", "jn 3:16"), "J..n 3");
    }
}
