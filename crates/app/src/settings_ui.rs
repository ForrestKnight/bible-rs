//! The Settings window (frame 09 in the design): Reading, Appearance, Translations, Shortcuts,
//! Data and About, as icon tabs under a 52 pt title bar. It is a window of its own; when the app
//! renders offscreen (screenshots) it fills the main window instead.

use std::path::PathBuf;
use std::sync::mpsc::Sender;

use bible_core::library::{EntryKind, Library};
use bible_core::model::Origin;
use bible_core::online::apibible::RemoteBible;
use bible_core::online::{OnlineConfig, ProviderKind};
use bible_core::study::StudyStore;
use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{
    self, Align2, Color32, FontId, Painter, Rect, RichText, ScrollArea, Sense, Stroke, Ui, pos2, vec2,
};

use crate::actions::{Action, ExportKind, SettingsTab};
use crate::app::{Settings, TaskResult};
use crate::icons;
use crate::reader::layout::{Mode, Typography};
use crate::theme::{self, Palette, Theme, text};
use crate::widgets;

/// The window's width, its title bar and its tab strip (icon tabs 76 × 52, 12 pt below).
const WIDTH: f32 = 720.0;
const TITLE_BAR: f32 = 52.0;
const HEADER: f32 = TITLE_BAR + 52.0 + 12.0;
const MARGIN: f32 = 32.0;

type IconFn = fn(&Painter, Rect, Color32);

const TABS: [(SettingsTab, &str, IconFn); 6] = [
    (SettingsTab::Reading, "Reading", icons::text_size),
    (SettingsTab::Appearance, "Appearance", icons::sun),
    (SettingsTab::Translations, "Translations", icons::book),
    (SettingsTab::Shortcuts, "Shortcuts", icons::keyboard),
    (SettingsTab::Data, "Data", icons::tray),
    (SettingsTab::About, "About", icons::info),
];

pub struct SettingsUi {
    tab: SettingsTab,
    esv_key: String,
    apibible_key: String,
    show_keys: bool,
    bibles: Option<Result<Vec<RemoteBible>, String>>,
    loading_bibles: bool,
    bible_filter: String,
    unlock_key: String,
    message: Option<String>,
}

impl SettingsUi {
    pub fn new(tab: SettingsTab, library: &Library) -> Self {
        Self {
            tab,
            esv_key: library.api_key("esv-api-key").unwrap_or_default(),
            apibible_key: library.api_key("api-bible-key").unwrap_or_default(),
            show_keys: false,
            bibles: None,
            loading_bibles: false,
            bible_filter: String::new(),
            unlock_key: String::new(),
            message: None,
        }
    }

    pub fn bibles_loaded(&mut self, r: Result<Vec<RemoteBible>, String>) {
        self.loading_bibles = false;
        self.bibles = Some(r);
    }

