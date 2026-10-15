//! Application state and the per-frame UI orchestration.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use anyhow::Result;
use bible_core::books::{BookId, ChapterIdx, VerseRef};
use bible_core::library::Library;
use bible_core::study::StudyStore;
use eframe::egui::{self, Align2, CornerRadius, Frame, Id, Key, Margin, Modifiers, Sense, Stroke, Ui, vec2};
use serde::{Deserialize, Serialize};

use crate::actions::{Action, ExportKind, SettingsTab, SidebarTab};
use crate::palette::CommandPalette;
use crate::panel::{PanelMode, VersePanel};
use crate::reader::layout::{Mode, Typography};
use crate::reader::{Env, Reader};
use crate::settings_ui::SettingsUi;
use crate::sidebar::Sidebar;
use crate::theme::{self, Theme};
use crate::widgets;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub translation: String,
    pub parallel: Vec<String>,
    pub parallel_on: bool,
    pub theme: Theme,
    pub typo: Typography,
    pub sidebar_open: bool,
    pub sidebar_tab: SidebarTab,
    pub chapter: u16,
    pub verse: u8,
    pub whole_words: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            translation: bible_core::bundled::DEFAULT_ID.into(),
            parallel: vec!["web".into()],
            parallel_on: false,
            theme: Theme::Light,
            typo: Typography::default(),
            sidebar_open: false,
            sidebar_tab: SidebarTab::Books,
            // John 1 is a friendlier first page than Genesis 1.
            chapter: ChapterIdx::new(BookId(42), 1).unwrap().0,
            verse: 1,
            whole_words: false,
        }
    }
}

impl Settings {
    pub fn load(study: &StudyStore) -> Self {
        study.setting("app").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self, study: &StudyStore) {
        if let Ok(s) = serde_json::to_string(self) {
            let _ = study.set_setting("app", &s);
        }
    }
}

/// Everything loaded before the first frame (done on a background thread while the window opens).
pub struct Init {
    pub data_dir: PathBuf,
    pub study: StudyStore,
    pub library: Library,
    pub settings: Settings,
    pub elapsed: Duration,
}

pub fn initialize(data_dir: PathBuf, notify: Arc<dyn Fn() + Send + Sync>) -> Result<Init> {
    let t = Instant::now();
    let study = StudyStore::open(&data_dir.join("study.sqlite"))?;
    let mut settings = Settings::load(&study);
    let library = Library::open(&data_dir, notify);
    if !library.contains(&settings.translation) {
        settings.translation = bible_core::bundled::DEFAULT_ID.into();
    }
    // Decode the translation being read now; the rest warm up after the first frame.
    let _ = library.load(&settings.translation);
    Ok(Init { data_dir, study, library, settings, elapsed: t.elapsed() })
}

/// Back/forward navigation history.
#[derive(Default)]
pub struct History {
    back: Vec<(ChapterIdx, u8)>,
    forward: Vec<(ChapterIdx, u8)>,
}

impl History {
    pub fn push(&mut self, here: (ChapterIdx, u8)) {
        if self.back.last() != Some(&here) {
            self.back.push(here);
            if self.back.len() > 200 {
                self.back.remove(0);
            }
        }
        self.forward.clear();
    }

    pub fn back(&mut self, here: (ChapterIdx, u8)) -> Option<(ChapterIdx, u8)> {
        let to = self.back.pop()?;
        self.forward.push(here);
        Some(to)
    }

    pub fn forward(&mut self, here: (ChapterIdx, u8)) -> Option<(ChapterIdx, u8)> {
        let to = self.forward.pop()?;
        self.back.push(here);
        Some(to)
    }

    pub fn can_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    pub fn recent(&self) -> impl Iterator<Item = &(ChapterIdx, u8)> {
        self.back.iter().rev()
    }
}

pub enum TaskResult {
    Imported(Box<Result<bible_core::import::Imported, String>>),
    Bibles(Result<Vec<bible_core::online::apibible::RemoteBible>, String>),
}

/// `BIBLE_RS_SCROLL_PROBE`: scrolls continuously and records how long each frame's UI takes.
pub struct ScrollProbe {
    pub frames: Vec<f64>,
    pub intervals: Vec<f64>,
    pub last: Option<Instant>,
    pub start_chapter: ChapterIdx,
    pub started: Option<Instant>,
}

pub struct Startup {
    pub t0: Instant,
    pub probe: bool,
    pub init: Duration,
    pub first_ui: Option<Duration>,
    pub frames: u32,
}

pub struct BibleApp {
    pub data_dir: PathBuf,
    pub library: Library,
    pub study: StudyStore,
    pub settings: Settings,
    pub reader: Reader,
    pub history: History,
    pub sidebar: Sidebar,
    palette: Option<CommandPalette>,
    pub panel: Option<VersePanel>,
    settings_ui: Option<SettingsUi>,
    shortcuts_open: bool,
    toasts: Vec<(String, f64)>,
    online_gen: Arc<AtomicU64>,
    pub task_tx: Sender<TaskResult>,
    task_rx: Receiver<TaskResult>,
    pub busy: Option<String>,
    startup: Startup,
    /// Serialized settings as last written, and when.
    saved: (String, f64),
    warmed: bool,
    scroll_probe: Option<ScrollProbe>,
    chrome: Chrome,
    /// Where the leading toolbar group was drawn this frame.
    leading: egui::Rect,
    /// The toolbar's button groups, this frame.
    #[cfg(target_os = "macos")]
    toolbar_items: Vec<egui::Rect>,
    #[cfg(target_os = "macos")]
    no_drag: crate::macos_window::NoDragAreas,
    #[cfg(target_os = "macos")]
    menu: Option<crate::macos_menu::AppMenu>,
}

/// Toolbar geometry for the current frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chrome {
    pub height: f32,
    /// Where the first toolbar item starts.
    pub leading: f32,
}

