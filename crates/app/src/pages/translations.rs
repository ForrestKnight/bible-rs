//! Library › Translations (frame 12): every installed translation with filters, a row per
//! translation, and an inspector for the selected one.

use std::collections::HashMap;
use std::sync::Arc;

use eframe::egui::{self, Color32, FontId, Rect, Sense, Ui, pos2, vec2};

use bible_core::library::{Entry, EntryKind};
use bible_core::online::ProviderKind;
use bible_core::{BookId, VerseRef, bundled};

use super::{Page, PageEnv, elide, text};
use crate::actions::{Action, SettingsTab};
use crate::icons;
use crate::theme::{self, Palette};
use crate::widgets::{self, BadgeKind, Button, Kind, css_baseline, flow, paint_runs};

const ROW: f32 = 60.0;
const ROW_GAP: f32 = 2.0;
const INSPECTOR: f32 = 280.0;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Filter {
    #[default]
    All,
    Bundled,
    Imported,
    Online,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Sort {
    /// Bundled, then imported, then online: the library's own order.
    #[default]
    Kind,
    Name,
}

#[derive(Default)]
pub struct State {
    filter: Filter,
    sort: Sort,
    /// The row shown in the inspector (the reading translation until another is picked).
    pub selected: Option<String>,
    /// Decoded sizes, read from each file's header once.
    sizes: HashMap<String, Option<u64>>,
}

fn kind_of(e: &Entry) -> Filter {
    match e.kind {
        EntryKind::Bundled(_) => Filter::Bundled,
        EntryKind::Imported(_) => Filter::Imported,
        EntryKind::Online(_) => Filter::Online,
    }
}

/// "4.4 MB" (binary megabytes, as Finder rounds them).
fn megabytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
}

/// The host of an http(s) URL.
fn host(url: &str) -> Option<&str> {
    let rest = url.split_once("://")?.1;
    Some(rest.split(['/', ' ']).next().unwrap_or(rest))
}

fn online_host(kind: &ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Esv => "api.esv.org",
        ProviderKind::ApiBible { .. } => "api.scripture.api.bible",
    }
}

/// The import's format, from the source line the importer writes ("file.zip (SWORD module)").
fn import_format(source: &str) -> &str {
    source.rsplit_once('(').map(|(_, f)| f.trim_end_matches(')')).unwrap_or("Imported file")
}

fn imported_on(e: &Entry) -> Option<String> {
    let EntryKind::Imported(path) = &e.kind else { return None };
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let secs = modified.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64;
    Some(crate::time::short_date(secs))
}

fn description(e: &Entry) -> String {
    let m = &e.meta;
    match &e.kind {
        EntryKind::Bundled(_) => {
            bundled::description(&m.id).map(str::to_string).unwrap_or_else(|| m.license.clone())
        }
        EntryKind::Imported(_) => match imported_on(e) {
            Some(d) => format!("{} · imported {d}", import_format(&m.source)),
            None => import_format(&m.source).to_string(),
        },
        EntryKind::Online(o) => {
            let key = if o.has_key() { "key in Keychain" } else { "no API key" };
            format!("{} · {key}", online_host(&o.config.provider))
        }
    }
}

fn features(e: &Entry) -> String {
    let m = &e.meta;
    let mut f = Vec::new();
    if m.headings {
        f.push("Section headings");
    }
    if m.red_letters {
        f.push("Red letters");
    }
    if m.paragraphs && f.len() < 2 {
        f.push("Paragraphs");
    }
    if f.is_empty() { "—".into() } else { f.join(", ") }
}

fn language(tag: &str) -> &str {
    match tag.split('-').next().unwrap_or(tag) {
        "en" => "English",
        "de" => "German",
        "fr" => "French",
        "es" => "Spanish",
        "pt" => "Portuguese",
        "it" => "Italian",
        "nl" => "Dutch",
        "la" => "Latin",
        "el" | "grc" => "Greek",
        "he" | "hbo" => "Hebrew",
        _ => tag,
    }
}

impl State {
    fn size(&mut self, e: &Entry) -> Option<u64> {
        *self.sizes.entry(e.meta.id.clone()).or_insert_with(|| match &e.kind {
            EntryKind::Bundled(b) => bible_core::format::decoded_len(b),
            EntryKind::Imported(path) => {
                std::fs::read(path).ok().and_then(|b| bible_core::format::decoded_len(&b))
            }
            EntryKind::Online(_) => None,
        })
    }

