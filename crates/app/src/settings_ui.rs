//! Settings window (reading, translations & API keys, data) and the toolbar's text popover.

use std::path::PathBuf;
use std::sync::mpsc::Sender;

use bible_core::library::{EntryKind, Library};
use bible_core::model::Origin;
use bible_core::online::apibible::RemoteBible;
use bible_core::online::{OnlineConfig, ProviderKind};
use bible_core::study::StudyStore;
use eframe::egui::{self, Align2, Frame, Key, RichText, ScrollArea, Ui, vec2};

use crate::actions::{Action, ExportKind, SettingsTab};
use crate::app::{Settings, TaskResult};
use crate::reader::layout::{Mode, Typography};
use crate::theme::{self, Theme};
use crate::widgets;

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

    /// Returns `false` once closed.
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
        let mut open = true;
        let p = settings.theme.palette();
        egui::Window::new("Settings")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size(vec2(680.0, 560.0))
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .frame(Frame::window(&ctx.style_of(ctx.theme())).fill(p.chrome))
            .show(ctx, |ui| {
                let idx = match self.tab {
                    SettingsTab::Reading => 0,
                    SettingsTab::Translations => 1,
                    SettingsTab::Data => 2,
                };
                if let Some(i) = widgets::segmented(ui, &["Reading", "Translations", "Data"], idx) {
                    self.tab = [SettingsTab::Reading, SettingsTab::Translations, SettingsTab::Data][i];
                }
                ui.add_space(10.0);
                ScrollArea::vertical().auto_shrink(false).show(ui, |ui| match self.tab {
                    SettingsTab::Reading => reading(ui, settings, actions),
                    SettingsTab::Translations => {
                        self.translations(ui, library, settings, actions, tx, import)
                    }
                    SettingsTab::Data => data(ui, library, study, actions),
                });
            });
        if ctx.input(|i| i.key_pressed(Key::Escape)) && !ctx.egui_wants_keyboard_input() {
            open = false;
        }
        open
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
                    ui.label(RichText::new(&m.name).color(p.text));
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
                    ui.label(RichText::new(origin).font(theme::ui_font(11.5)).color(p.muted));
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
            .color(p.muted),
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
            .color(p.muted),
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
                    .color(p.muted),
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
                                ui.label(RichText::new("Added").color(p.muted));
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
                ui.colored_label(p.red_letter, e);
            }
            None => {}
        }
        if let Some(m) = &self.message {
            ui.add_space(6.0);
            ui.label(RichText::new(m).color(p.accent));
        }
    }
}

fn reading(ui: &mut Ui, settings: &mut Settings, actions: &mut Vec<Action>) {
    let p = settings.theme.palette();
    widgets::caption(ui, "Theme");
    ui.horizontal(|ui| {
        for t in Theme::ALL {
            if ui.selectable_label(settings.theme == t, t.label()).clicked() {
                actions.push(Action::SetTheme(t));
            }
        }
    });
    ui.add_space(10.0);
    widgets::caption(ui, "Text");
    let typo = &mut settings.typo;
    egui::Grid::new("typo").num_columns(2).spacing(vec2(16.0, 10.0)).show(ui, |ui| {
        ui.label("Size");
        ui.add(egui::Slider::new(&mut typo.size, 12.0..=34.0).step_by(1.0).suffix(" pt"));
        ui.end_row();
        ui.label("Line spacing");
        ui.add(egui::Slider::new(&mut typo.line_height, 1.3..=2.1).step_by(0.02));
        ui.end_row();
        ui.label("Line length");
        ui.add(egui::Slider::new(&mut typo.measure, 24.0..=52.0).step_by(1.0).suffix(" em"));
        ui.end_row();
        ui.label("Layout");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut typo.mode, Mode::Paragraph, "Paragraphs");
            ui.selectable_value(&mut typo.mode, Mode::VersePerLine, "Verse per line");
        });
        ui.end_row();
        ui.label("Show");
        ui.vertical(|ui| {
            ui.checkbox(&mut typo.red_letters, "Words of Christ in red");
            ui.checkbox(&mut typo.verse_numbers, "Verse numbers");
            ui.checkbox(&mut typo.headings, "Section headings");
            ui.checkbox(&mut typo.justify, "Justify paragraphs");
        });
        ui.end_row();
    });
    ui.add_space(6.0);
    if ui.button("Reset text settings").clicked() {
        *typo = Typography::default();
    }
    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "Literata (reading) and Inter (interface) are bundled under the SIL Open Font License.",
        )
        .color(p.muted)
        .font(theme::ui_font(11.5)),
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
            ui.painter().rect_filled(r, egui::CornerRadius::same(8), p.page);
            let stroke = if settings.theme == t {
                egui::Stroke::new(2.0, p.accent)
            } else {
                egui::Stroke::new(1.0, p.border)
            };
            ui.painter().rect_stroke(r, egui::CornerRadius::same(8), stroke, egui::StrokeKind::Inside);
            ui.painter().text(
                r.center(),
                Align2::CENTER_CENTER,
                t.label(),
                egui::FontId::new(13.0, theme::family(theme::SERIF)),
                p.text,
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