/// On macOS the toolbar follows the window's traffic lights: its centre line is theirs and its
/// first item sits a fixed gap after them. Full screen hides them, so the toolbar starts at the edge.
#[cfg(target_os = "macos")]
fn chrome(frame: &eframe::Frame, ctx: &egui::Context) -> Chrome {
    use theme::metrics::{GROUP_GAP, LEADING_INSET, TOOLBAR_HEIGHT};
    let zoom = ctx.zoom_factor();
    match crate::macos_window::traffic_lights(frame) {
        Some(r) => Chrome {
            height: (2.0 * r.center().y / zoom).clamp(28.0, 2.0 * TOOLBAR_HEIGHT),
            leading: r.right() / zoom + GROUP_GAP,
        },
        None => Chrome { height: TOOLBAR_HEIGHT, leading: LEADING_INSET },
    }
}

#[cfg(not(target_os = "macos"))]
fn chrome(_frame: &eframe::Frame, _ctx: &egui::Context) -> Chrome {
    Chrome { height: theme::metrics::COMPACT_TOOLBAR_HEIGHT, leading: theme::metrics::LEADING_INSET }
}

/// A toolbar row from `left` to `right` (window points), centred on the toolbar's centre line.
fn toolbar_row(chrome: Chrome, left: f32, right: f32) -> egui::Rect {
    let h = theme::metrics::CAPSULE_HEIGHT;
    egui::Rect::from_x_y_ranges(left..=right, chrome.height / 2.0 - h / 2.0..=chrome.height / 2.0 + h / 2.0)
}