    fn size_label(&mut self, e: &Entry, env: &PageEnv<'_>) -> String {
        match &e.kind {
            EntryKind::Online(_) => {
                let n = env.library.cached_verses(&e.meta.id);
                format!("{n} {} cached", if n == 1 { "verse" } else { "verses" })
            }
            _ => self.size(e).map(megabytes).unwrap_or_default(),
        }
    }

    /// Draws the page from `content`'s top-left; returns the bottom of what was drawn.
    pub fn show(&mut self, ui: &mut Ui, content: Rect, env: &PageEnv<'_>, actions: &mut Vec<Action>) -> f32 {
        let p = env.settings.theme.palette();
        let painter = ui.painter().clone();
        let left = content.left();
        let right = content.right();
        let top = content.top();
        let entries: Vec<Arc<Entry>> = env.library.entries().to_vec();
        let count = |f: Filter| entries.iter().filter(|e| f == Filter::All || kind_of(e) == f).count();
        let (nb, ni, no) = (count(Filter::Bundled), count(Filter::Imported), count(Filter::Online));

        // Header: title and counts; Add API Key… and Import… at the right.
        let mut parts = Vec::new();
        for (n, what) in [(nb, "bundled"), (ni, "imported"), (no, "online")] {
            if n > 0 {
                parts.push(format!("{n} {what}"));
            }
        }
        let subtitle = format!("{} installed · {}", entries.len(), parts.join(", "));
        super::title(&painter, p, left, top, "Translations", &subtitle);
        let cy = top + 26.0 + 16.0;
        let (import, resp) = Button::new(Kind::Primary, "Import…").icon(icons::download).show_right(
            ui,
            p,
            right,
            cy,
            ui.id().with("import"),
        );
        if resp.clicked() {
            actions.push(Action::ImportTranslation);
        }
        let (_, resp) = Button::new(Kind::Secondary, "Add API Key…").icon(icons::key).show_right(
            ui,
            p,
            import.left() - 8.0,
            cy,
            ui.id().with("api-key"),
        );
        if resp.clicked() {
            actions.push(Action::OpenSettings(SettingsTab::Translations));
        }

        // Filter tabs, and the sort order at the right.
        let y = top + 58.0 + 20.0;
        let mut x = left;
        for (f, label, n) in [
            (Filter::All, "All", entries.len()),
            (Filter::Bundled, "Bundled", nb),
            (Filter::Imported, "Imported", ni),
            (Filter::Online, "Online", no),
        ] {
            let n = n.to_string();
            let on = self.filter == f;
            let w = widgets::pill_tab_width(&painter, label, Some(&n), on, false);
            let r = Rect::from_min_size(pos2(x, y), vec2(w, 28.0));
            if widgets::pill_tab(ui, p, r, label, Some(&n), None, on, ui.id().with(("tab", label))).clicked()
            {
                self.filter = f;
            }
            x = r.right() + 4.0;
        }
        let sort_label = match self.sort {
            Sort::Kind => "Kind",
            Sort::Name => "Name",
        };
        let value_w = painter
            .layout_no_wrap(sort_label.into(), theme::ui_semibold(12.0), Color32::PLACEHOLDER)
            .size()
            .x;
        let sort =
            Rect::from_min_size(pos2(right - value_w - 2.0 - 12.0, y + 6.0), vec2(value_w + 14.0, 16.0));
        let resp = ui.interact(sort, ui.id().with("sort"), Sense::click());
        text(&painter, sort.left(), sort.top(), 16.0, sort_label, theme::ui_semibold(12.0), p.ink_2, false);
        icons::chevron_down(
            &painter,
            Rect::from_min_size(pos2(right - 12.0, sort.top() + 2.0), vec2(12.0, 12.0)),
            p.ink_3,
        );
        text(&painter, sort.left() - 8.0, sort.top(), 16.0, "Sorted by", theme::ui_font(12.0), p.ink_3, true);
        crate::menu::popup(&resp, p).show(|ui| {
            for (s, label) in [(Sort::Kind, "Kind"), (Sort::Name, "Name")] {
                if ui.selectable_label(self.sort == s, label).clicked() {
                    self.sort = s;
                }
            }
        });

        // The list.
        let q = env.query.trim().to_lowercase();
        let mut shown: Vec<Arc<Entry>> = entries
            .iter()
            .filter(|e| self.filter == Filter::All || kind_of(e) == self.filter)
            .filter(|e| {
                q.is_empty()
                    || e.meta.abbrev.to_lowercase().contains(&q)
                    || e.meta.name.to_lowercase().contains(&q)
                    || description(e).to_lowercase().contains(&q)
            })
            .cloned()
            .collect();
        if self.sort == Sort::Name {
            shown.sort_by_key(|e| e.meta.name.to_lowercase());
        }
        let selected = self.selected.clone().unwrap_or_else(|| env.settings.translation.clone());
        let body = y + 28.0 + 20.0;
        let list_w = (right - left - 24.0 - INSPECTOR).max(320.0);
        let mut y = body;
        for e in &shown {
            let row = Rect::from_min_size(pos2(left, y), vec2(list_w, ROW));
            self.row(ui, p, row, e, env, &selected, actions);
            y += ROW + ROW_GAP;
        }
        if shown.is_empty() {
            text(
                &painter,
                left + 12.0,
                body + 20.0,
                20.0,
                "No translations match.",
                theme::ui_font(13.0),
                p.ink_3,
                false,
            );
            y = body + 60.0;
        }
        let list_bottom = y;

        // The inspector.
        let inspector_bottom = match entries.iter().find(|e| e.meta.id == selected) {
            Some(e) => self.inspector(ui, p, pos2(left + list_w + 24.0, body), e, env, actions),
            None => body,
        };
        list_bottom.max(inspector_bottom)
    }