    fn title(&self) -> &'static str {
        TABS.iter().find(|(t, _, _)| *t == self.tab).map_or("Settings", |(_, l, _)| l)
    }

    /// Shows the window; returns `false` once it is closed.
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        library: &mut Library,
        settings: &mut Settings,
        study: &StudyStore,
        actions: &mut Vec<Action>,
        tx: &Sender<TaskResult>,
        import: &mut Option<(PathBuf, Option<String>)>,
    ) -> bool {
        if crate::offscreen() {
            // A screenshot of the window: fill the main window with it.
            let screen = ctx.content_rect();
            egui::Area::new(egui::Id::new("settings-offscreen"))
                .order(egui::Order::Foreground)
                .fixed_pos(screen.min)
                .show(ctx, |ui| {
                    ui.set_min_size(screen.size());
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(screen));
                    self.window(&mut child, library, settings, study, actions, tx, import);
                });
            return true;
        }
        let mut open = true;
        let mut builder = egui::ViewportBuilder::default()
            .with_title("Settings")
            .with_inner_size([WIDTH, 860.0])
            .with_min_inner_size([WIDTH, 420.0])
            .with_max_inner_size([WIDTH, 4000.0]);
        if cfg!(target_os = "macos") {
            builder =
                builder.with_fullsize_content_view(true).with_titlebar_shown(false).with_title_shown(false);
        }
        ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("settings"), builder, |ui, _class| {
            #[cfg(target_os = "macos")]
            crate::macos_window::unify_window_titled("Settings");
            if ui.ctx().input(|i| i.viewport().close_requested() || i.key_pressed(egui::Key::Escape))
                && !ui.ctx().egui_wants_keyboard_input()
            {
                open = false;
            }
            if ui.ctx().input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::W)) {
                open = false;
            }
            let rect = ui.max_rect();
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
            self.window(&mut child, library, settings, study, actions, tx, import);
        });
        open
    }

    /// The window's content: title bar, tabs, then the tab's settings.
    #[allow(clippy::too_many_arguments)]
    fn window(
        &mut self,
        ui: &mut Ui,
        library: &mut Library,
        settings: &mut Settings,
        study: &StudyStore,
        actions: &mut Vec<Action>,
        tx: &Sender<TaskResult>,
        import: &mut Option<(PathBuf, Option<String>)>,
    ) {
        let p = settings.theme.palette();
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, p.sidebar);
        // Header: the title over the tabs, in the toolbar material with a hairline.
        let header = Rect::from_min_size(rect.min, vec2(rect.width(), HEADER));
        ui.painter().rect_filled(header, 0.0, p.paper);
        ui.painter().rect_filled(header, 0.0, p.toolbar);
        ui.painter().hline(header.x_range(), header.bottom() - 0.5, Stroke::new(1.0, p.separator));
        ui.painter().text(
            pos2(header.center().x, rect.top() + TITLE_BAR / 2.0),
            Align2::CENTER_CENTER,
            self.title(),
            text::UI_STRONG.font(),
            p.ink,
        );
        let tabs_w = TABS.len() as f32 * 76.0 + (TABS.len() - 1) as f32 * 4.0;
        let mut x = header.center().x - tabs_w / 2.0;
        for (tab, label, icon) in TABS {
            let r = Rect::from_min_size(pos2(x, rect.top() + TITLE_BAR), vec2(76.0, 52.0));
            let resp = ui.interact(r, ui.id().with(("settings-tab", label)), Sense::click());
            let on = self.tab == tab;
            if on {
                ui.painter().rect_filled(r, 10.0, p.accent_soft);
            } else if let Some(fill) = widgets::hover_fill(ui, &resp, p, false) {
                ui.painter().rect_filled(r, 10.0, fill);
            }
            let top = r.top() + (52.0 - 38.0) / 2.0;
            icon(
                ui.painter(),
                Rect::from_min_size(pos2(r.center().x - 10.0, top), vec2(20.0, 20.0)),
                if on { p.accent } else { p.ink_2 },
            );
            let (font, color) =
                if on { (theme::ui_semibold(11.0), p.accent) } else { (theme::ui_medium(11.0), p.ink_2) };
            ui.painter().text(
                pos2(r.center().x, top + 20.0 + 2.0 + 8.0),
                Align2::CENTER_CENTER,
                label,
                font,
                color,
            );
            if resp.clicked() {
                self.tab = tab;
            }
            x += 76.0 + 4.0;
        }
        #[cfg(target_os = "macos")]
        {
            // The title bar moves the window; the tabs don't (see `macos_window::NoDragAreas`).
        }

        let body = Rect::from_min_max(pos2(rect.left(), header.bottom()), rect.max);
        let mut child = ui.new_child(
            egui::UiBuilder::new().max_rect(body).layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_clip_rect(body);
        ScrollArea::vertical().auto_shrink(false).id_salt(("settings", self.title())).show(
            &mut child,
            |ui| {
                egui::Frame::NONE
                    .inner_margin(egui::Margin {
                        left: MARGIN as i8,
                        right: MARGIN as i8,
                        top: 24,
                        bottom: 32,
                    })
                    .show(ui, |ui| {
                        ui.set_width(body.width() - 2.0 * MARGIN);
                        ui.spacing_mut().item_spacing.y = 0.0;
                        match self.tab {
                            SettingsTab::Reading => reading(ui, library, settings, actions),
                            SettingsTab::Appearance => appearance(ui, settings, actions),
                            SettingsTab::Translations => {
                                ui.spacing_mut().item_spacing.y = 6.0;
                                self.translations(ui, library, settings, actions, tx, import)
                            }
                            SettingsTab::Shortcuts => shortcuts(ui, p),
                            SettingsTab::Data => {
                                ui.spacing_mut().item_spacing.y = 6.0;
                                data(ui, library, study, actions)
                            }
                            SettingsTab::About => about(ui, p),
                        }
                    });
            },
        );
    }

    fn translations(
        &mut self,
        ui: &mut Ui,
        library: &mut Library,
        settings: &mut Settings,
        actions: &mut Vec<Action>,
        tx: &Sender<TaskResult>,
        import: &mut Option<(PathBuf, Option<String>)>,
    ) {
        let p = settings.theme.palette();
        widgets::caption(ui, "Installed");
        let mut remove: Option<String> = None;
        for e in library.entries() {
            let m = &e.meta;
            ui.horizontal_top(|ui| {
                ui.add_sized(
                    [58.0, 20.0],
                    egui::Label::new(RichText::new(&m.abbrev).font(theme::ui_medium(13.0)).color(p.accent)),
                );
                let w = (ui.available_width() - 90.0).max(200.0);
                ui.allocate_ui_with_layout(vec2(w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(w);
                    ui.label(RichText::new(&m.name).color(p.ink));
                    let origin = match &m.origin {
                        Origin::Bundled => format!("Bundled · {}", m.license),
                        Origin::Imported { format, file_name } => {
                            format!("Imported {format} from {file_name} · stored only on this computer")
                        }
                        Origin::Online { provider, .. } => {
                            let cached = library.cached_verses(&m.id);
                            format!(
                                "Online via {provider} · {cached} verses cached (within the API's limits)"
                            )
                        }
                    };
                    ui.label(RichText::new(origin).font(theme::ui_font(11.5)).color(p.ink_2));
                });
                if !matches!(e.kind, EntryKind::Bundled(_)) && ui.button("Remove").clicked() {
                    remove = Some(m.id.clone());
                }
            });
            ui.add_space(6.0);
        }
        if let Some(id) = remove {
            match library.remove(&id) {
                Ok(()) => {
                    if settings.translation == id {
                        settings.translation = bible_core::bundled::DEFAULT_ID.into();
                    }
                    settings.parallel.retain(|x| *x != id);
                    actions.push(Action::Toast(format!("Removed {id}")));
                }
                Err(e) => actions.push(Action::Toast(format!("{e:#}"))),
            }
        }

        ui.add_space(14.0);
        widgets::caption(ui, "Import a translation you own");
        ui.label(
            RichText::new(
                "Licensed translations (NLT, NASB, LSB, ESV, NIV, …) are never bundled. If you have a copy as USFM \
                 (file, folder or zip), OSIS XML, Zefania XML or a SWORD module, import it here. It is converted and \
                 stored in your own data folder.",
            )
            .color(p.ink_2),
        );
        ui.horizontal(|ui| {
            ui.label("SWORD unlock key (if locked):");
            ui.add(egui::TextEdit::singleline(&mut self.unlock_key).desired_width(160.0).password(true));
        });
        ui.horizontal(|ui| {
            if ui.button("Import file…").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .set_title("Import a translation (USFM, OSIS, Zefania, SWORD)")
                    .add_filter("Bible files", &["xml", "osis", "usfm", "sfm", "zip", "conf", "txt"])
                    .pick_file()
            {
                *import = Some((path, Some(self.unlock_key.clone()).filter(|k| !k.is_empty())));
            }
            if ui
                .button("Import folder…")
                .on_hover_text("A USFM folder or an unpacked SWORD module")
                .clicked()
                && let Some(path) = rfd::FileDialog::new().pick_folder()
            {
                *import = Some((path, Some(self.unlock_key.clone()).filter(|k| !k.is_empty())));
            }
        });

        ui.add_space(14.0);
        widgets::caption(ui, "Online translations (your own API key)");
        ui.label(
            RichText::new(format!(
                "Keys are stored in the {}. Fetched text is cached only as each API's terms allow and shown with \
                 the publisher's copyright notice.",
                library.secrets().backend_name()
            ))
            .color(p.ink_2),
        );
        ui.checkbox(&mut self.show_keys, "Show keys");
        ui.add_space(4.0);
        // ESV.
        ui.horizontal(|ui| {
            ui.label(RichText::new("ESV API").font(theme::ui_medium(13.0)));
            ui.hyperlink_to("get a key", "https://api.esv.org/");
        });
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.esv_key)
                    .desired_width(320.0)
                    .password(!self.show_keys)
                    .hint_text("ESV API token"),
            );
            if ui.button("Save").clicked() {
                match library.set_api_key("esv-api-key", &self.esv_key) {
                    Ok(()) => self.message = Some("ESV key saved".into()),
                    Err(e) => self.message = Some(format!("Couldn't save key: {e:#}")),
                }
            }
            let has = library
                .entries()
                .iter()
                .any(|e| e.online().is_some_and(|o| o.config.provider == ProviderKind::Esv));
            if !has && ui.button("Add ESV").clicked() {
                let cfg = OnlineConfig {
                    id: "esv".into(),
                    abbrev: "ESV".into(),
                    name: "English Standard Version".into(),
                    provider: ProviderKind::Esv,
                    copyright: bible_core::online::esv::COPYRIGHT.into(),
                };
                match library.add_online(cfg) {
                    Ok(id) => actions.push(Action::SetTranslation(id)),
                    Err(e) => self.message = Some(format!("{e:#}")),
                }
            }
        });
        ui.add_space(8.0);
        // API.Bible.
        ui.horizontal(|ui| {
            ui.label(RichText::new("API.Bible").font(theme::ui_medium(13.0)));
            ui.hyperlink_to("get a key", "https://scripture.api.bible/");
            ui.label(
                RichText::new("NASB, LSB, NLT and others where your plan and the publisher allow")
                    .color(p.ink_2),
            );
        });
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.apibible_key)
                    .desired_width(320.0)
                    .password(!self.show_keys)
                    .hint_text("API.Bible key"),
            );
            if ui.button("Save").clicked() {
                match library.set_api_key("api-bible-key", &self.apibible_key) {
                    Ok(()) => self.message = Some("API.Bible key saved".into()),
                    Err(e) => self.message = Some(format!("Couldn't save key: {e:#}")),
                }
            }
            if ui
                .add_enabled(
                    !self.loading_bibles && !self.apibible_key.trim().is_empty(),
                    egui::Button::new("List my Bibles"),
                )
                .clicked()
            {
                self.loading_bibles = true;
                let key = self.apibible_key.trim().to_string();
                let base = library.api_bible_base.clone();
                let tx = tx.clone();
                let _ = library.set_api_key("api-bible-key", &key);
                std::thread::spawn(move || {
                    let r = bible_core::online::apibible::list_bibles(&base, &key, Some("eng"))
                        .map_err(|e| format!("{e:#}"));
                    let _ = tx.send(TaskResult::Bibles(r));
                });
            }
            if self.loading_bibles {
                ui.spinner();
            }
        });
        match &self.bibles {
            Some(Ok(list)) => {
                ui.add(
                    egui::TextEdit::singleline(&mut self.bible_filter)
                        .hint_text("Filter (e.g. NASB, LSB, NLT)")
                        .desired_width(320.0),
                );
                let f = self.bible_filter.to_lowercase();
                let mut add: Option<RemoteBible> = None;
                egui::ScrollArea::vertical().max_height(220.0).id_salt("bibles").show(ui, |ui| {
                    for b in list.iter().filter(|b| {
                        f.is_empty()
                            || b.name.to_lowercase().contains(&f)
                            || b.abbreviation.to_lowercase().contains(&f)
                    }) {
                        ui.horizontal(|ui| {
                            let installed = library.entries().iter().any(|e| {
                                e.online().is_some_and(|o| {
                                    o.config.provider == ProviderKind::ApiBible { bible_id: b.id.clone() }
                                })
                            });
                            if installed {
                                ui.label(RichText::new("Added").color(p.ink_2));
                            } else if ui.small_button("Add").clicked() {
                                add = Some(b.clone());
                            }
                            ui.label(
                                RichText::new(&b.abbreviation).font(theme::ui_medium(12.5)).color(p.accent),
                            );
                            ui.label(&b.name);
                        });
                    }
                });
                if let Some(b) = add {
                    let cfg = OnlineConfig {
                        id: format!("api-{}", bible_core::import::slug(&b.abbreviation)),
                        abbrev: b.abbreviation.clone(),
                        name: b.name.clone(),
                        provider: ProviderKind::ApiBible { bible_id: b.id.clone() },
                        copyright: String::new(),
                    };
                    match library.add_online(cfg) {
                        Ok(id) => actions.push(Action::SetTranslation(id)),
                        Err(e) => self.message = Some(format!("{e:#}")),
                    }
                }
            }
            Some(Err(e)) => {
                ui.colored_label(p.rubric, e);
            }
            None => {}
        }
        if let Some(m) = &self.message {
            ui.add_space(6.0);
            ui.label(RichText::new(m).color(p.accent));
        }
    }
}