/// Sidebar, back and forward, a fixed gap after the traffic lights: in the sidebar's top strip
/// when it is open, in the toolbar when it is closed, so they never move.
fn leading_group(
    ui: &mut Ui,
    p: &theme::Palette,
    chrome: Chrome,
    history: &History,
    actions: &mut Vec<Action>,
) -> egui::Rect {
    use theme::metrics::GROUP_GAP;
    const ITEM_GAP: f32 = 0.0;
    let r = toolbar_row(chrome, chrome.leading, chrome.leading + 200.0);
    let mut ui = ui.new_child(
        egui::UiBuilder::new().max_rect(r).layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    ui.spacing_mut().item_spacing.x = ITEM_GAP;
    if widgets::icon_button(&mut ui, p, "Sidebar (⌘B)", false, crate::icons::sidebar).clicked() {
        actions.push(Action::ToggleSidebar);
    }
    ui.add_space(GROUP_GAP - ITEM_GAP);
    ui.add_enabled_ui(history.can_back(), |ui| {
        if widgets::icon_button(ui, p, "Back (⌘[)", false, crate::icons::chevron_left).clicked() {
            actions.push(Action::Back);
        }
    });
    ui.add_enabled_ui(history.can_forward(), |ui| {
        if widgets::icon_button(ui, p, "Forward (⌘])", false, crate::icons::chevron_right).clicked() {
            actions.push(Action::Forward);
        }
    });
    ui.min_rect()
}

impl BibleApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        init: Init,
        online_gen: Arc<AtomicU64>,
        startup: Startup,
    ) -> Self {
        let Init { data_dir, study, library, settings, elapsed } = init;
        theme::apply(&cc.egui_ctx, settings.theme);
        #[cfg(target_os = "macos")]
        crate::macos_window::install_unified_toolbar(cc);
        let chapter = ChapterIdx(settings.chapter.min(ChapterIdx::LAST.0));
        let reader = Reader::new(chapter, settings.verse);
        let (task_tx, task_rx) = channel();
        let sidebar = Sidebar::new(settings.sidebar_tab);
        Self {
            data_dir,
            library,
            study,
            reader,
            history: History::default(),
            sidebar,
            palette: None,
            panel: None,
            settings_ui: None,
            shortcuts_open: false,
            toasts: Vec::new(),
            online_gen,
            task_tx,
            task_rx,
            busy: None,
            startup: Startup { init: elapsed, ..startup },
            saved: (serde_json::to_string(&settings).unwrap_or_default(), 0.0),
            warmed: false,
            scroll_probe: std::env::var_os("BIBLE_RS_SCROLL_PROBE").map(|_| ScrollProbe {
                frames: Vec::new(),
                intervals: Vec::new(),
                last: None,
                start_chapter: chapter,
                started: None,
            }),
            chrome: Chrome { height: theme::metrics::TOOLBAR_HEIGHT, leading: theme::metrics::LEADING_INSET },
            leading: egui::Rect::NOTHING,
            #[cfg(target_os = "macos")]
            toolbar_items: Vec::new(),
            #[cfg(target_os = "macos")]
            no_drag: Default::default(),
            settings,
            #[cfg(target_os = "macos")]
            menu: crate::macos_menu::AppMenu::install(),
        }
    }

    pub fn columns(&self) -> Vec<String> {
        let mut cols = vec![self.settings.translation.clone()];
        if self.settings.parallel_on {
            for id in &self.settings.parallel {
                if cols.len() < 4 && !cols.contains(id) && self.library.contains(id) {
                    cols.push(id.clone());
                }
            }
            if cols.len() == 1 {
                // Nothing chosen yet: pair with the first other translation.
                if let Some(e) = self.library.entries().iter().find(|e| e.meta.id != cols[0]) {
                    cols.push(e.meta.id.clone());
                }
            }
        }
        cols
    }

    pub fn toast(&mut self, ctx: &egui::Context, msg: impl Into<String>) {
        self.toasts.push((msg.into(), ctx.input(|i| i.time)));
    }

    fn here(&self) -> (ChapterIdx, u8) {
        self.reader.location
    }

    fn jump(&mut self, chapter: ChapterIdx, verse: Option<u8>, flash: bool) {
        let here = self.here();
        self.history.push(here);
        self.reader.jump(chapter, verse, flash);
    }

    fn poll_tasks(&mut self, ctx: &egui::Context) {
        while let Ok(r) = self.task_rx.try_recv() {
            match r {
                TaskResult::Imported(r) if r.is_ok() => {
                    let Ok(imported) = *r else { unreachable!() };
                    self.busy = None;
                    let name = imported.translation.meta.name.clone();
                    let verses = imported.translation.verse_count();
                    match self.library.install(imported) {
                        Ok(id) => {
                            self.toast(ctx, format!("Imported {name} ({verses} verses)"));
                            self.settings.translation = id;
                            self.reader.invalidate();
                        }
                        Err(e) => self.toast(ctx, format!("Couldn't save the import: {e:#}")),
                    }
                }
                TaskResult::Imported(r) => {
                    let Err(e) = *r else { unreachable!() };
                    self.busy = None;
                    self.toast(ctx, format!("Import failed: {e}"));
                }
                TaskResult::Bibles(r) => {
                    if let Some(s) = &mut self.settings_ui {
                        s.bibles_loaded(r);
                    }
                }
            }
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let typing = ctx.egui_wants_keyboard_input();
        let overlay = self.palette.is_some() || self.settings_ui.is_some() || self.shortcuts_open;
        ctx.input_mut(|i| {
            let cmd = Modifiers::COMMAND;
            let cmd_shift = Modifiers::COMMAND | Modifiers::SHIFT;
            let mut key = |m: Modifiers, k: Key, a: Action| {
                if i.consume_key(m, k) {
                    actions.push(a);
                }
            };
            key(cmd, Key::K, Action::OpenPalette);
            key(cmd, Key::F, Action::OpenSearch(None));
            key(cmd, Key::Comma, Action::OpenSettings(SettingsTab::Reading));
            key(cmd, Key::Slash, Action::ShowShortcuts);
            if overlay {
                return;
            }
            // Shifted combinations first: egui matches shift loosely, so ⌘L would also eat ⌘⇧L.
            key(cmd_shift, Key::L, Action::CycleTheme);
            key(cmd_shift, Key::P, Action::ToggleParallel);
            key(cmd_shift, Key::E, Action::Export(ExportKind::Json));
            key(cmd, Key::OpenBracket, Action::Back);
            key(cmd, Key::CloseBracket, Action::Forward);
            key(Modifiers::ALT, Key::ArrowLeft, Action::Back);
            key(Modifiers::ALT, Key::ArrowRight, Action::Forward);
            key(cmd, Key::Plus, Action::FontSize(1.0));
            key(cmd, Key::Equals, Action::FontSize(1.0));
            key(cmd, Key::Minus, Action::FontSize(-1.0));
            key(cmd, Key::Num0, Action::FontReset);
            key(cmd, Key::B, Action::ToggleSidebar);
            key(cmd, Key::L, Action::ToggleMode);
            key(cmd, Key::R, Action::ToggleRedLetters);
            key(cmd, Key::D, Action::ToggleBookmark);
            key(cmd, Key::E, Action::Export(ExportKind::Markdown));
            key(cmd, Key::I, Action::ImportTranslation);
            for (n, k) in [
                Key::Num1,
                Key::Num2,
                Key::Num3,
                Key::Num4,
                Key::Num5,
                Key::Num6,
                Key::Num7,
                Key::Num8,
                Key::Num9,
            ]
            .into_iter()
            .enumerate()
            {
                key(cmd, k, Action::TranslationNumber(n));
            }
            if typing {
                return;
            }
            key(cmd, Key::C, Action::Copy);
            key(Modifiers::NONE, Key::ArrowLeft, Action::PrevChapter);
            key(Modifiers::NONE, Key::ArrowRight, Action::NextChapter);
            key(Modifiers::NONE, Key::ArrowDown, Action::ScrollLines(3.0));
            key(Modifiers::NONE, Key::ArrowUp, Action::ScrollLines(-3.0));
            key(Modifiers::NONE, Key::PageDown, Action::ScrollPages(1.0));
            key(Modifiers::NONE, Key::PageUp, Action::ScrollPages(-1.0));
            key(Modifiers::SHIFT, Key::Space, Action::ScrollPages(-1.0));
            key(Modifiers::NONE, Key::Space, Action::ScrollPages(1.0));
            key(Modifiers::NONE, Key::J, Action::SelectNext);
            key(Modifiers::NONE, Key::K, Action::SelectPrev);
            key(Modifiers::NONE, Key::Escape, Action::ClearSelection);
            key(Modifiers::NONE, Key::N, Action::EditSelectedNote);
            key(Modifiers::NONE, Key::B, Action::ToggleBookmark);
            key(Modifiers::NONE, Key::C, Action::Compare);
            key(Modifiers::NONE, Key::Slash, Action::OpenSearch(None));
            key(Modifiers::SHIFT, Key::Questionmark, Action::ShowShortcuts);
            key(Modifiers::NONE, Key::Questionmark, Action::ShowShortcuts);
            for (n, k) in
                [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6].into_iter().enumerate()
            {
                key(Modifiers::NONE, k, Action::Highlight(Some(bible_core::study::HighlightColor::ALL[n])));
            }
            key(Modifiers::NONE, Key::Num0, Action::Highlight(None));
            key(Modifiers::NONE, Key::Backspace, Action::Highlight(None));
        });
    }

    fn apply(&mut self, ctx: &egui::Context, actions: Vec<Action>) {
        for a in actions {
            self.apply_one(ctx, a);
        }
    }

    fn selection_or_location(&self) -> Vec<VerseRef> {
        let sel = self.reader.selected_refs();
        if sel.is_empty() {
            let (c, v) = self.here();
            vec![VerseRef::new(c.book(), c.chapter(), v)]
        } else {
            sel
        }
    }

    fn apply_one(&mut self, ctx: &egui::Context, a: Action) {
        match a {
            Action::Jump { chapter, verse, flash } => {
                self.jump(chapter, verse, flash);
                // Landing on a specific verse selects it, so N/B/C/1–6 act on it right away.
                if let (Some(v), true) = (verse, flash) {
                    self.reader.set_selection(Some(VerseRef::new(chapter.book(), chapter.chapter(), v)));
                }
            }
            Action::Back => {
                if let Some((c, v)) = self.history.back(self.here()) {
                    self.reader.jump(c, Some(v), false);
                }
            }
            Action::Forward => {
                if let Some((c, v)) = self.history.forward(self.here()) {
                    self.reader.jump(c, Some(v), false);
                }
            }
            Action::PrevChapter => {
                let (c, _) = self.here();
                // Within a chapter, ← first returns to its start.
                let target = if self.reader.anchor.chapter == c && self.reader.anchor.offset > 40.0 {
                    Some(c)
                } else {
                    c.prev()
                };
                if let Some(t) = target {
                    self.reader.jump(t, None, false);
                }
            }
            Action::NextChapter => {
                if let Some(n) = self.here().0.next() {
                    self.reader.jump(n, None, false);
                }
            }
            Action::SetTranslation(id) => {
                if self.library.contains(&id) {
                    self.settings.translation = id.clone();
                    self.settings.parallel.retain(|p| *p != id);
                    self.library.warm(&[id]);
                    self.sidebar.invalidate_search();
                }
            }
            Action::TranslationNumber(n) => {
                if let Some(e) = self.library.entries().get(n) {
                    let id = e.meta.id.clone();
                    self.apply_one(ctx, Action::SetTranslation(id));
                }
            }
            Action::ToggleParallel => {
                self.settings.parallel_on = !self.settings.parallel_on;
                if self.settings.parallel_on {
                    let cols = self.columns();
                    self.library.warm(&cols);
                }
            }
            Action::AddParallel(id) => {
                if id != self.settings.translation && !self.settings.parallel.contains(&id) {
                    self.settings.parallel.push(id.clone());
                }
                self.settings.parallel_on = true;
                self.library.warm(&[id]);
            }
            Action::RemoveParallel(id) => {
                self.settings.parallel.retain(|p| *p != id);
            }
            Action::Highlight(color) => {
                let refs = self.reader.selected_refs();
                if refs.is_empty() {
                    return;
                }
                for r in &refs {
                    if let Err(e) = self.study.set_highlight(*r, color) {
                        self.toast(ctx, format!("Couldn't save highlight: {e:#}"));
                        break;
                    }
                }
            }
            Action::EditNote(v) => {
                self.reader.set_selection(Some(v));
                self.panel = Some(VersePanel::new(v, PanelMode::Note, &self.study));
            }
            Action::EditSelectedNote => {
                let v = self.selection_or_location()[0];
                self.reader.set_selection(Some(v));
                self.panel = Some(VersePanel::new(v, PanelMode::Note, &self.study));
            }
            Action::ToggleBookmark => {
                let v = self.selection_or_location()[0];
                match self.study.toggle_bookmark(v) {
                    Ok(true) => self.toast(ctx, format!("Bookmarked {v}")),
                    Ok(false) => self.toast(ctx, format!("Removed bookmark {v}")),
                    Err(e) => self.toast(ctx, format!("Couldn't save bookmark: {e:#}")),
                }
            }
            Action::RemoveBookmark(v) => {
                if let Err(e) = self.study.set_bookmark(v, None) {
                    self.toast(ctx, format!("{e:#}"));
                }
            }
            Action::RemoveHighlight(v) => {
                if let Err(e) = self.study.set_highlight(v, None) {
                    self.toast(ctx, format!("{e:#}"));
                }
            }
            Action::RemoveNote(v) => {
                if let Err(e) = self.study.set_note(v, "") {
                    self.toast(ctx, format!("{e:#}"));
                }
                if self.panel.as_ref().is_some_and(|p| p.verse == v) {
                    self.panel = None;
                }
            }
            Action::Copy => {
                let refs = self.reader.selected_refs();
                if refs.is_empty() {
                    return;
                }
                if let Ok(t) = self.library.load(&self.settings.translation)
                    && let Some(text) = bible_core::copy::format_verses(&t, &refs)
                {
                    ctx.copy_text(text);
                    self.toast(ctx, "Copied with reference");
                }
            }
            Action::Compare => {
                let v = self.selection_or_location()[0];
                self.reader.set_selection(Some(v));
                self.panel = Some(VersePanel::new(v, PanelMode::Compare, &self.study));
                let ids: Vec<String> = self.library.entries().iter().map(|e| e.meta.id.clone()).collect();
                self.library.warm(&ids);
            }
            Action::ClearSelection => {
                if self.reader.selection.is_some() {
                    self.reader.selection = None;
                } else {
                    self.panel = None;
                }
            }
            Action::SelectNext => {
                let id = self.settings.translation.clone();
                self.reader.step_selection(&self.library, &id, 1);
            }
            Action::SelectPrev => {
                let id = self.settings.translation.clone();
                self.reader.step_selection(&self.library, &id, -1);
            }
            Action::OpenPalette => self.palette = Some(CommandPalette::default()),
            Action::OpenSearch(q) => {
                self.settings.sidebar_open = true;
                self.settings.sidebar_tab = SidebarTab::Search;
                self.sidebar.focus_search(q);
            }
            Action::ToggleSidebar => self.settings.sidebar_open = !self.settings.sidebar_open,
            Action::SidebarTab(t) => {
                self.settings.sidebar_open = true;
                self.settings.sidebar_tab = t;
            }
            Action::OpenSettings(tab) => self.settings_ui = Some(SettingsUi::new(tab, &self.library)),
            Action::FontSize(d) => self.settings.typo.size = (self.settings.typo.size + d).clamp(12.0, 34.0),
            Action::FontReset => self.settings.typo.size = Typography::default().size,
            Action::SetTheme(t) => {
                self.settings.theme = t;
                theme::apply(ctx, t);
            }
            Action::CycleTheme => {
                let t = self.settings.theme.next();
                self.apply_one(ctx, Action::SetTheme(t));
            }
            Action::ToggleMode => {
                self.settings.typo.mode = if self.settings.typo.mode == Mode::Paragraph {
                    Mode::VersePerLine
                } else {
                    Mode::Paragraph
                };
            }
            Action::ToggleRedLetters => self.settings.typo.red_letters = !self.settings.typo.red_letters,
            Action::Export(kind) => self.export(ctx, kind),
            Action::ImportStudy => self.import_study(ctx),
            Action::ImportTranslation => self.pick_import(None),
            Action::ShowShortcuts => self.shortcuts_open = !self.shortcuts_open,
            Action::ScrollLines(n) => {
                let lh = self.settings.typo.size * self.settings.typo.line_height;
                self.reader.scroll_by(n * lh);
            }
            Action::ScrollPages(n) => {
                let page = self.reader.page();
                self.reader.scroll_by(n * page);
            }
            Action::Toast(m) => self.toast(ctx, m),
            Action::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        }
    }

    /// Opens a file dialog and imports a translation on a background thread.
    pub fn pick_import(&mut self, unlock_key: Option<String>) {
        let tx = self.task_tx.clone();
        self.busy = Some("Choose a file to import…".into());
        let picked = rfd::FileDialog::new()
            .set_title("Import a translation (USFM, OSIS, Zefania, SWORD)")
            .add_filter("Bible files", &["xml", "osis", "usfm", "sfm", "zip", "conf", "txt"])
            .pick_file();
        let Some(path) = picked else {
            self.busy = None;
            return;
        };
        self.start_import(path, unlock_key, tx);
    }

    pub fn start_import(&mut self, path: PathBuf, unlock_key: Option<String>, tx: Sender<TaskResult>) {
        self.busy =
            Some(format!("Importing {}…", path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()));
        std::thread::spawn(move || {
            let opts = bible_core::import::ImportOptions { unlock_key, ..Default::default() };
            let r = bible_core::import::import_path(&path, &opts).map_err(|e| format!("{e:#}"));
            let _ = tx.send(TaskResult::Imported(Box::new(r)));
        });
    }

    fn export(&mut self, ctx: &egui::Context, kind: ExportKind) {
        let (ext, label) = match kind {
            ExportKind::Json => ("json", "JSON"),
            ExportKind::Markdown => ("md", "Markdown"),
        };
        let Some(path) = rfd::FileDialog::new()
            .set_title(format!("Export highlights, notes and bookmarks as {label}"))
            .set_file_name(format!("bible-notes.{ext}"))
            .save_file()
        else {
            return;
        };
        match self.export_to(&path, kind) {
            Ok(()) => self.toast(ctx, format!("Exported to {}", path.display())),
            Err(e) => self.toast(ctx, format!("Export failed: {e:#}")),
        }
    }

    pub fn export_to(&self, path: &std::path::Path, kind: ExportKind) -> Result<()> {
        let t = self.library.load(&self.settings.translation)?;
        let text = |v: VerseRef| t.find(v).map(|v| v.plain_text());
        let abbrev = t.meta.abbrev.clone();
        let body = match kind {
            ExportKind::Json => self.study.export_json(&abbrev, text)?,
            ExportKind::Markdown => self.study.export_markdown(&abbrev, text),
        };
        std::fs::write(path, body)?;
        Ok(())
    }

    fn import_study(&mut self, ctx: &egui::Context) {
        let Some(path) = rfd::FileDialog::new().add_filter("bible-rs export", &["json"]).pick_file() else {
            return;
        };
        let r = std::fs::read_to_string(&path)
            .map_err(anyhow::Error::from)
            .and_then(|s| self.study.import_json(&s));
        match r {
            Ok((h, n, b)) => self.toast(ctx, format!("Imported {h} highlights, {n} notes, {b} bookmarks")),
            Err(e) => self.toast(ctx, format!("Import failed: {e:#}")),
        }
    }

    /// Saves settings (including the reading position) when they change, at most every 1.5 s
    /// so scrolling never waits on the database.
    fn persist(&mut self, ctx: &egui::Context, force: bool) {
        let now = ctx.input(|i| i.time);
        self.settings.chapter = self.reader.location.0.0;
        self.settings.verse = self.reader.location.1;
        let json = serde_json::to_string(&self.settings).unwrap_or_default();
        if json == self.saved.0 {
            return;
        }
        if force || now - self.saved.1 > 1.5 {
            let _ = self.study.set_setting("app", &json);
            self.saved = (json, now);
        } else {
            ctx.request_repaint_after(Duration::from_millis(1600));
        }
    }

    /// The toolbar over the reading area: location and translation in the middle, view buttons at
    /// the end, and the leading group too when the sidebar (which otherwise holds it) is closed.
    fn toolbar(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        use theme::metrics::{GROUP_GAP, TRAILING_INSET as EDGE};
        const ITEM_GAP: f32 = 0.0;
        let p = self.settings.theme.palette();
        let bar = ui.max_rect();
        ui.painter().rect_filled(bar, 0.0, p.paper);
        // The hairline is the bar's last point, so the bar's centre stays the traffic lights'.
        let y = bar.bottom() - 0.5;
        ui.painter().line_segment(
            [egui::pos2(bar.left(), y), egui::pos2(bar.right(), y)],
            Stroke::new(1.0, p.separator),
        );
        let row = |left: f32, right: f32| toolbar_row(self.chrome, left, right);
        let builder = |r: egui::Rect, layout: egui::Layout| egui::UiBuilder::new().max_rect(r).layout(layout);
        if !self.settings.sidebar_open {
            self.leading = leading_group(ui, p, self.chrome, &self.history, actions);
        }
        let trail = ui.scope_builder(
            builder(row(bar.left(), bar.right() - EDGE), egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = ITEM_GAP;
                let aa = widgets::icon_button(ui, p, "Text & theme", false, crate::icons::text_size);
                egui::Popup::menu(&aa).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(
                    |ui| {
                        ui.set_min_width(250.0);
                        crate::settings_ui::quick_text_settings(ui, &mut self.settings, actions);
                    },
                );
                if widgets::icon_button(
                    ui,
                    p,
                    "Parallel translations (⌘⇧P)",
                    self.settings.parallel_on,
                    crate::icons::columns,
                )
                .clicked()
                {
                    actions.push(Action::ToggleParallel);
                }
                if widgets::icon_button(ui, p, "Search (⌘F)", false, crate::icons::search).clicked() {
                    actions.push(Action::OpenSearch(None));
                }
            },
        );

        // Location and translation, centred in the window unless that would collide.
        let (c, _) = self.reader.location;
        let tmeta = self.library.get(&self.settings.translation).map(|e| e.meta.clone());
        let title = format!(
            "{} {}",
            tmeta
                .as_ref()
                .map(|m| m.ref_name(c.book()).to_string())
                .unwrap_or_else(|| c.book().ref_name().into()),
            c.chapter()
        );
        let abbrev = tmeta.as_ref().map(|m| m.abbrev.clone()).unwrap_or_default();
        let width = widgets::pill_width(ui, &title) + ITEM_GAP + widgets::pill_width(ui, &abbrev);
        let lo = if self.settings.sidebar_open { bar.left() } else { self.leading.right() } + GROUP_GAP;
        let hi = trail.response.rect.left() - GROUP_GAP - width;
        let left = (bar.center().x - width / 2.0).min(hi).max(lo).round();
        let centre = ui.scope_builder(
            builder(row(left, left + width), egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = ITEM_GAP;
                let loc = widgets::pill(ui, p, &title, "Jump to a book or chapter (⌘K)");
                crate::sidebar::book_picker_popup(
                    ui,
                    &loc,
                    &self.library,
                    &self.settings.translation,
                    actions,
                );
                let tr = widgets::pill(ui, p, &abbrev, "Translation (⌘1…⌘9)");
                egui::Popup::menu(&tr).show(|ui| {
                    ui.set_min_width(300.0);
                    translation_menu(ui, &self.library, &self.settings, actions);
                });
            },
        );

        // Empty toolbar space moves the window and double-clicks to zoom, like a title bar;
        // the buttons don't.
        #[cfg(target_os = "macos")]
        {
            self.toolbar_items = vec![self.leading, centre.response.rect, trail.response.rect];
        }
        #[cfg(not(target_os = "macos"))]
        let _ = centre;
    }

    fn selection_bar(&mut self, ui: &mut Ui, reader_rect: egui::Rect, actions: &mut Vec<Action>) {
        let refs = self.reader.selected_refs();
        let (Some(first), Some(last)) = (refs.first().copied(), refs.last().copied()) else { return };
        let p = self.settings.theme.palette();
        let label = bible_core::reference::format_range(first, last);
        let center = egui::pos2(reader_rect.center().x, reader_rect.bottom() - 34.0);
        egui::Area::new(Id::new("selection-bar"))
            .order(egui::Order::Foreground)
            .fixed_pos(center)
            .pivot(Align2::CENTER_CENTER)
            .show(ui.ctx(), |ui| {
                Frame::new()
                    .fill(p.surface)
                    .stroke(Stroke::new(1.0, p.separator))
                    .corner_radius(CornerRadius::same(20))
                    .inner_margin(Margin::symmetric(14, 6))
                    .shadow(ui.visuals().popup_shadow)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(label).font(theme::ui_medium(13.0)).color(p.ink));
                            ui.add_space(6.0);
                            for (i, c) in bible_core::study::HighlightColor::ALL.into_iter().enumerate() {
                                let (r, resp) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::click());
                                let on = refs.iter().all(|v| self.study.highlight(*v) == Some(c));
                                ui.painter().circle_filled(r.center(), 7.5, p.swatch(c));
                                if on || resp.hovered() {
                                    ui.painter().circle_stroke(r.center(), 9.5, Stroke::new(1.5, p.ink));
                                }
                                if resp.on_hover_text(format!("Highlight {} ({})", c.name(), i + 1)).clicked()
                                {
                                    actions.push(Action::Highlight(if on { None } else { Some(c) }));
                                }
                            }
                            ui.add_space(4.0);
                            for (label, tip, a) in [
                                ("Note", "Add or edit a note (N)", Action::EditSelectedNote),
                                ("Bookmark", "Bookmark (B)", Action::ToggleBookmark),
                                ("Copy", "Copy with reference (⌘C)", Action::Copy),
                                ("Compare", "Compare translations (C)", Action::Compare),
                            ] {
                                if ui.add(egui::Button::new(label).frame(false)).on_hover_text(tip).clicked()
                                {
                                    actions.push(a);
                                }
                            }
                            if widgets::icon_button(
                                ui,
                                p,
                                "Clear selection (Esc)",
                                false,
                                crate::icons::close,
                            )
                            .clicked()
                            {
                                actions.push(Action::ClearSelection);
                            }
                        });
                    });
            });
    }

    fn toasts(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        self.toasts.retain(|(_, t)| now - t < 3.0);
        let p = self.settings.theme.palette();
        let screen = ctx.content_rect();
        let mut y = screen.bottom() - 18.0;
        let mut msgs: Vec<String> = self.toasts.iter().map(|(m, _)| m.clone()).collect();
        if let Some(b) = &self.busy {
            msgs.push(b.clone());
        }
        for (i, m) in msgs.iter().enumerate().rev() {
            let resp = egui::Area::new(Id::new(("toast", i)))
                .order(egui::Order::Tooltip)
                .fixed_pos(egui::pos2(screen.right() - 18.0, y))
                .pivot(Align2::RIGHT_BOTTOM)
                .interactable(false)
                .show(ctx, |ui| {
                    Frame::new()
                        .fill(p.surface)
                        .stroke(Stroke::new(1.0, p.separator))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::symmetric(12, 8))
                        .shadow(ui.visuals().popup_shadow)
                        .show(ui, |ui| {
                            ui.set_max_width(420.0);
                            ui.label(egui::RichText::new(m).color(p.ink));
                        });
                });
            y -= resp.response.rect.height() + 8.0;
        }
        if !self.toasts.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }

    fn shortcuts_help(&mut self, ctx: &egui::Context) {
        if !self.shortcuts_open {
            return;
        }
        let mut open = true;
        let p = self.settings.theme.palette();
        egui::Window::new("Keyboard shortcuts")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .frame(Frame::window(&ctx.style_of(ctx.theme())).fill(p.surface))
            .show(ctx, |ui| {
                let groups: &[(&str, &[(&str, &str)])] = &[
                    (
                        "Navigate",
                        &[
                            ("⌘K", "Jump to a reference (\"jn 3:16\", \"Rom 8\")"),
                            ("← / →", "Previous / next chapter"),
                            ("↑ ↓ Space", "Scroll"),
                            ("⌘[ / ⌘]", "Back / forward"),
                            ("⌘F or /", "Search"),
                            ("⌘B", "Books & study sidebar"),
                        ],
                    ),
                    (
                        "Verses",
                        &[
                            ("Click, ⇧-click", "Select a verse / range"),
                            ("J / K", "Select next / previous verse"),
                            ("1–6, 0", "Highlight colour, remove"),
                            ("N", "Note"),
                            ("B or ⌘D", "Bookmark"),
                            ("⌘C", "Copy with reference"),
                            ("C", "Compare translations"),
                        ],
                    ),
                    (
                        "View",
                        &[
                            ("⌘1…⌘9", "Switch translation"),
                            ("⌘⇧P", "Parallel translations"),
                            ("⌘+ / ⌘− / ⌘0", "Text size"),
                            ("⌘L", "Paragraphs / verse per line"),
                            ("⌘R", "Red letters"),
                            ("⌘⇧L", "Light / sepia / dark"),
                            ("⌘,", "Settings"),
                        ],
                    ),
                ];
                ui.horizontal_top(|ui| {
                    for (title, rows) in groups {
                        ui.vertical(|ui| {
                            ui.set_width(250.0);
                            ui.label(
                                egui::RichText::new(*title).font(theme::ui_medium(13.0)).color(p.accent),
                            );
                            ui.add_space(4.0);
                            egui::Grid::new(*title).num_columns(2).spacing(vec2(12.0, 6.0)).show(ui, |ui| {
                                for (k, d) in *rows {
                                    ui.label(egui::RichText::new(*k).font(theme::ui_medium(12.5)));
                                    ui.label(egui::RichText::new(*d).color(p.ink_2));
                                    ui.end_row();
                                }
                            });
                        });
                    }
                });
            });
        if !open || ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.shortcuts_open = false;
        }
    }
}