    #[allow(clippy::too_many_arguments)]
    fn row(
        &mut self,
        ui: &mut Ui,
        p: &Palette,
        row: Rect,
        e: &Entry,
        env: &PageEnv<'_>,
        selected: &str,
        actions: &mut Vec<Action>,
    ) {
        let m = &e.meta;
        let id = ui.id().with(("translation-row", &m.id));
        let resp = ui.interact(row, id, Sense::click());
        let more = Rect::from_min_size(pos2(row.right() - 12.0 - 28.0, row.top() + 16.0), vec2(28.0, 28.0));
        let more_resp = ui.interact(more, id.with("more"), Sense::click());
        let is_selected = m.id == selected;
        let is_default = m.id == env.settings.translation;
        let painter = ui.painter().clone();
        if is_selected {
            painter.rect_filled(row, 12.0, p.accent_soft);
        } else if let Some(fill) = widgets::hover_fill(ui, &resp, p, more_resp.hovered()) {
            painter.rect_filled(row, 12.0, fill);
        }
        // Abbreviation tile.
        let tile = Rect::from_min_size(pos2(row.left() + 12.0, row.top() + 10.0), vec2(48.0, 40.0));
        painter.rect_filled(tile, 10.0, if is_selected { p.raised } else { p.fill });
        let size = if m.abbrev.chars().count() > 3 { 11.0 } else { 12.0 };
        let ink = if is_selected || is_default { p.accent } else { p.ink_2 };
        let g = widgets::line(&painter, &m.abbrev, theme::ui_bold(size), ink, 0.0);
        let x = tile.center().x - g.size().x / 2.0;
        widgets::paint_on_baseline(&painter, x, css_baseline(tile.top() + 12.0, 16.0, size, false), g, ink);

        // Badges (right to left), then the size, then the name and description in what's left.
        let mut badges: Vec<(BadgeKind, &str)> = vec![match e.kind {
            EntryKind::Bundled(_) => (BadgeKind::Neutral, "Bundled"),
            EntryKind::Imported(_) => (BadgeKind::Accent, "Imported"),
            EntryKind::Online(_) => (BadgeKind::Success, "Online"),
        }];
        if is_default {
            badges.push((BadgeKind::Solid, "Default"));
        }
        let mut bx = more.left() - 14.0;
        for (kind, label) in badges.iter().rev() {
            let w = widgets::badge_width(&painter, *kind, label);
            widgets::badge(&painter, p, bx - w, row.top() + 30.0, *kind, label);
            bx -= w + 6.0;
        }
        let size_right = more.left() - 14.0 - 150.0 - 14.0;
        let size = self.size_label(e, env);
        text(&painter, size_right, row.top() + 22.0, 16.0, &size, theme::ui_font(12.0), p.ink_3, true);
        let tx = tile.right() + 14.0;
        let width = size_right - 110.0 - 14.0 - tx;
        let name = elide(&painter, &m.name, &theme::ui_semibold(13.0), width);
        text(&painter, tx, row.top() + 12.0, 20.0, &name, theme::ui_semibold(13.0), p.ink, false);
        let desc = elide(&painter, &description(e), &theme::ui_font(12.0), width);
        text(&painter, tx, row.top() + 32.0, 16.0, &desc, theme::ui_font(12.0), p.ink_2, false);

        // More: a menu of what can be done with it.
        if more_resp.hovered() || more_resp.is_pointer_button_down_on() {
            let fill = if more_resp.is_pointer_button_down_on() { p.fill_strong } else { p.fill };
            painter.circle_filled(more.center(), 14.0, fill);
        }
        icons::heavy(|| {
            icons::ellipsis(&painter, Rect::from_center_size(more.center(), vec2(16.0, 16.0)), p.ink_2)
        });
        crate::menu::popup(&more_resp, p).show(|ui| {
            ui.set_min_width(180.0);
            if ui.button("Read").clicked() {
                actions.push(Action::SetTranslation(m.id.clone()));
                actions.push(Action::OpenPage(Page::Reader));
            }
            if ui.add_enabled(!is_default, egui::Button::new("Make Default")).clicked() {
                actions.push(Action::SetTranslation(m.id.clone()));
            }
            if ui.add_enabled(!is_default, egui::Button::new("Read in Parallel")).clicked() {
                actions.push(Action::OpenInParallel(Box::new(Action::AddParallel(m.id.clone()))));
                actions.push(Action::OpenPage(Page::Reader));
            }
            if !matches!(e.kind, EntryKind::Bundled(_)) {
                crate::menu::separator(ui);
                if ui.button("Remove…").clicked() {
                    actions.push(Action::RemoveTranslation(m.id.clone()));
                }
            }
        });

        if resp.clicked() {
            self.selected = Some(m.id.clone());
        }
        if resp.double_clicked() {
            actions.push(Action::SetTranslation(m.id.clone()));
            actions.push(Action::OpenPage(Page::Reader));
        }
    }