/// A group: a 13 pt semibold title, then its content 8 pt below; groups are 24 pt apart.
fn group_title(ui: &mut Ui, p: &Palette, title: &str) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
    ui.painter().text(r.left_center(), Align2::LEFT_CENTER, title, text::UI_STRONG.font(), p.ink);
    ui.add_space(8.0);
}

/// A card of rows (`ds-14` form group): the surface colour with a hairline ring, radius 12; rows
/// are at least 44 pt, separated by hairlines inset 16 pt.
fn rows_card(ui: &mut Ui, p: &Palette, rows: &[f32], mut row: impl FnMut(&mut Ui, usize, Rect)) {
    let width = ui.available_width();
    let height: f32 = rows.iter().sum::<f32>() + (rows.len().saturating_sub(1)) as f32;
    let (card, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    ui.painter().rect_filled(card, 12.0, p.surface);
    ui.painter().rect_stroke(card, 12.0, Stroke::new(0.5, p.border), egui::StrokeKind::Outside);
    let mut y = card.top();
    for (i, h) in rows.iter().enumerate() {
        if i > 0 {
            ui.painter().hline(card.left() + 16.0..=card.right(), y + 0.5, Stroke::new(1.0, p.separator));
            y += 1.0;
        }
        row(ui, i, Rect::from_min_size(pos2(card.left(), y), vec2(width, *h)));
        y += h;
    }
}

/// A row's label (13 pt) and optional detail (12 pt tertiary) at the row's left.
fn row_label(ui: &Ui, p: &Palette, r: Rect, label: &str, detail: Option<&str>) {
    let x = r.left() + 16.0;
    match detail {
        None => {
            ui.painter().text(pos2(x, r.center().y), Align2::LEFT_CENTER, label, text::UI.font(), p.ink);
        }
        Some(d) => {
            ui.painter().text(
                pos2(x, r.center().y - 8.0),
                Align2::LEFT_CENTER,
                label,
                text::UI.font(),
                p.ink,
            );
            ui.painter().text(
                pos2(x, r.center().y + 10.0),
                Align2::LEFT_CENTER,
                d,
                text::SMALL.font(),
                p.ink_3,
            );
        }
    }
}

/// Line length shown in characters; stored as the measure in ems (66 characters ≈ 32 em).
const CHARS_PER_EM: f32 = 66.0 / 32.0;

fn reading(ui: &mut Ui, library: &Library, settings: &mut Settings, actions: &mut Vec<Action>) {
    let p = settings.theme.palette();

    // Preview: John 1:1–2 as the reader would set it.
    let typo = settings.typo.clone();
    let width = ui.available_width();
    let inner = width - 48.0;
    let (v1, v2) = library
        .load(&settings.translation)
        .ok()
        .map(|t| {
            let get = |v| {
                t.find(bible_core::VerseRef::new(bible_core::BookId(42), 1, v))
                    .map(|v| v.plain_text())
                    .unwrap_or_default()
            };
            (get(1), get(2))
        })
        .unwrap_or_default();
    let lh = (typo.size * typo.line_height).round();
    let mut job = LayoutJob::default();
    let body = TextFormat {
        font_id: FontId::new(typo.size, theme::family(theme::SERIF)),
        color: p.ink,
        line_height: Some(lh),
        ..Default::default()
    };
    if typo.verse_numbers {
        job.append(
            "1\u{a0}",
            0.0,
            TextFormat {
                font_id: FontId::new((typo.size * 0.55).round(), theme::family(theme::UI_SEMIBOLD)),
                color: p.verse,
                line_height: Some(lh),
                valign: egui::Align::TOP,
                ..Default::default()
            },
        );
    }
    job.append(&format!("{v1} {v2}"), 0.0, body);
    job.wrap =
        TextWrapping { max_width: inner.min(typo.size * typo.measure), max_rows: 4, ..Default::default() };
    job.justify = typo.justify;
    let galley = ui.painter().layout_job(job);
    let card_h = 16.0 + 16.0 + 8.0 + galley.size().y + 20.0;
    let (card, _) = ui.allocate_exact_size(vec2(width, card_h), Sense::hover());
    ui.painter().rect_filled(card, 12.0, p.paper);
    ui.painter().rect_stroke(card, 12.0, Stroke::new(0.5, p.border), egui::StrokeKind::Outside);
    let label_font = theme::ui_semibold(text::LABEL.size);
    let mut label = LayoutJob::single_section(
        "JOHN 1".into(),
        TextFormat {
            font_id: label_font,
            color: p.accent,
            extra_letter_spacing: 0.12 * 11.0,
            ..Default::default()
        },
    );
    label.wrap.max_width = f32::INFINITY;
    let lg = ui.painter().layout_job(label);
    ui.painter().galley(
        pos2(card.left() + 24.0, card.top() + 16.0 + (16.0 - lg.size().y) / 2.0),
        lg,
        p.accent,
    );
    ui.painter().galley(pos2(card.left() + 24.0, card.top() + 16.0 + 16.0 + 8.0), galley, p.ink);
    ui.add_space(8.0);
    let (r, _) = ui.allocate_exact_size(vec2(width, 16.0), Sense::hover());
    ui.painter().text(
        pos2(r.left() + 8.0, r.center().y),
        Align2::LEFT_CENTER,
        "Preview updates as you change settings.",
        text::SMALL.font(),
        p.ink_3,
    );
    ui.add_space(24.0);

    // Theme cards.
    group_title(ui, p, "Theme");
    theme_cards(ui, settings, actions);
    ui.add_space(24.0);

    // Text.
    group_title(ui, p, "Text");
    let typo = &mut settings.typo;
    rows_card(ui, p, &[44.0, 44.0, 44.0, 44.0], |ui, i, r| {
        let right = r.right() - 16.0;
        let cy = r.center().y;
        match i {
            0 => {
                row_label(ui, p, r, "Font", None);
                let field = Rect::from_min_size(pos2(right - 220.0, cy - 16.0), vec2(220.0, 32.0));
                let resp = widgets::select_field(ui, p, field, "Literata", None, ui.id().with("font"));
                egui::Popup::menu(&resp).show(|ui| {
                    let _ = ui.selectable_label(true, "Literata");
                });
            }
            1 => {
                row_label(ui, p, r, "Size", None);
                let value = format!("{:.0} pt", typo.size);
                ui.painter().text(
                    pos2(right, cy),
                    Align2::RIGHT_CENTER,
                    value,
                    theme::ui_medium(text::UI.size),
                    p.ink_2,
                );
                let big_a = right - 44.0 - 12.0;
                ui.painter().text(
                    pos2(big_a, cy),
                    Align2::RIGHT_CENTER,
                    "A",
                    FontId::new(19.0, theme::family(theme::SERIF)),
                    p.ink_2,
                );
                let slider =
                    Rect::from_min_size(pos2(big_a - 12.0 - 12.0 - 200.0, cy - 12.0), vec2(200.0, 24.0));
                ui.painter().text(
                    pos2(slider.left() - 12.0, cy),
                    Align2::RIGHT_CENTER,
                    "A",
                    FontId::new(13.0, theme::family(theme::SERIF)),
                    p.ink_2,
                );
                widgets::slider(ui, p, slider, ui.id().with("size"), &mut typo.size, 14.0..=32.0, 1.0, 7);
            }
            2 => {
                row_label(ui, p, r, "Line spacing", None);
                let value = format!("{}", (typo.line_height * 100.0).round() / 100.0);
                ui.painter().text(
                    pos2(right, cy),
                    Align2::RIGHT_CENTER,
                    value,
                    theme::ui_medium(text::UI.size),
                    p.ink_2,
                );
                let slider =
                    Rect::from_min_size(pos2(right - 44.0 - 12.0 - 256.0, cy - 12.0), vec2(256.0, 24.0));
                widgets::slider(
                    ui,
                    p,
                    slider,
                    ui.id().with("spacing"),
                    &mut typo.line_height,
                    1.3..=2.0,
                    0.05,
                    0,
                );
            }
            _ => {
                row_label(ui, p, r, "Line length", None);
                let mut chars = (typo.measure * CHARS_PER_EM).round();
                let value = format!("{chars:.0} ch");
                ui.painter().text(
                    pos2(right, cy),
                    Align2::RIGHT_CENTER,
                    value,
                    theme::ui_medium(text::UI.size),
                    p.ink_2,
                );
                let slider =
                    Rect::from_min_size(pos2(right - 44.0 - 12.0 - 256.0, cy - 12.0), vec2(256.0, 24.0));
                if widgets::slider(ui, p, slider, ui.id().with("length"), &mut chars, 45.0..=90.0, 1.0, 0) {
                    typo.measure = chars / CHARS_PER_EM;
                }
            }
        }
    });
    ui.add_space(24.0);

    // Layout.
    group_title(ui, p, "Layout");
    let mut red = typo.red_letters;
    rows_card(ui, p, &[44.0, 48.0, 44.0, 48.0, 44.0], |ui, i, r| {
        let right = pos2(r.right() - 16.0, r.center().y);
        match i {
            0 => {
                row_label(ui, p, r, "Verses", None);
                let selected = usize::from(typo.mode == Mode::VersePerLine);
                if let Some(k) = widgets::segmented_fit(
                    ui,
                    p,
                    right,
                    ui.id().with("verses"),
                    &["Paragraphs", "Verse per line"],
                    selected,
                ) {
                    typo.mode = if k == 0 { Mode::Paragraph } else { Mode::VersePerLine };
                }
            }
            1 => {
                row_label(ui, p, r, "Words of Christ in red", Some("⌘R"));
                widgets::toggle(ui, p, right, ui.id().with("red"), &mut red);
            }
            2 => {
                row_label(ui, p, r, "Verse numbers", None);
                widgets::toggle(ui, p, right, ui.id().with("numbers"), &mut typo.verse_numbers);
            }
            3 => {
                row_label(ui, p, r, "Section headings", Some("Where the translation has them (BSB, WEB)"));
                widgets::toggle(ui, p, right, ui.id().with("headings"), &mut typo.headings);
            }
            _ => {
                row_label(ui, p, r, "Justify text", None);
                widgets::toggle(ui, p, right, ui.id().with("justify"), &mut typo.justify);
            }
        }
    });
    typo.red_letters = red;
    ui.add_space(24.0);

    // Footer: the fonts' licence, and a reset.
    let (r, _) = ui.allocate_exact_size(vec2(width, 32.0), Sense::hover());
    ui.painter().text(
        r.left_center(),
        Align2::LEFT_CENTER,
        "Literata and Inter are bundled under the SIL Open Font License.",
        text::SMALL.font(),
        p.ink_3,
    );
    let label = "Reset to Defaults";
    let w = widgets::secondary_button_width(ui, label);
    let b = Rect::from_min_size(pos2(r.right() - w, r.top()), vec2(w, 32.0));
    if widgets::secondary_button(ui, p, b, label, ui.id().with("reset")).clicked() {
        *typo = Typography::default();
    }
}

/// The four theme cards: Light, Sepia, Dark, Match System (`ds-14` theme picker).
fn theme_cards(ui: &mut Ui, settings: &mut Settings, actions: &mut Vec<Action>) {
    let p = settings.theme.palette();
    let width = ui.available_width();
    let (row, _) = ui.allocate_exact_size(vec2(width, 108.0), Sense::hover());
    let gap = (width - 4.0 * 136.0) / 3.0;
    let options: [(Option<Theme>, &str); 4] = [
        (Some(Theme::Light), "Light"),
        (Some(Theme::Sepia), "Sepia"),
        (Some(Theme::Dark), "Dark"),
        (None, "Match System"),
    ];
    for (i, (t, label)) in options.into_iter().enumerate() {
        let card =
            Rect::from_min_size(pos2(row.left() + i as f32 * (136.0 + gap), row.top()), vec2(136.0, 84.0));
        let resp = ui.interact(
            card.union(Rect::from_min_size(card.left_bottom(), vec2(136.0, 24.0))),
            ui.id().with(("theme", i)),
            Sense::click(),
        );
        let on = match t {
            Some(t) => !settings.theme_system && settings.theme == t,
            None => settings.theme_system,
        };
        if on {
            ui.painter().rect_filled(card.expand(5.0), 15.0, p.focus);
            ui.painter().rect_filled(card.expand(2.0), 12.0, p.accent);
        } else if resp.hovered() {
            ui.painter().rect_filled(card.expand(3.0), 13.0, p.fill_strong);
        }
        match t {
            Some(t) => {
                let tp = t.palette();
                ui.painter().rect_filled(card, 10.0, tp.paper);
                let strip = Rect::from_min_size(card.min, vec2(28.0, card.height()));
                ui.painter().rect_filled(
                    strip,
                    egui::CornerRadius { nw: 10, sw: 10, ne: 0, se: 0 },
                    tp.sidebar,
                );
                let x = strip.right() + 12.0;
                ui.painter().text(
                    pos2(x, card.top() + 10.0 + 10.0),
                    Align2::LEFT_CENTER,
                    "Aa",
                    FontId::new(15.0, theme::family(theme::SERIF)),
                    tp.ink,
                );
                for (k, w) in [64.0, 72.0, 56.0].into_iter().enumerate() {
                    let y = card.top() + 10.0 + 20.0 + 6.0 + k as f32 * 10.0;
                    ui.painter().rect_filled(
                        Rect::from_min_size(pos2(x, y), vec2(w, 4.0)),
                        2.0,
                        tp.ink.gamma_multiply(0.55),
                    );
                }
            }
            None => {
                let half = card.width() / 2.0;
                ui.painter().rect_filled(
                    Rect::from_min_size(card.min, vec2(half, card.height())),
                    egui::CornerRadius { nw: 10, sw: 10, ne: 0, se: 0 },
                    crate::theme::LIGHT.paper,
                );
                ui.painter().rect_filled(
                    Rect::from_min_size(pos2(card.left() + half, card.top()), vec2(half, card.height())),
                    egui::CornerRadius { nw: 0, sw: 0, ne: 10, se: 10 },
                    crate::theme::DARK.paper,
                );
            }
        }
        ui.painter().rect_stroke(card, 10.0, Stroke::new(0.5, p.border), egui::StrokeKind::Outside);
        let (font, color) =
            if on { (theme::ui_semibold(text::SMALL.size), p.accent) } else { (text::SMALL.font(), p.ink_2) };
        ui.painter().text(
            pos2(card.center().x, card.bottom() + 8.0 + 8.0),
            Align2::CENTER_CENTER,
            label,
            font,
            color,
        );
        if resp.clicked() {
            match t {
                Some(t) => {
                    settings.theme_system = false;
                    actions.push(Action::SetTheme(t));
                }
                None => settings.theme_system = true,
            }
        }
    }
}
/// Appearance (frame 24): theme, accent colour, the toolbar's translation, the sidebar; names
/// for the highlight colours; motion.
fn appearance(ui: &mut Ui, settings: &mut Settings, actions: &mut Vec<Action>) {
    let p = settings.theme.palette();
    group_title(ui, p, "Appearance");
    rows_card(ui, p, &[44.0, 64.0, 44.0, 44.0], |ui, i, r| match i {
        0 => {
            row_label(ui, p, r, "Theme", None);
            let items: [(IconFn, &str); 4] =
                [(icons::sun, "Light"), (icons::page, "Sepia"), (icons::moon, "Dark"), (icons::auto, "Auto")];
            let selected = if settings.theme_system { 3 } else { settings.theme as usize };
            let right = pos2(r.right() - 16.0, r.center().y);
            if let Some(k) = icon_segmented(ui, p, right, ui.id().with("theme-seg"), &items, selected) {
                if k == 3 {
                    settings.theme_system = true;
                } else {
                    settings.theme_system = false;
                    actions.push(Action::SetTheme(Theme::ALL[k]));
                }
            }
        }
        1 => {
            row_label(ui, p, r, "Accent colour", Some("Tints selection, links and primary buttons"));
            let x0 = r.right() - 16.0 - (5.0 * 52.0 + 4.0 * 4.0);
            for (k, a) in theme::Accent::ALL.into_iter().enumerate() {
                let col = Rect::from_min_size(pos2(x0 + 56.0 * k as f32, r.top() + 10.0), vec2(52.0, 44.0));
                let resp = ui.interact(col, ui.id().with(("accent", k)), Sense::click());
                let c = pos2(col.center().x, col.top() + 11.0);
                let on = settings.accent == a;
                let painter = ui.painter();
                if on {
                    painter.circle_filled(c, 14.5, p.ink);
                    painter.circle_filled(c, 13.0, p.surface);
                } else if resp.hovered() {
                    painter.circle_filled(c, 14.0, p.fill_strong);
                }
                painter.circle_filled(c, 11.0, a.swatch());
                if on {
                    icons::check(painter, Rect::from_center_size(c, vec2(12.0, 12.0)), Color32::WHITE);
                } else {
                    painter.circle_stroke(c, 10.75, Stroke::new(0.5, Color32::from_white_alpha(51)));
                }
                let (font, ink) =
                    if on { (theme::ui_semibold(11.0), p.ink) } else { (theme::ui_medium(11.0), p.ink_3) };
                painter.text(
                    pos2(col.center().x, col.top() + 28.0 + 8.0),
                    Align2::CENTER_CENTER,
                    a.label(),
                    font,
                    ink,
                );
                if resp.clicked() && !on {
                    actions.push(Action::SetAccent(a));
                }
            }
        }
        2 => {
            row_label(ui, p, r, "Show translation in toolbar", None);
            let right = pos2(r.right() - 16.0, r.center().y);
            widgets::toggle(
                ui,
                p,
                right,
                ui.id().with("toolbar-translation"),
                &mut settings.toolbar_translation,
            );
        }
        _ => {
            row_label(ui, p, r, "Sidebar", None);
            let right = pos2(r.right() - 16.0, r.center().y);
            let (_, pick) = widgets::segmented_tight(
                ui,
                p,
                right,
                ui.id().with("sidebar-style"),
                &["Floating", "Attached"],
                usize::from(settings.sidebar_attached),
            );
            if let Some(k) = pick {
                settings.sidebar_attached = k == 1;
            }
        }
    });
    ui.add_space(24.0);

    group_title(ui, p, "Highlight colours");
    settings.highlight_names.resize(6, String::new());
    let colours = bible_core::study::HighlightColor::ALL;
    rows_card(ui, p, &[48.0; 6], |ui, i, r| {
        let c = colours[i];
        let dot = pos2(r.left() + 16.0 + 9.0, r.center().y);
        ui.painter().circle_filled(dot, 9.0, p.swatch(c));
        ui.painter().circle_stroke(dot, 8.75, Stroke::new(0.5, Color32::from_black_alpha(46)));
        let field = Rect::from_min_size(pos2(r.left() + 46.0, r.top() + 8.0), vec2(280.0, 32.0));
        let name = c.name();
        let placeholder = name[..1].to_uppercase() + &name[1..];
        widgets::text_field(
            ui,
            p,
            field,
            &mut settings.highlight_names[i],
            &placeholder,
            None,
            false,
            false,
            egui::Id::new(("highlight-name", i)),
        );
        let key = (i + 1).to_string();
        let kw = widgets::keycap_width(ui.painter(), &key);
        let cap = widgets::keycap(ui.painter(), p, r.right() - 16.0 - kw, r.center().y, &key);
        ui.painter().text(
            pos2(cap.left() - 12.0, r.center().y),
            Align2::RIGHT_CENTER,
            "Key",
            text::SMALL.font(),
            p.ink_3,
        );
    });
    ui.add_space(8.0);
    footnote(ui, p, "Names appear in the highlight menu and the Highlights filter.");
    ui.add_space(24.0);

    group_title(ui, p, "Motion");
    rows_card(ui, p, &[48.0, 44.0], |ui, i, r| {
        let right = pos2(r.right() - 16.0, r.center().y);
        if i == 0 {
            row_label(ui, p, r, "Reduce motion", Some("Follows the system setting by default"));
            let mut on = settings.reduce_motion;
            if widgets::toggle(ui, p, right, ui.id().with("reduce-motion"), &mut on).clicked() {
                actions.push(Action::SetReduceMotion(on));
            }
        } else {
            row_label(ui, p, r, "Flash a verse after jumping to it", None);
            widgets::toggle(ui, p, right, ui.id().with("flash-jumps"), &mut settings.flash_jumps);
        }
    });
}

/// A note under a group: 12 pt tertiary, aligned with the group's caption.
fn footnote(ui: &mut Ui, p: &Palette, note: &str) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
    ui.painter().text(
        pos2(r.left() + 4.0, r.center().y),
        Align2::LEFT_CENTER,
        note,
        text::SMALL.font(),
        p.ink_3,
    );
}