fn translation_menu(ui: &mut Ui, library: &Library, settings: &Settings, actions: &mut Vec<Action>) {
    let p = settings.theme.palette();
    for (i, e) in library.entries().iter().enumerate() {
        let m = &e.meta;
        let current = m.id == settings.translation;
        let shortcut = if i < 9 { format!("⌘{}", i + 1) } else { String::new() };
        let resp = ui.add(
            egui::Button::selectable(current, egui::RichText::new(format!("{}  {}", m.abbrev, m.name)))
                .shortcut_text(shortcut)
                .min_size(vec2(290.0, 26.0)),
        );
        if resp.clicked() {
            actions.push(Action::SetTranslation(m.id.clone()));
            ui.close();
        }
    }
    ui.separator();
    ui.label(egui::RichText::new("Parallel view").color(p.ink_2).font(theme::ui_font(11.5)));
    for e in library.entries() {
        let m = &e.meta;
        if m.id == settings.translation {
            continue;
        }
        let on = settings.parallel_on && settings.parallel.contains(&m.id);
        let mut checked = on;
        if ui.checkbox(&mut checked, format!("{} — side by side", m.abbrev)).changed() {
            actions.push(if checked {
                Action::AddParallel(m.id.clone())
            } else {
                Action::RemoveParallel(m.id.clone())
            });
        }
    }
    ui.separator();
    if ui.button("Manage translations…").clicked() {
        actions.push(Action::OpenSettings(SettingsTab::Translations));
        ui.close();
    }
}