    /// The selected translation's card: tile, name, badges, a sample, facts and actions.
    fn inspector(
        &mut self,
        ui: &mut Ui,
        p: &Palette,
        at: egui::Pos2,
        e: &Entry,
        env: &PageEnv<'_>,
        actions: &mut Vec<Action>,
    ) -> f32 {
        let m = &e.meta;
        let painter = ui.painter().clone();
        let x = at.x + 20.0;
        let inner = INSPECTOR - 40.0;
        let local = if e.is_local() { env.library.load(&m.id).ok() } else { None };
        let sample = local
            .as_ref()
            .and_then(|t| t.find(VerseRef::new(BookId(18), 23, 1)))
            .map(|v| v.plain_text())
            .unwrap_or_default();
        let serif = FontId::new(14.0, theme::family(theme::SERIF));
        let lines =
            if sample.is_empty() { Vec::new() } else { flow(&painter, &sample, &serif, 10.0, inner, 2) };
        let sample_h = if lines.is_empty() { 0.0 } else { lines.len() as f32 * 20.0 + 16.0 };
        let height = 20.0 + 48.0 + 16.0 + 20.0 + 16.0 + sample_h + 88.0 + 16.0 + 24.0 + 20.0;
        let card = Rect::from_min_size(at, vec2(INSPECTOR, height));
        p.shadows.panel.paint(&painter, card, 16.0);
        painter.rect_filled(card, 16.0, p.sidebar);

        // Header: a raised tile, the name and a line of facts.
        let mut y = at.y + 20.0;
        let tile = Rect::from_min_size(pos2(x, y), vec2(56.0, 48.0));
        p.shadows.control.paint(&painter, tile, 12.0);
        painter.rect_filled(tile, 12.0, p.raised);
        let g = widgets::line(&painter, &m.abbrev, theme::ui_bold(15.0), p.accent, 0.0);
        let gx = tile.center().x - g.size().x / 2.0;
        widgets::paint_on_baseline(
            &painter,
            gx,
            css_baseline(tile.top() + 14.0, 20.0, 15.0, false),
            g,
            p.accent,
        );
        let tx = tile.right() + 12.0;
        let name = elide(&painter, &m.name, &theme::ui_semibold(15.0), card.right() - 20.0 - tx);
        text(&painter, tx, y + 5.0, 20.0, &name, theme::ui_semibold(15.0), p.ink, false);
        let books = local.as_ref().map(|t| (0..66).filter(|b| t.has_book(BookId(*b))).count());
        let mut facts = Vec::new();
        if let Some(year) = bundled::year(&m.id) {
            facts.push(year.to_string());
        }
        facts.push(language(&m.language).to_string());
        if let Some(n) = books {
            facts.push(format!("{n} books"));
        }
        text(&painter, tx, y + 27.0, 16.0, &facts.join(" · "), theme::ui_font(12.0), p.ink_2, false);
        y += 48.0 + 16.0;

        // Badges.
        let mut badges = vec![match e.kind {
            EntryKind::Bundled(_) => (BadgeKind::Neutral, "Bundled"),
            EntryKind::Imported(_) => (BadgeKind::Accent, "Imported"),
            EntryKind::Online(_) => (BadgeKind::Success, "Online"),
        }];
        if m.license.starts_with("Public Domain") {
            badges.push((BadgeKind::Outline, "Public domain"));
        }
        if m.headings {
            badges.push((BadgeKind::Accent, "Headings"));
        }
        if m.red_letters {
            badges.push((BadgeKind::Accent, "Red letters"));
        }
        let mut bx = x;
        for (kind, label) in badges {
            let w = widgets::badge_width(&painter, kind, label);
            if bx + w > card.right() - 20.0 {
                break;
            }
            bx = widgets::badge(&painter, p, bx, y + 10.0, kind, label).right() + 6.0;
        }
        y += 20.0 + 16.0;

        // Psalm 23:1 as this translation has it.
        for (k, l) in lines.iter().enumerate() {
            paint_runs(&painter, x, css_baseline(y + 20.0 * k as f32, 20.0, 14.0, true), l, p.ink);
        }
        y += sample_h;

        // Facts.
        let source = match &e.kind {
            EntryKind::Bundled(_) => {
                let format = if m.source.contains("usfm") { " USFM" } else { "" };
                // eBible.org writes its name with a lower-case e.
                let host = host(&m.source).unwrap_or("Bundled").replace("ebible.org", "eBible.org");
                format!("{host}{format}")
            }
            EntryKind::Imported(_) => import_format(&m.source).to_string(),
            EntryKind::Online(o) => o.config.provider.label().to_string(),
        };
        let size = self.size_label(e, env);
        let value_w = card.right() - 20.0 - (x + 80.0);
        for (k, (label, value)) in
            [("Source", source), ("Licence", m.license.clone()), ("Size", size), ("Features", features(e))]
                .into_iter()
                .enumerate()
        {
            let ry = y + 24.0 * k as f32;
            text(&painter, x, ry, 16.0, label, theme::ui_font(12.0), p.ink_3, false);
            let v = elide(&painter, &value, &theme::ui_medium(12.0), value_w);
            text(&painter, x + 80.0, ry, 16.0, &v, theme::ui_medium(12.0), p.ink, false);
        }
        y += 88.0 + 16.0;

        // Actions.
        let is_default = m.id == env.settings.translation;
        let label = if is_default { "Default" } else { "Make Default" };
        let (r, resp) = Button::new(Kind::Secondary, label).small().enabled(!is_default).show_left(
            ui,
            p,
            x,
            y + 12.0,
            ui.id().with("make-default"),
        );
        if resp.clicked() {
            actions.push(Action::SetTranslation(m.id.clone()));
        }
        let (_, resp) = Button::new(Kind::Plain, "Compare")
            .small()
            .icon(icons::compare)
            .enabled(!is_default)
            .show_left(ui, p, r.right() + 8.0, y + 12.0, ui.id().with("compare"));
        if resp.clicked() {
            actions.push(Action::OpenInParallel(Box::new(Action::AddParallel(m.id.clone()))));
            actions.push(Action::OpenPage(Page::Reader));
        }
        card.bottom()
    }
}