/// A segmented control of icon-and-label segments, drawn with its right edge at `right_center`.
fn icon_segmented(
    ui: &mut Ui,
    p: &Palette,
    right_center: egui::Pos2,
    id: egui::Id,
    items: &[(IconFn, &str)],
    selected: usize,
) -> Option<usize> {
    let widths: Vec<f32> = items
        .iter()
        .enumerate()
        .map(|(i, (_, l))| {
            let font = if i == selected { theme::ui_semibold(13.0) } else { theme::ui_medium(13.0) };
            ui.painter().layout_no_wrap(l.to_string(), font, Color32::PLACEHOLDER).size().x
                + 10.0
                + 16.0
                + 6.0
                + 10.0
        })
        .collect();
    let total = widths.iter().sum::<f32>() + 2.0 * (items.len() as f32 - 1.0) + 4.0;
    let track = Rect::from_min_size(pos2(right_center.x - total, right_center.y - 16.0), vec2(total, 32.0));
    ui.painter().rect_filled(track, 16.0, p.fill);
    let mut x = track.left() + 2.0;
    let mut out = None;
    for (i, ((icon, label), w)) in items.iter().zip(widths).enumerate() {
        let seg = Rect::from_min_size(pos2(x, track.top() + 2.0), vec2(w, 28.0));
        let resp = ui.interact(seg, id.with(i), Sense::click());
        let on = i == selected;
        if on {
            p.shadows.knob.paint(ui.painter(), seg, 14.0);
            ui.painter().rect_filled(seg, 14.0, p.raised);
        } else if let Some(fill) = widgets::hover_fill(ui, &resp, p, false) {
            ui.painter().rect_filled(seg, 14.0, fill);
        }
        let ink = if on { p.ink } else { p.ink_2 };
        let r = Rect::from_min_size(pos2(seg.left() + 10.0, seg.center().y - 8.0), vec2(16.0, 16.0));
        icons::heavy(|| icon(ui.painter(), r, ink));
        let font = if on { theme::ui_semibold(13.0) } else { theme::ui_medium(13.0) };
        ui.painter().text(pos2(r.right() + 6.0, seg.center().y), Align2::LEFT_CENTER, *label, font, ink);
        if resp.clicked() && !on {
            out = Some(i);
        }
        x += w + 2.0;
    }
    out
}