impl eframe::App for BibleApp {
    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        let frame_start = Instant::now();
        if self.startup.frames < 2 {
            crate::trace(self.startup.t0, &format!("frame {} start", self.startup.frames));
        }
        let ctx = ui.ctx().clone();
        if let Some(sp) = &mut self.scroll_probe {
            // ~2,200 px/s at 60 fps: a fast, continuous fling through many chapters.
            if self.startup.frames > 2 {
                self.reader.anchor.offset += 37.0;
                if let Some(last) = sp.last {
                    sp.intervals.push(last.elapsed().as_secs_f64() * 1000.0);
                }
                sp.last = Some(Instant::now());
                sp.started.get_or_insert_with(Instant::now);
            }
            ctx.request_repaint();
        }
        self.poll_tasks(&ctx);
        let mut actions = Vec::new();
        self.shortcuts(&ctx, &mut actions);
        #[cfg(target_os = "macos")]
        if let Some(menu) = &self.menu {
            menu.poll(&mut actions);
        }
        let p = self.settings.theme.palette();

        self.chrome = chrome(frame, &ctx);
        if self.settings.sidebar_open {
            // Full height, like a source list: the traffic lights and the leading toolbar group
            // sit in its top strip.
            egui::Panel::left("sidebar")
                .resizable(true)
                .default_size(300.0)
                .size_range(240.0..=480.0)
                .frame(Frame::new().fill(p.sidebar))
                .show(ui, |ui| {
                    self.leading = leading_group(ui, p, self.chrome, &self.history, &mut actions);
                    ui.add_space(self.chrome.height);
                    let tab = &mut self.settings.sidebar_tab;
                    self.sidebar.show(
                        ui,
                        tab,
                        &self.library,
                        &self.study,
                        &self.settings.translation,
                        &mut self.settings.whole_words,
                        self.settings.theme,
                        self.reader.location,
                        &mut actions,
                    );
                });
        }
        egui::Panel::top("toolbar")
            .exact_size(self.chrome.height)
            .frame(Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| {
                self.toolbar(ui, &mut actions);
            });
        if let Some(panel) = &mut self.panel {
            let mut close = false;
            egui::Panel::right("verse-panel")
                .resizable(true)
                .default_size(340.0)
                .size_range(280.0..=560.0)
                .frame(Frame::new().fill(p.sidebar))
                .show(ui, |ui| {
                    close = panel.show(ui, &self.library, &mut self.study, &self.settings, &mut actions);
                });
            if close {
                self.panel = None;
            }
        }
        let mut reader_rect = egui::Rect::NOTHING;
        let first = self.startup.frames == 0;
        egui::CentralPanel::default().frame(Frame::NONE.fill(p.paper)).show(ui, |ui| {
            if first {
                crate::trace(self.startup.t0, "toolbar laid out");
            }
            reader_rect = ui.max_rect();
            let cols = self.columns();
            let keyboard = !ctx.egui_wants_keyboard_input() && self.palette.is_none();
            let env = Env {
                library: &self.library,
                study: &self.study,
                typo: &self.settings.typo,
                theme: self.settings.theme,
                columns: &cols,
                keyboard,
                online_generation: &self.online_gen,
            };
            self.reader.show(ui, &env, &mut actions);
            if first {
                crate::trace(self.startup.t0, "reader laid out and painted");
            }
        });
        self.selection_bar(ui, reader_rect, &mut actions);