/// Every keyboard shortcut, by group.
fn shortcuts(ui: &mut Ui, p: &Palette) {
    for (title, rows) in crate::app::SHORTCUTS {
        group_title(ui, p, title);
        let heights: Vec<f32> = rows.iter().map(|_| 44.0).collect();
        rows_card(ui, p, &heights, |ui, i, r| {
            let (keys, what) = rows[i];
            row_label(ui, p, r, what, None);
            let w = widgets::keycap_width(ui.painter(), keys);
            widgets::keycap(ui.painter(), p, r.right() - 16.0 - w, r.center().y, keys);
        });
        ui.add_space(24.0);
    }
}

/// The app, its version and its licences.
fn about(ui: &mut Ui, p: &Palette) {
    let width = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(width, 120.0), Sense::hover());
    ui.painter().text(
        pos2(r.center().x, r.top() + 40.0),
        Align2::CENTER_CENTER,
        "bible-rs",
        FontId::new(28.0, theme::family(theme::SERIF_HEADING)),
        p.ink,
    );
    ui.painter().text(
        pos2(r.center().x, r.top() + 76.0),
        Align2::CENTER_CENTER,
        format!("Version {}", env!("CARGO_PKG_VERSION")),
        text::UI.font(),
        p.ink_2,
    );
    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "A fast, native Bible reader. Bundled translations are in the public domain; Literata and Inter are under the SIL Open Font License; the app is MIT-licensed.",
        )
        .color(p.ink_2),
    );
}