        if let Some(pal) = &mut self.palette {
            let close = pal.show(&ctx, &self.library, &self.history, &self.settings, &mut actions);
            if close {
                self.palette = None;
            }
        }
        if let Some(s) = &mut self.settings_ui {
            let tx = self.task_tx.clone();
            let mut import: Option<(PathBuf, Option<String>)> = None;
            let open = s.show(
                &ctx,
                &mut self.library,
                &mut self.settings,
                &self.study,
                &mut actions,
                &tx,
                &mut import,
            );
            if let Some((path, key)) = import {
                self.start_import(path, key, tx);
            }
            if !open {
                self.settings_ui = None;
            }
        }
        self.shortcuts_help(&ctx);
        self.toasts(&ctx);
        #[cfg(target_os = "macos")]
        {
            // Popups over the title bar aren't drag handles either.
            let bar = egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(ctx.content_rect().width(), self.chrome.height),
            );
            let mut rects = self.toolbar_items.clone();
            ctx.memory(|m| {
                for layer in m.areas().visible_layer_ids() {
                    if layer.order != egui::Order::Background
                        && let Some(r) = m.area_rect(layer.id)
                        && r.intersects(bar)
                    {
                        rects.push(r.intersect(bar));
                    }
                }
            });
            rects.sort_by(|a, b| a.left().total_cmp(&b.left()));
            self.no_drag.set(frame, &rects, ctx.zoom_factor());
        }

        let had_actions = !actions.is_empty();
        self.apply(&ctx, actions);
        if had_actions {
            ctx.request_repaint();
        }
        self.persist(&ctx, false);

        // Startup bookkeeping: warm other translations once the first frame is up.
        if self.startup.frames < 2 {
            crate::trace(self.startup.t0, &format!("frame {} built", self.startup.frames));
        }
        self.startup.frames += 1;
        let Some(first_ui) = self.startup.first_ui else {
            self.startup.first_ui = Some(self.startup.t0.elapsed());
            ctx.request_repaint();
            return;
        };
        if !self.warmed {
            self.warmed = true;
            let mut ids = self.columns();
            ids.extend(
                self.library.entries().iter().filter(|e| e.is_local()).map(|e| e.meta.id.clone()).take(3),
            );
            self.library.warm(&ids);
            if self.startup.probe {
                let presented = self.startup.t0.elapsed();
                let epoch_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs_f64() * 1000.0)
                    .unwrap_or(0.0);
                println!(
                    "{{\"init_ms\":{:.2},\"first_frame_built_ms\":{:.2},\"first_frame_presented_ms\":{:.2},\"epoch_ms\":{:.3}}}",
                    self.startup.init.as_secs_f64() * 1000.0,
                    first_ui.as_secs_f64() * 1000.0,
                    presented.as_secs_f64() * 1000.0,
                    epoch_ms
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if let Some(sp) = &mut self.scroll_probe {
            if self.startup.frames > 3 {
                sp.frames.push(frame_start.elapsed().as_secs_f64() * 1000.0);
            }
            if sp.frames.len() >= 900 {
                let stats = |v: &mut Vec<f64>| {
                    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    let pct = |p: f64| v[((v.len() as f64 - 1.0) * p).round() as usize];
                    (
                        v.iter().sum::<f64>() / v.len() as f64,
                        pct(0.5),
                        pct(0.95),
                        pct(0.99),
                        *v.last().unwrap(),
                    )
                };
                let (mean, p50, p95, p99, max) = stats(&mut sp.frames);
                let (imean, ip50, ip95, _, imax) = stats(&mut sp.intervals);
                let chapters = self.reader.anchor.chapter.0 as i32 - sp.start_chapter.0 as i32;
                println!(
                    "{{\"frames\":{},\"chapters_crossed\":{chapters},\"ui_ms\":{{\"mean\":{mean:.3},\"p50\":{p50:.3},\"p95\":{p95:.3},\"p99\":{p99:.3},\"max\":{max:.3}}},\"frame_interval_ms\":{{\"mean\":{imean:.3},\"p50\":{ip50:.3},\"p95\":{ip95:.3},\"max\":{imax:.3}}}}}",
                    sp.frames.len()
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                self.scroll_probe = None;
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.settings.chapter = self.reader.location.0.0;
        self.settings.verse = self.reader.location.1;
        self.settings.save(&self.study);
    }
}

pub fn online_counter() -> Arc<AtomicU64> {
    Arc::new(AtomicU64::new(0))
}

pub fn bump(counter: &AtomicU64) {
    counter.fetch_add(1, Ordering::Relaxed);
}