fn data(ui: &mut Ui, library: &Library, study: &StudyStore, actions: &mut Vec<Action>) {
    let (h, n, b) = study.counts();
    widgets::caption(ui, "Your study data");
    ui.label(format!("{h} highlights · {n} notes · {b} bookmarks"));
    ui.label(format!("Stored in {}", library.data_dir().join("study.sqlite").display()));
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui.button("Export Markdown…").clicked() {
            actions.push(Action::Export(ExportKind::Markdown));
        }
        if ui.button("Export JSON…").clicked() {
            actions.push(Action::Export(ExportKind::Json));
        }
        if ui.button("Import JSON…").clicked() {
            actions.push(Action::ImportStudy);
        }
    });
    ui.add_space(12.0);
    widgets::caption(ui, "Locations");
    ui.label(format!("Data folder: {}", library.data_dir().display()));
    ui.label(format!("API keys: {}", library.secrets().backend_name()));
}

/// The toolbar's "Aa" popover: the settings people change while reading.
pub fn quick_text_settings(ui: &mut Ui, settings: &mut Settings, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        for t in Theme::ALL {
            let p = t.palette();
            let (r, resp) = ui.allocate_exact_size(vec2(72.0, 44.0), egui::Sense::click());
            ui.painter().rect_filled(r, egui::CornerRadius::same(8), p.paper);
            let stroke = if settings.theme == t {
                egui::Stroke::new(2.0, p.accent)
            } else {
                egui::Stroke::new(1.0, p.separator)
            };
            ui.painter().rect_stroke(r, egui::CornerRadius::same(8), stroke, egui::StrokeKind::Inside);
            ui.painter().text(
                r.center(),
                Align2::CENTER_CENTER,
                t.label(),
                egui::FontId::new(13.0, theme::family(theme::SERIF)),
                p.ink,
            );
            if resp.clicked() {
                actions.push(Action::SetTheme(t));
            }
        }
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui.button(RichText::new("A−").font(theme::ui_medium(13.0))).on_hover_text("Smaller (⌘−)").clicked()
        {
            actions.push(Action::FontSize(-1.0));
        }
        ui.label(format!("{:.0} pt", settings.typo.size));
        if ui.button(RichText::new("A+").font(theme::ui_medium(15.0))).on_hover_text("Bigger (⌘+)").clicked()
        {
            actions.push(Action::FontSize(1.0));
        }
    });
    ui.add_space(4.0);
    let mut per_line = settings.typo.mode == Mode::VersePerLine;
    if ui.checkbox(&mut per_line, "Verse per line (⌘L)").changed() {
        actions.push(Action::ToggleMode);
    }
    let mut red = settings.typo.red_letters;
    if ui.checkbox(&mut red, "Red letters (⌘R)").changed() {
        actions.push(Action::ToggleRedLetters);
    }
    ui.checkbox(&mut settings.typo.justify, "Justify paragraphs");
    ui.separator();
    if ui.button("More settings…").clicked() {
        actions.push(Action::OpenSettings(SettingsTab::Reading));
        ui.close();
    }
}
