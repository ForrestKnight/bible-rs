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
use eframe::egui::{self, Align2, Frame, Id, Key, Modifiers, Sense, Stroke, Ui, vec2};
use serde::{Deserialize, Serialize};

use crate::actions::{Action, ExportKind, SettingsTab, SidebarTab};
use crate::note_popover::NotePopover;
use crate::palette::CommandPalette;
use crate::panel::VersePanel;
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
    /// Follow the system's light or dark appearance instead of `theme`.
    pub theme_system: bool,
    pub chapter: u16,
    pub verse: u8,
    pub whole_words: bool,
    /// Appearance: the accent colour, the translation picker in the toolbar, the sidebar
    /// attached to the window's edge rather than floating, names for the highlight colours
    /// (empty for the colour's own), less motion, and the flash on a verse jumped to.
    pub accent: theme::Accent,
    pub toolbar_translation: bool,
    pub sidebar_attached: bool,
    pub highlight_names: Vec<String>,
    pub reduce_motion: bool,
    pub flash_jumps: bool,
    /// Data: what exports include (highlights, notes, bookmarks, history), and automatic
    /// backups (on, and weekly rather than daily).
    pub export_include: [bool; 4],
    pub backup_auto: bool,
    pub backup_weekly: bool,
    /// Chords chosen in Settings › Shortcuts, by row id (`keymap::ALL`).
    pub shortcuts: std::collections::BTreeMap<String, String>,
    /// The share sheet's style and options, as last used.
    pub share: crate::share_sheet::ShareOptions,
    /// Remind about the day's plan reading at 7:00 while the app is open.
    pub plan_reminder: bool,
    /// The first-launch screens are done. Settings saved before they existed count as done.
    #[serde(default = "yes")]
    pub onboarded: bool,
}

fn yes() -> bool {
    true
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
            theme_system: false,
            // John 1 is a friendlier first page than Genesis 1.
            chapter: ChapterIdx::new(BookId(42), 1).unwrap().0,
            verse: 1,
            whole_words: false,
            plan_reminder: false,
            accent: theme::Accent::Ink,
            toolbar_translation: true,
            sidebar_attached: false,
            highlight_names: Vec::new(),
            reduce_motion: false,
            flash_jumps: true,
            shortcuts: Default::default(),
            export_include: [true, true, true, false],
            backup_auto: true,
            backup_weekly: false,
            share: Default::default(),
            onboarded: false,
        }
    }
}

impl Settings {
    /// A highlight colour's name: the one given in Settings, else the colour's.
    pub fn highlight_name(&self, c: bible_core::study::HighlightColor) -> String {
        let i = bible_core::study::HighlightColor::ALL.iter().position(|x| *x == c).unwrap_or(0);
        match self.highlight_names.get(i).map(|n| n.trim()).filter(|n| !n.is_empty()) {
            Some(n) => n.to_string(),
            None => {
                let name = c.name();
                name[..1].to_uppercase() + &name[1..]
            }
        }
    }

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
    shortcuts_overlay: crate::shortcuts_overlay::ShortcutsOverlay,
    /// The compact toolbar's More menu: open it next frame (demo), and its keyboard item.
    open_more: bool,
    more_highlight: Option<usize>,
    toasts: Vec<(String, f64)>,
    /// The reading (online) translation can't load the chapter being read.
    offline: Option<crate::offline::Offline>,
    /// The main area (right of the sidebar), where toasts centre.
    main_area: egui::Rect,
    online_gen: Arc<AtomicU64>,
    pub task_tx: Sender<TaskResult>,
    task_rx: Receiver<TaskResult>,
    pub busy: Option<String>,
    startup: Startup,
    /// Serialized settings as last written, and when.
    saved: (String, f64),
    warmed: bool,
    scroll_probe: Option<ScrollProbe>,
    sidebar_probe: Option<crate::probe::SidebarProbe>,
    reading: crate::reading::Tracker,
    verse_menu: Option<crate::verse_menu::VerseMenu>,
    note_popover: Option<NotePopover>,
    /// The first-launch screens, until they are done.
    onboarding: Option<crate::onboarding::Onboarding>,
    /// Compare Verse as a popover on the selected verse (frame 23).
    compare: Option<crate::compare_popover::ComparePopover>,
    /// Sharing the selected verses as an image or text (frame 22).
    share_sheet: Option<crate::share_sheet::ShareSheet>,
    /// Importing a translation (frames 13–15).
    import_sheet: Option<crate::import_sheet::ImportSheet>,
    /// Focus mode (frame 30): full screen, the chrome hidden, larger type.
    focus: bool,
    /// Screenshots: show focus mode's reveal bar as if the pointer were at the top.
    focus_reveal: bool,
    /// When an automatic backup was last considered (egui time).
    backup_checked: Option<f64>,
    /// The day the plan reminder last showed.
    reminded: Option<i64>,
    /// The page in the main area (the reader unless a library or study page is open).
    pub pages: crate::pages::Pages,
    /// `BIBLE_RS_DEMO`: steps that put the window into a given state (screenshots), one per frame.
    demo: std::collections::VecDeque<String>,
    chrome: Chrome,
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
    /// Where the first toolbar control starts when the sidebar is hidden.
    pub leading: f32,
}

/// On macOS the toolbar follows the window's traffic lights: its centre line is theirs and its
/// first control sits a fixed gap after them. Full screen hides them, so the toolbar starts at the
/// window's leading inset.
#[cfg(target_os = "macos")]
fn chrome(frame: &eframe::Frame, ctx: &egui::Context) -> Chrome {
    use theme::metrics::{AFTER_TRAFFIC_LIGHTS, LEADING_INSET, TOOLBAR_HEIGHT};
    let zoom = ctx.zoom_factor();
    match crate::macos_window::traffic_lights(frame) {
        Some(r) => Chrome {
            height: (2.0 * r.center().y / zoom).clamp(28.0, 2.0 * TOOLBAR_HEIGHT),
            leading: r.right() / zoom + AFTER_TRAFFIC_LIGHTS,
        },
        // Rendering offscreen (`examples/shot.rs`): lay out as if AppKit's lights were there.
        None if crate::offscreen() => {
            let lights = theme::titlebar::TRAFFIC_LIGHT_FIRST_CENTER_X
                - theme::titlebar::TRAFFIC_LIGHT_DIAMETER / 2.0
                + 3.0 * theme::titlebar::TRAFFIC_LIGHT_DIAMETER
                + 2.0 * theme::titlebar::TRAFFIC_LIGHT_GAP;
            Chrome { height: TOOLBAR_HEIGHT, leading: lights + AFTER_TRAFFIC_LIGHTS }
        }
        None => Chrome { height: TOOLBAR_HEIGHT, leading: LEADING_INSET },
    }
}

#[cfg(not(target_os = "macos"))]
fn chrome(_frame: &eframe::Frame, _ctx: &egui::Context) -> Chrome {
    Chrome { height: theme::metrics::COMPACT_TOOLBAR_HEIGHT, leading: theme::metrics::LEADING_INSET }
}

impl BibleApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        init: Init,
        online_gen: Arc<AtomicU64>,
        startup: Startup,
    ) -> Self {
        let Init { data_dir, study, library, settings, elapsed } = init;
        theme::set_accent(settings.accent);
        theme::apply(&cc.egui_ctx, settings.theme);
        if settings.reduce_motion {
            cc.egui_ctx.all_styles_mut(|s| s.animation_time = 0.0);
        }
        #[cfg(target_os = "macos")]
        crate::macos_window::install_unified_toolbar(cc);
        let chapter = ChapterIdx(settings.chapter.min(ChapterIdx::LAST.0));
        let reader = Reader::new(chapter, settings.verse);
        let (task_tx, task_rx) = channel();
        let sidebar = Sidebar::new(settings.sidebar_tab);
        let onboarding = (!settings.onboarded && !crate::probing()).then(|| {
            // A window opened at the reader's size (onboarding left unfinished) shrinks to fit.
            cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(crate::onboarding::SIZE));
            crate::onboarding::Onboarding::new(&cc.egui_ctx)
        });
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
            shortcuts_overlay: Default::default(),
            open_more: false,
            more_highlight: None,
            toasts: Vec::new(),
            offline: None,
            main_area: egui::Rect::NOTHING,
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
            sidebar_probe: crate::probe::SidebarProbe::from_env(),
            reading: Default::default(),
            verse_menu: None,
            note_popover: None,
            onboarding,
            import_sheet: None,
            share_sheet: None,
            compare: None,
            reminded: None,
            focus: false,
            focus_reveal: false,
            backup_checked: None,
            pages: Default::default(),
            demo: std::env::var("BIBLE_RS_DEMO")
                .map(|s| s.split(';').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect())
                .unwrap_or_default(),
            chrome: Chrome { height: theme::metrics::TOOLBAR_HEIGHT, leading: theme::metrics::LEADING_INSET },
            #[cfg(target_os = "macos")]
            toolbar_items: Vec::new(),
            #[cfg(target_os = "macos")]
            no_drag: Default::default(),
            settings,
            #[cfg(target_os = "macos")]
            menu: if crate::offscreen() { None } else { crate::macos_menu::AppMenu::install() },
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
        let flash = flash && self.settings.flash_jumps && !self.settings.reduce_motion;
        self.reader.jump(chapter, verse, flash);
    }

    fn poll_tasks(&mut self, _ctx: &egui::Context) {
        while let Ok(r) = self.task_rx.try_recv() {
            match r {
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
        // The book picker takes the single keys (arrows, return, escape) while it's open.
        let picker = crate::book_picker::is_open(ctx);
        let overlay = self.palette.is_some() || self.settings_ui.is_some() || self.shortcuts_open;
        // The rows of Settings › Shortcuts, with any chords chosen there.
        let bindings = crate::keymap::bindings(&self.settings.shortcuts);
        let anywhere = |a: &Action| matches!(a, Action::OpenPalette | Action::OpenSearch(_));
        ctx.input_mut(|i| {
            let cmd = Modifiers::COMMAND;
            let cmd_shift = Modifiers::COMMAND | Modifiers::SHIFT;
            let mut key = |m: Modifiers, k: Key, a: Action| {
                if i.consume_key(m, k) {
                    actions.push(a);
                }
            };
            for (c, a) in bindings.iter().filter(|(_, a)| anywhere(a)) {
                key(c.modifiers(), c.key, a.clone());
            }
            key(cmd, Key::Comma, Action::OpenSettings(SettingsTab::Reading));
            key(cmd, Key::Slash, Action::ShowShortcuts);
            if overlay {
                return;
            }
            // Command chords (shifted ones first: egui matches shift loosely, so ⌘L would also eat
            // ⌘⇧L); copying waits until no field has the keyboard.
            for (c, a) in
                bindings.iter().filter(|(c, a)| c.is_command() && !anywhere(a) && *a != Action::Copy)
            {
                key(c.modifiers(), c.key, a.clone());
            }
            key(cmd_shift, Key::E, Action::Export(ExportKind::Json));
            key(cmd, Key::OpenBracket, Action::Back);
            key(cmd, Key::CloseBracket, Action::Forward);
            key(Modifiers::ALT, Key::ArrowLeft, Action::Back);
            key(Modifiers::ALT, Key::ArrowRight, Action::Forward);
            key(cmd, Key::Plus, Action::FontSize(1.0));
            key(cmd, Key::Equals, Action::FontSize(1.0));
            key(cmd, Key::Minus, Action::FontSize(-1.0));
            key(cmd, Key::Num0, Action::FontReset);
            key(cmd, Key::T, Action::TextFormat);
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
            // Copying works with the book picker open; the reader's single keys wait for it.
            for (c, a) in bindings.iter().filter(|(c, a)| *a == Action::Copy || !(c.is_command() || picker)) {
                key(c.modifiers(), c.key, a.clone());
            }
            key(cmd, Key::C, Action::CopyText);
            if picker {
                return;
            }
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
                self.pages.open(crate::pages::Page::Reader);
                self.jump(chapter, verse, flash);
                // Landing on a specific verse selects it, so N/B/C/1–6 act on it right away.
                if let (Some(v), true) = (verse, flash) {
                    self.reader.set_selection(Some(VerseRef::new(chapter.book(), chapter.chapter(), v)));
                }
            }
            // From a page, Back returns to the reader.
            Action::Back if !self.pages.reader() => self.pages.open(crate::pages::Page::Reader),
            Action::Back => {
                if let Some((c, v)) = self.history.back(self.here()) {
                    self.reader.jump(c, Some(v), false);
                }
            }
            Action::Forward => {
                self.pages.open(crate::pages::Page::Reader);
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
            Action::OpenInParallel(a) => {
                self.apply_one(ctx, *a);
                if !self.settings.parallel_on {
                    self.apply_one(ctx, Action::ToggleParallel);
                }
            }
            Action::CompareVerse(v) => {
                self.reader.set_selection(Some(v));
                self.apply_one(ctx, Action::Compare);
            }
            Action::AddParallel(id) => {
                if id != self.settings.translation && !self.settings.parallel.contains(&id) {
                    self.settings.parallel.push(id.clone());
                }
                self.settings.parallel_on = true;
                self.library.warm(&[id]);
            }
            Action::ReplaceParallel(i, id) => {
                if id != self.settings.translation {
                    let mut shown: Vec<String> = self.columns().into_iter().skip(1).collect();
                    match shown.iter().position(|c| *c == id) {
                        // Already shown: swap the two columns.
                        Some(at) if i < shown.len() => shown.swap(at, i),
                        Some(_) => {}
                        None if i < shown.len() => shown[i] = id.clone(),
                        None => shown.push(id.clone()),
                    }
                    self.settings.parallel = shown;
                    self.library.warm(&[id]);
                }
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
                self.note_popover = Some(NotePopover::new(v, &self.study));
            }
            Action::EditSelectedNote => {
                let v = self.selection_or_location()[0];
                self.apply_one(ctx, Action::EditNote(v));
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
                if self.note_popover.as_ref().is_some_and(|n| n.verse == v) {
                    self.note_popover = None;
                }
            }
            Action::CopyText => {
                let refs = self.reader.selected_refs();
                if let Ok(t) = self.library.load(&self.settings.translation) {
                    let text: Vec<String> =
                        refs.iter().filter_map(|v| t.find(*v)).map(|v| v.plain_text()).collect();
                    if !text.is_empty() {
                        ctx.copy_text(text.join(" "));
                        self.toast(ctx, "Copied");
                    }
                }
            }
            Action::OpenVerseMenu(pos) => {
                if !self.reader.selected_refs().is_empty() {
                    self.verse_menu = Some(crate::verse_menu::VerseMenu::new(pos));
                }
            }
            Action::ShareImage => {
                let refs = self.reader.selected_refs();
                if !refs.is_empty() {
                    self.share_sheet = Some(crate::share_sheet::ShareSheet::new(refs));
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
                if self.columns().len() > 1 {
                    // Beside parallel columns: the Compare inspector (frame 07).
                    self.panel = Some(VersePanel::new(v));
                } else {
                    self.compare = Some(crate::compare_popover::ComparePopover::new(v));
                }
                let ids: Vec<String> = self.library.entries().iter().map(|e| e.meta.id.clone()).collect();
                self.library.warm(&ids);
            }
            Action::SearchStep(d) => self.sidebar.step_result(d),
            Action::ClearSearch => {
                self.sidebar.query.clear();
                self.reader.selection = None;
            }
            Action::ClearSelection => {
                if self.sidebar.searching() {
                    self.apply_one(ctx, Action::ClearSearch);
                } else if self.reader.selection.is_some() {
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
            Action::ImportTranslation => {
                self.import_sheet = Some(crate::import_sheet::ImportSheet::new());
            }
            Action::RemoveTranslation(id) => match self.library.remove(&id) {
                Ok(()) => {
                    if self.settings.translation == id {
                        self.settings.translation = bible_core::bundled::DEFAULT_ID.into();
                    }
                    self.settings.parallel.retain(|x| *x != id);
                    if self.pages.translations.selected.as_deref() == Some(id.as_str()) {
                        self.pages.translations.selected = None;
                    }
                    self.toast(ctx, format!("Removed {id}"));
                }
                Err(e) => self.toast(ctx, format!("{e:#}")),
            },
            Action::OpenPage(page) => self.pages.open(page),
            Action::SaveNote(v, body) => {
                if let Err(e) = self.study.set_note(v, &body) {
                    self.toast(ctx, format!("Couldn't save note: {e:#}"));
                }
            }
            Action::StartPlan(id) => {
                let today = crate::time::day_start(bible_core::study::now());
                if let Err(e) = self.study.start_plan(&id, today) {
                    self.toast(ctx, format!("{e:#}"));
                }
            }
            Action::StopPlan(id) => {
                if let Err(e) = self.study.stop_plan(&id) {
                    self.toast(ctx, format!("{e:#}"));
                }
            }
            Action::SetPlanRead { plan, day, passage, read } => {
                if let Err(e) = self.study.set_plan_read(&plan, day, passage, read) {
                    self.toast(ctx, format!("{e:#}"));
                }
            }
            Action::SetPlanReminder(on) => self.settings.plan_reminder = on,
            Action::EraseStudy => {
                // A backup first, so it can be undone.
                match self
                    .backup_now()
                    .map_err(|e| format!("{e:#}"))
                    .and_then(|path| self.study.erase_all().map(|_| path).map_err(|e| format!("{e:#}")))
                {
                    Ok(path) => {
                        self.reader.invalidate();
                        self.toast(ctx, format!("Erased. A backup was saved first: {}", path.display()));
                    }
                    Err(e) => self.toast(ctx, format!("Couldn't erase: {e}")),
                }
            }
            Action::RevealDataDir => reveal(&self.data_dir),
            Action::TextFormat => egui::Popup::open_id(ctx, Id::new(TEXT_POPUP)),
            Action::ToggleFocus => {
                self.focus = !self.focus;
                if self.focus {
                    self.pages.open(crate::pages::Page::Reader);
                }
                if !crate::offscreen() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.focus));
                }
            }
            Action::SetAccent(a) => {
                self.settings.accent = a;
                theme::set_accent(a);
                theme::apply(ctx, self.settings.theme);
            }
            Action::SetReduceMotion(on) => {
                self.settings.reduce_motion = on;
                let t = if on { 0.0 } else { egui::Style::default().animation_time };
                ctx.all_styles_mut(|s| s.animation_time = t);
            }
            Action::ClearReading(since) => {
                if let Err(e) = self.study.clear_reading(since) {
                    self.toast(ctx, format!("{e:#}"));
                }
            }
            Action::ToggleBookmarkAt(v) => {
                if let Err(e) = self.study.toggle_bookmark(v) {
                    self.toast(ctx, format!("{e:#}"));
                }
            }
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

    /// One `BIBLE_RS_DEMO` step: `jump:<reference>`, `select:<reference>`, `palette:<query>`,
    /// `search:<query>`, `scope:<ot|nt>`, `step:<n>` (search results), `offline:<reference>`, `note:<reference>`, `compare:<reference>`, `study:<bookmarks|highlights|notes>`,
    /// `settings:<reading|translations|data>`, `plans-tab:<active|browse|completed>`,
    /// `size:<width>x<height>` (the window's inner size).
    fn demo_step(&mut self, ctx: &egui::Context, step: &str, actions: &mut Vec<Action>) {
        let (cmd, arg) = step.split_once(':').unwrap_or((step, ""));
        let verse = || bible_core::reference::parse(arg).map(|r| r.verse_ref());
        match cmd {
            "jump" => {
                if let Some(r) = bible_core::reference::parse(arg) {
                    actions.push(Action::Jump {
                        chapter: r.chapter_idx(),
                        verse: r.verse,
                        flash: r.verse.is_some(),
                    });
                }
            }
            "select" => self.reader.set_selection(verse()),
            "palette" => self.palette = Some(CommandPalette::with_query(arg)),
            "search" => actions.push(Action::OpenSearch(Some(arg.to_string()))),
            "step" => actions.push(Action::SearchStep(arg.parse().unwrap_or(1))),
            "note" => actions.extend(verse().map(Action::EditNote)),
            "compare" => {
                self.reader.set_selection(verse());
                actions.push(Action::Compare);
            }
            "study" => self.sidebar.open_study(arg),
            // `offline:<reference>`: that chapter of the reading (online) translation fails to
            // load as it would without a connection.
            "offline" => {
                if let (Some(r), Some(o)) = (
                    bible_core::reference::parse(arg),
                    self.library.get(&self.settings.translation).and_then(|e| e.online()),
                ) {
                    let message = format!("{}: network error: io: failed to lookup address", o.meta.abbrev);
                    o.mark_failed(r.chapter_idx(), &message);
                }
            }
            "scope" => self.sidebar.set_scope(arg),
            // `more:<item>`: the compact toolbar's More menu, that item highlighted.
            "more" => {
                self.open_more = true;
                self.more_highlight = arg.parse().ok();
            }
            // `menu:<reference>@x,y`: select the verse and open its menu there.
            "menu" => {
                let (r, at) = arg.split_once('@').unwrap_or((arg, "600,352"));
                let (x, y) = at.split_once(',').unwrap_or(("600", "352"));
                self.reader.set_selection(bible_core::reference::parse(r).map(|r| r.verse_ref()));
                let pos = egui::pos2(x.parse().unwrap_or(600.0), y.parse().unwrap_or(352.0));
                self.verse_menu = Some(crate::verse_menu::VerseMenu::new(pos).with_first_highlighted());
            }
            "share" => actions.push(Action::ShareImage),
            "size" => {
                if let Some((w, h)) = arg.split_once('x')
                    && let (Ok(w), Ok(h)) = (w.parse::<f32>(), h.parse::<f32>())
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(w, h)));
                }
            }
            "focus" => actions.push(Action::ToggleFocus),
            "reveal" => self.focus_reveal = true,
            "shortcuts" => self.shortcuts_open = true,
            "page" => {
                if let Some(page) = crate::pages::Page::from_name(arg) {
                    actions.push(Action::OpenPage(page));
                }
            }
            "pick" => self.pages.translations.selected = Some(arg.to_string()),
            "plans-tab" => self.pages.plans.set_tab(arg),
            // `plan-day:<plan>@<day>`: a plan's day view.
            "plan-day" => {
                let (id, day) = arg.split_once('@').unwrap_or((arg, "1"));
                self.pages.plans.open = Some((id.to_string(), day.parse().unwrap_or(1)));
            }
            "note-select" => {
                if let Some(r) = bible_core::reference::parse(arg) {
                    self.pages.notes.select(r.verse_ref());
                }
            }
            // `import`, `import:<path>`, then `import-progress:<books>` or
            // `import-review:<error>|<key tried>` (empty error: import the file for real).
            "import" => {
                self.import_sheet = Some(if arg.is_empty() {
                    crate::import_sheet::ImportSheet::new()
                } else {
                    crate::import_sheet::ImportSheet::with_path(arg.into())
                });
            }
            "import-drag" => {
                if let Some(s) = &mut self.import_sheet {
                    s.demo_drag = true;
                }
            }
            "import-progress" => {
                if let Some(s) = &mut self.import_sheet {
                    s.demo_converting(arg.parse().unwrap_or(47));
                }
            }
            "import-review" => {
                if let Some(s) = &mut self.import_sheet {
                    let (err, key) = arg.split_once('|').unwrap_or((arg, ""));
                    s.demo_review(Some(err).filter(|e| !e.is_empty()), Some(key).filter(|k| !k.is_empty()));
                }
            }
            "onboarding" => {
                if let Some(o) = &mut self.onboarding {
                    o.step = arg.parse::<usize>().unwrap_or(1).saturating_sub(1);
                }
            }
            "record" => {
                if let Some(s) = &mut self.settings_ui {
                    s.record(arg);
                }
            }
            "settings" => actions.push(Action::OpenSettings(match arg {
                "translations" => SettingsTab::Translations,
                "data" => SettingsTab::Data,
                "appearance" => SettingsTab::Appearance,
                "shortcuts" => SettingsTab::Shortcuts,
                "about" => SettingsTab::About,
                _ => SettingsTab::Reading,
            })),
            _ => eprintln!("BIBLE_RS_DEMO: unknown step {step:?}"),
        }
    }

    /// Adds an import to the library, reads it, and selects it on the library page.
    fn install_import(&mut self, ctx: &egui::Context, imported: bible_core::import::Imported) {
        let name = imported.translation.meta.name.clone();
        let verses = imported.translation.verse_count();
        match self.library.install(imported) {
            Ok(id) => {
                self.toast(ctx, format!("Imported {name} ({verses} verses)"));
                self.pages.translations.selected = Some(id.clone());
                self.settings.translation = id;
                self.reader.invalidate();
            }
            Err(e) => self.toast(ctx, format!("Couldn't save the import: {e:#}")),
        }
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
        let [highlights, notes, bookmarks, history] = self.settings.export_include;
        let include = bible_core::study::Include { highlights, notes, bookmarks, history };
        self.write_export(path, kind, include)
    }

    fn write_export(
        &self,
        path: &std::path::Path,
        kind: ExportKind,
        include: bible_core::study::Include,
    ) -> Result<()> {
        let t = self.library.load(&self.settings.translation)?;
        let text = |v: VerseRef| t.find(v).map(|v| v.plain_text());
        let abbrev = t.meta.abbrev.clone();
        let body = match kind {
            ExportKind::Json => self.study.export_json_with(&abbrev, text, include)?,
            ExportKind::Markdown => self.study.export_markdown_with(&abbrev, text, include),
        };
        std::fs::write(path, body)?;
        Ok(())
    }

    /// Writes a full JSON backup into the data folder's `backups`, keeping the newest ten.
    pub fn backup_now(&self) -> Result<PathBuf> {
        let dir = backups_dir(&self.data_dir);
        std::fs::create_dir_all(&dir)?;
        let stamp = bible_core::study::rfc3339(bible_core::study::now()).replace(':', "-");
        let path = dir.join(format!("bible-rs-backup-{stamp}.json"));
        self.write_export(&path, ExportKind::Json, bible_core::study::Include::EVERYTHING)?;
        let mut files: Vec<_> = std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        files.sort();
        for old in files.iter().rev().skip(10) {
            let _ = std::fs::remove_file(old);
        }
        Ok(path)
    }

    /// An automatic backup when one is due (checked once a launch and then every hour).
    fn auto_backup(&mut self, ctx: &egui::Context) {
        if !self.settings.backup_auto || self.startup.frames < 10 {
            return;
        }
        let now = ctx.input(|i| i.time);
        if self.backup_checked.is_some_and(|t| now - t < 3600.0) {
            return;
        }
        self.backup_checked = Some(now);
        let every = if self.settings.backup_weekly { 7 * 86_400 } else { 86_400 };
        let due = last_backup(&self.data_dir).is_none_or(|(at, _)| bible_core::study::now() - at >= every);
        let (h, n, b) = self.study.counts();
        if due && h + n + b > 0 {
            let _ = self.backup_now();
        }
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
    /// The daily plan reminder: once a day from 7:00, while a plan has today's reading to do.
    fn plan_reminder(&mut self, ctx: &egui::Context) {
        if !self.settings.plan_reminder || self.startup.frames < 3 {
            return;
        }
        let now = bible_core::study::now();
        let today = crate::time::day_start(now);
        if self.reminded == Some(today) || now - today < 7 * 3600 {
            return;
        }
        self.reminded = Some(today);
        let due = self.study.plans().into_iter().filter(|p| p.finished.is_none()).find_map(|p| {
            let plan = bible_core::plans::get(&p.id)?;
            let day = ((today - p.started) / 86_400 + 1) as usize;
            let reads = self.study.plan_reads(&p.id);
            let todo = (0..plan.day(day).len()).any(|i| !reads.contains(&(day as u16, i as u8)));
            todo.then(|| {
                let name = |b: bible_core::BookId| b.ref_name().to_string();
                format!("Today's reading: {}", bible_core::plans::label(plan.day(day), name))
            })
        });
        if let Some(msg) = due {
            self.toast(ctx, msg);
        }
    }

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

    /// The toolbar over the reading area (`ds-06` in the design): back/forward (after the
    /// sidebar toggle when the sidebar is hidden), the location and translation pickers centred,
    /// view tools and the search field at the end. Every control is a glass capsule centred on
    /// the traffic lights' line; once text scrolls beneath, the bar takes the toolbar material.
    fn toolbar(&mut self, ui: &mut Ui, main: egui::Rect, material: f32, actions: &mut Vec<Action>) {
        use theme::metrics::{
            BUTTON_WIDTH, CAPSULE_HEIGHT, GROUP_GAP, LEADING_INSET, SEARCH_FIELD_WIDTH,
            SEARCH_FIELD_WIDTH_COMPACT, SIDEBAR_TOGGLE_WIDTH, TRAILING_INSET,
        };
        use widgets::CapsuleButton;
        let p = self.settings.theme.palette();
        let bar = egui::Rect::from_min_size(main.min, vec2(main.width(), self.chrome.height));
        if material > 0.0 {
            ui.painter().rect_filled(bar, 0.0, p.toolbar.gamma_multiply(material));
            ui.painter().hline(
                bar.x_range(),
                bar.bottom() - 0.5,
                Stroke::new(1.0, p.separator.gamma_multiply(material)),
            );
        }
        let cy = bar.center().y;
        let mut items = Vec::new();

        // Leading: the sidebar toggle (when the sidebar, which otherwise holds it, is hidden),
        // then back and forward.
        let sidebar = self.settings.sidebar_open;
        let mut x = if sidebar {
            bar.left() + GROUP_GAP
        } else {
            self.chrome.leading.max(bar.left() + LEADING_INSET)
        };
        if !sidebar {
            let toggle = CapsuleButton {
                tooltip: "Show sidebar (⌘B)",
                enabled: true,
                on: false,
                icon: crate::icons::sidebar,
            };
            let (r, resp) = widgets::button_capsule(ui, p, x, cy, SIDEBAR_TOGGLE_WIDTH, false, &[toggle]);
            if resp[0].clicked() {
                actions.push(Action::ToggleSidebar);
            }
            items.push(r);
            x = r.right() + GROUP_GAP;
        }
        let nav = [
            CapsuleButton {
                tooltip: "Back (⌘[)",
                // From a page, Back returns to the reader.
                enabled: self.history.can_back() || !self.pages.reader(),
                on: false,
                icon: crate::icons::chevron_left,
            },
            CapsuleButton {
                tooltip: "Forward (⌘])",
                enabled: self.history.can_forward() && self.pages.reader(),
                on: false,
                icon: crate::icons::chevron_right,
            },
        ];
        let (nav_rect, resp) = widgets::button_capsule(ui, p, x, cy, BUTTON_WIDTH, true, &nav);
        if resp[0].clicked() {
            actions.push(Action::Back);
        }
        if resp[1].clicked() {
            actions.push(Action::Forward);
        }
        items.push(nav_rect);

        // On a page: a wider field that filters it, and nothing else.
        if !self.pages.reader() {
            let w = PAGE_SEARCH_WIDTH;
            let search = egui::Rect::from_min_size(
                egui::pos2(bar.right() - TRAILING_INSET - w, cy - CAPSULE_HEIGHT / 2.0),
                vec2(w, CAPSULE_HEIGHT),
            );
            let placeholder = self.pages.page.placeholder();
            let (field, cleared) = widgets::search_field(
                ui,
                search,
                p,
                &mut self.pages.query,
                placeholder,
                Id::new("page-search"),
            );
            if cleared || (field.has_focus() && ui.input(|i| i.key_pressed(Key::Escape))) {
                self.pages.query.clear();
                field.surrender_focus();
            }
            items.push(search);
            #[cfg(target_os = "macos")]
            self.toolbar_items.extend(items);
            return;
        }

        // Trailing: view tools, then the search field. A compact window folds both into one
        // capsule, Search and More (frame 34), and shows the narrower field while searching.
        let compact = bar.width() < COMPACT_TOOLBAR;
        let focus_field = self.sidebar.take_focus();
        let show_field = !compact || focus_field || self.sidebar.searching() || self.sidebar.field_focused;
        let trailing_left;
        // What the text popover hangs from, and whether clicking it toggles the popover.
        let mut text_anchor: Option<(egui::Response, bool)> = None;
        if show_field {
            let w = if compact { SEARCH_FIELD_WIDTH_COMPACT } else { SEARCH_FIELD_WIDTH };
            let search = egui::Rect::from_min_size(
                egui::pos2(bar.right() - TRAILING_INSET - w, cy - CAPSULE_HEIGHT / 2.0),
                vec2(w, CAPSULE_HEIGHT),
            );
            trailing_left = if compact {
                search.left()
            } else {
                let tools = [
                    CapsuleButton {
                        tooltip: "Parallel translations (⌘⇧P)",
                        enabled: true,
                        on: self.settings.parallel_on,
                        icon: crate::icons::columns,
                    },
                    CapsuleButton {
                        tooltip: "Text & layout (⌘T)",
                        enabled: true,
                        on: false,
                        icon: crate::icons::text_size,
                    },
                ];
                let tools_left = search.left() - GROUP_GAP - 2.0 * BUTTON_WIDTH;
                let (tools_rect, resp) =
                    widgets::button_capsule(ui, p, tools_left, cy, BUTTON_WIDTH, false, &tools);
                if resp[0].clicked() {
                    actions.push(Action::ToggleParallel);
                }
                text_anchor = Some((resp[1].clone(), true));
                items.push(tools_rect);
                tools_rect.left()
            };
            let field_id = Id::new("toolbar-search");
            let (field, cleared) =
                widgets::search_field(ui, search, p, &mut self.sidebar.query, "Search", field_id);
            if focus_field {
                field.request_focus();
            }
            if field.changed() && self.sidebar.searching() {
                self.settings.sidebar_open = true;
            }
            if cleared || (field.has_focus() && ui.input(|i| i.key_pressed(Key::Escape))) {
                self.sidebar.query.clear();
                field.surrender_focus();
            }
            self.sidebar.field_focused = field.has_focus();
            items.push(search);
        } else {
            let buttons = [
                CapsuleButton {
                    tooltip: "Search (⌘F)",
                    enabled: true,
                    on: false,
                    icon: crate::icons::search_thin,
                },
                CapsuleButton { tooltip: "More", enabled: true, on: false, icon: crate::icons::ellipsis },
            ];
            let left = bar.right() - TRAILING_INSET - 2.0 * BUTTON_WIDTH;
            let (r, resp) = widgets::button_capsule(ui, p, left, cy, BUTTON_WIDTH, false, &buttons);
            if resp[0].clicked() {
                actions.push(Action::OpenSearch(None));
            }
            if egui::Popup::is_id_open(ui.ctx(), Id::new(MORE_POPUP)) {
                // Pressed while its menu is open.
                let b = resp[1].rect;
                ui.painter().rect_filled(b, CAPSULE_HEIGHT / 2.0, p.fill);
                let icon = egui::Rect::from_center_size(b.center(), vec2(20.0, 20.0));
                crate::icons::ellipsis(ui.painter(), icon, p.ink);
            }
            self.more_menu(ui, &resp[1], actions);
            text_anchor = Some((resp[1].clone(), false));
            trailing_left = r.left();
            items.push(r);
        }
        if let Some((anchor, toggles)) = text_anchor {
            let open = (toggles && anchor.clicked()).then_some(egui::SetOpenCommand::Toggle);
            egui::Popup::menu(&anchor)
                .id(Id::new(TEXT_POPUP))
                .open_memory(open)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    ui.set_min_width(250.0);
                    crate::settings_ui::quick_text_settings(ui, &mut self.settings, actions);
                });
        }

        // Principal: location and translation in one capsule, centred between the bar's paddings
        // unless that would collide with the groups at either end.
        let (c, _) = self.reader.location;
        let tmeta = self.library.get(&self.settings.translation).map(|e| e.meta.clone());
        // Compact windows shorten the book: "Rom 8".
        let book_name = if compact {
            c.book().info().abbrev.to_string()
        } else {
            tmeta
                .as_ref()
                .map(|m| m.ref_name(c.book()).to_string())
                .unwrap_or_else(|| c.book().ref_name().into())
        };
        let title = format!("{book_name} {}", c.chapter());
        let abbrev = if self.settings.parallel_on {
            self.columns()
                .iter()
                .filter_map(|id| self.library.get(id).map(|e| e.meta.abbrev.clone()))
                .collect::<Vec<_>>()
                .join(" · ")
        } else {
            tmeta.as_ref().map(|m| m.abbrev.clone()).unwrap_or_default()
        };
        let show_translation = self.settings.toolbar_translation;
        let book = widgets::Picker::new(ui, &title, 14.0, if show_translation { 10.0 } else { 12.0 });
        let translation = widgets::Picker::new(ui, &abbrev, 10.0, 12.0);
        let width = book.width() + if show_translation { 1.0 + translation.width() } else { 0.0 };
        let pad_left = if sidebar { GROUP_GAP } else { LEADING_INSET };
        let centre = (bar.left() + pad_left + bar.right() - TRAILING_INSET) / 2.0;
        let lo = nav_rect.right() + GROUP_GAP;
        let hi = trailing_left - GROUP_GAP - width;
        if hi < lo {
            // No room between the groups (a narrow window searching).
            #[cfg(target_os = "macos")]
            self.toolbar_items.extend(items);
            return;
        }
        let left = (centre - width / 2.0).min(hi).max(lo).round();
        let pickers = egui::Rect::from_min_size(
            egui::pos2(left, cy - CAPSULE_HEIGHT / 2.0),
            vec2(width, CAPSULE_HEIGHT),
        );
        widgets::capsule(ui.painter(), pickers, p);
        let book_rect = egui::Rect::from_min_size(pickers.min, vec2(book.width(), CAPSULE_HEIGHT));
        let tr_rect = egui::Rect::from_min_size(
            egui::pos2(book_rect.right() + 1.0, pickers.top()),
            vec2(translation.width(), CAPSULE_HEIGHT),
        );
        let loc = book.show(ui, book_rect, p, "book", "Jump to a book or chapter (⌘K)");
        crate::book_picker::show(&loc, p, tmeta.as_ref(), c, actions);
        if show_translation {
            widgets::capsule_divider(ui.painter(), book_rect.right(), cy, p);
            let tr = translation.show(ui, tr_rect, p, "translation", "Translation (⌘1…⌘9)");
            crate::menu::popup(&tr, p).show(|ui| {
                ui.set_min_width(300.0);
                translation_menu(ui, &self.library, &self.settings, actions);
            });
        }
        items.push(pickers);

        // Empty toolbar space moves the window and double-clicks to zoom, like a title bar;
        // the controls don't.
        #[cfg(target_os = "macos")]
        {
            self.toolbar_items.extend(items);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = items;
    }

    /// Focus mode's chrome (frame 30): a reveal bar while the pointer is near the top (Exit
    /// Focus, text options, the shortcut), and a reading-progress hairline with what's left.
    fn focus_chrome(&mut self, ui: &mut Ui, full: egui::Rect, actions: &mut Vec<Action>) {
        let p = self.settings.theme.palette();
        let ctx = ui.ctx().clone();
        let near = self.focus_reveal
            || ctx.input(|i| i.pointer.hover_pos().is_some_and(|q| q.y < full.top() + 64.0));
        let t = ctx.animate_bool_with_time(Id::new("focus-reveal"), near, theme::motion::BASE_MS / 1000.0);
        if t > 0.0 {
            let chord = crate::keymap::ALL
                .iter()
                .find(|s| s.id == "focus")
                .map(|s| crate::keymap::keycaps(s, &self.settings.shortcuts).concat())
                .unwrap_or_default();
            let hint = format!("Exit Focus {chord}");
            let hint_w = ui
                .painter()
                .layout_no_wrap(hint.clone(), theme::ui_medium(12.0), egui::Color32::PLACEHOLDER)
                .size()
                .x;
            let width = 73.0 + 10.0 + hint_w;
            let left = (full.center().x - width / 2.0).round();
            let mut bar = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(egui::Rect::from_min_size(egui::pos2(left, full.top()), vec2(width, 52.0))),
            );
            bar.set_opacity(0.9 * t);
            let buttons = [
                widgets::CapsuleButton {
                    tooltip: "Exit Focus",
                    enabled: true,
                    on: false,
                    icon: crate::icons::contract,
                },
                widgets::CapsuleButton {
                    tooltip: "Text & theme",
                    enabled: true,
                    on: false,
                    icon: crate::icons::text_size,
                },
            ];
            let (capsule, resp) =
                widgets::button_capsule(&mut bar, p, left, full.top() + 26.0, 36.0, true, &buttons);
            bar.painter().text(
                egui::pos2(capsule.right() + 10.0, full.top() + 26.0),
                Align2::LEFT_CENTER,
                hint,
                theme::ui_medium(12.0),
                p.ink_3,
            );
            if resp[0].clicked() {
                actions.push(Action::ToggleFocus);
            }
            egui::Popup::menu(&resp[1]).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(
                |ui| {
                    ui.set_min_width(250.0);
                    crate::settings_ui::quick_text_settings(ui, &mut self.settings, actions);
                },
            );
        }

        // Progress through the chapter.
        let (c, _) = self.reader.location;
        let t = self.library.load(&self.settings.translation).ok();
        let verses: Vec<(u8, usize)> = t
            .as_ref()
            .map(|t| t.verses_in(c).map(|v| (v.number, v.text.split_whitespace().count())).collect())
            .unwrap_or_default();
        let total = verses.len();
        // The text scrolls under the progress line, which sits on the page colour.
        let band = egui::Rect::from_min_max(egui::pos2(full.left(), full.bottom() - 30.0), full.max);
        ui.painter().rect_filled(band, 0.0, p.paper);
        if total > 0 {
            let read = (self.reader.furthest_in_view as usize).min(total);
            let words: usize = verses.iter().filter(|(n, _)| *n as usize > read).map(|(_, w)| w).sum();
            let track = egui::Rect::from_min_size(
                egui::pos2(full.left(), full.bottom() - 30.0),
                vec2(full.width(), 2.0),
            );
            ui.painter().rect_filled(track, 0.0, p.fill);
            let done =
                egui::Rect::from_min_size(track.min, vec2(full.width() * read as f32 / total as f32, 2.0));
            ui.painter().rect_filled(done, 0.0, p.accent.gamma_multiply(0.5));
            let left = if words == 0 {
                "the end of the chapter".to_string()
            } else {
                format!("about {} min left", words.div_ceil(230))
            };
            ui.painter().text(
                egui::pos2(full.center().x, full.bottom() - 14.0),
                Align2::CENTER_CENTER,
                format!("{read} of {total} verses · {left}"),
                theme::ui_medium(11.0),
                p.ink_3,
            );
        }
    }

    /// The parallel columns' heads (frame 07): each translation's abbreviation as a picker over
    /// its full name, a hairline below.
    fn column_heads(
        &mut self,
        ui: &mut Ui,
        reader_rect: egui::Rect,
        cols: &[String],
        actions: &mut Vec<Action>,
    ) {
        let p = self.settings.theme.palette();
        let top = reader_rect.top() + self.chrome.height + 16.0;
        let pad = crate::reader::layout::PARALLEL_ROW_PAD.x;
        let (Some(first), Some(last)) =
            (self.reader.columns_x.first().copied(), self.reader.columns_x.last().copied())
        else {
            return;
        };
        let line = top + 34.0 + 16.0 - 0.5;
        ui.painter().hline(first.0 - pad..=last.0 + last.1 + pad, line, Stroke::new(1.0, p.separator));
        for (i, (x, _w)) in self.reader.columns_x.clone().into_iter().enumerate() {
            let Some(id) = cols.get(i) else { break };
            let Some(meta) = self.library.get(id).map(|e| e.meta.clone()) else { continue };
            let font = theme::text::UI_STRONG.font();
            let g = ui.painter().layout_no_wrap(meta.abbrev.clone(), font, p.ink);
            let pick = egui::Rect::from_min_size(egui::pos2(x, top), vec2(g.size().x + 4.0 + 12.0, 16.0));
            let resp = ui.interact(pick.expand(4.0), Id::new(("column-head", i)), Sense::click());
            ui.painter().galley(egui::pos2(x, top + 8.0 - g.size().y / 2.0), g, p.ink);
            crate::icons::chevron_down(
                ui.painter(),
                egui::Rect::from_center_size(egui::pos2(pick.right() - 6.0, top + 8.0), vec2(12.0, 12.0)),
                p.ink_2,
            );
            ui.painter().text(
                egui::pos2(x, top + 16.0 + 2.0 + 8.0),
                egui::Align2::LEFT_CENTER,
                &meta.name,
                theme::ui_medium(11.0),
                p.ink_3,
            );
            crate::menu::popup(&resp, p).show(|ui| {
                ui.set_min_width(240.0);
                for e in self.library.entries() {
                    let m = &e.meta;
                    if ui.selectable_label(m.id == *id, format!("{}  {}", m.abbrev, m.name)).clicked() {
                        actions.push(if i == 0 {
                            Action::SetTranslation(m.id.clone())
                        } else {
                            Action::ReplaceParallel(i - 1, m.id.clone())
                        });
                        ui.close();
                    }
                }
                if i > 0 {
                    crate::menu::separator(ui);
                    if ui.button("Remove column").clicked() {
                        actions.push(Action::RemoveParallel(id.clone()));
                        ui.close();
                    }
                }
            });
        }
    }

    /// While a search has results: "Result 5 of 7", a stepper and a hint, floating over the top
    /// of the text (frame 06).
    fn result_navigator(&mut self, ui: &mut Ui, reader_rect: egui::Rect, actions: &mut Vec<Action>) {
        let Some((at, of)) = self.sidebar.result_position() else { return };
        let p = self.settings.theme.palette();
        let painter = ui.painter();
        let font = theme::ui_medium(theme::text::SMALL.size);
        let label = painter.layout_no_wrap(format!("Result {at} of {of}"), font, p.ink_2);
        let hint = painter.layout_no_wrap("Esc to clear".into(), theme::text::SMALL.font(), p.ink_3);
        let w = 14.0 + label.size().x + 8.0 + 24.0 + 8.0 + 1.0 + 8.0 + hint.size().x + 8.0;
        let top = reader_rect.top() + self.chrome.height + crate::reader::layout::PAGE_TOP;
        let r = egui::Rect::from_min_size(
            egui::pos2((reader_rect.center().x - w / 2.0).round(), top),
            vec2(w, 32.0),
        );
        widgets::capsule(painter, r, p);
        let cy = r.center().y;
        let mut x = r.left() + 14.0;
        let lw = label.size().x;
        painter.galley(egui::pos2(x, cy - label.size().y / 2.0), label, p.ink_2);
        x += lw + 8.0;
        // The stepper: the upper half goes back, the lower half on.
        let b = egui::Rect::from_min_size(egui::pos2(x, cy - 12.0), vec2(24.0, 24.0));
        let resp = ui.interact(b, Id::new("result-step"), Sense::click());
        if let Some(fill) = widgets::hover_fill(ui, &resp, p, false) {
            ui.painter().circle_filled(b.center(), 12.0, fill);
        }
        crate::icons::heavy(|| {
            crate::icons::chevrons_up_down(
                ui.painter(),
                egui::Rect::from_center_size(b.center(), vec2(14.0, 14.0)),
                p.ink_2,
            )
        });
        if resp.clicked() {
            let up = resp.interact_pointer_pos().is_some_and(|pt| pt.y < b.center().y);
            actions.push(Action::SearchStep(if up { -1 } else { 1 }));
        }
        let resp = resp.on_hover_text("Previous / next result (↑ ↓ in the search field)");
        let _ = resp;
        x = b.right() + 8.0;
        widgets::capsule_divider(ui.painter(), x, cy, p);
        x += 1.0 + 8.0;
        ui.painter().galley(egui::pos2(x, cy - hint.size().y / 2.0), hint, p.ink_3);
        #[cfg(target_os = "macos")]
        self.toolbar_items.push(r);
    }

    /// The compact toolbar's More menu (frame 34): the view tools, the sidebar and focus mode,
    /// with their shortcuts as currently bound.
    fn more_menu(&mut self, ui: &mut Ui, more: &egui::Response, actions: &mut Vec<Action>) {
        let p = self.settings.theme.palette();
        let chord = |id: &str| {
            crate::keymap::ALL
                .iter()
                .find(|s| s.id == id)
                .map(|s| crate::keymap::keycaps(s, &self.settings.shortcuts).concat())
                .unwrap_or_default()
        };
        let sidebar = if self.settings.sidebar_open { "Hide Sidebar" } else { "Show Sidebar" };
        let items: [Option<(widgets::IconFn, &str, String, Action)>; 5] = [
            Some((crate::icons::columns, "Parallel Translations", chord("parallel"), Action::ToggleParallel)),
            Some((crate::icons::text_size, "Text & Layout", "⌘T".into(), Action::TextFormat)),
            None,
            Some((crate::icons::sidebar, sidebar, chord("sidebar"), Action::ToggleSidebar)),
            Some((crate::icons::expand, "Focus Mode", chord("focus"), Action::ToggleFocus)),
        ];
        let id = Id::new(MORE_POPUP);
        if std::mem::take(&mut self.open_more) {
            egui::Popup::open_id(ui.ctx(), id);
        }
        if egui::Popup::is_id_open(ui.ctx(), id) {
            // Arrow keys move the highlight; return chooses.
            let enabled: Vec<usize> = (0..items.len()).filter(|i| items[*i].is_some()).collect();
            let (down, up, enter) = ui.input_mut(|i| {
                (
                    i.consume_key(Modifiers::NONE, Key::ArrowDown),
                    i.consume_key(Modifiers::NONE, Key::ArrowUp),
                    i.consume_key(Modifiers::NONE, Key::Enter),
                )
            });
            let at = self.more_highlight.and_then(|h| enabled.iter().position(|i| *i == h));
            let n = enabled.len();
            if down {
                self.more_highlight = Some(enabled[at.map_or(0, |a| (a + 1) % n)]);
            }
            if up {
                self.more_highlight = Some(enabled[at.map_or(n - 1, |a| (a + n - 1) % n)]);
            }
            if enter && let Some(Some((.., a))) = self.more_highlight.and_then(|h| items.get(h)) {
                actions.push(a.clone());
                egui::Popup::close_id(ui.ctx(), id);
            }
        } else {
            self.more_highlight = None;
        }
        let popup = crate::menu::popup(more, p).id(id).align(egui::RectAlign::BOTTOM_END).gap(6.0);
        popup.show(|ui| {
            for (i, item) in items.iter().enumerate() {
                let Some((icon, label, keys, action)) = item else {
                    let (r, _) =
                        ui.allocate_exact_size(vec2(220.0, crate::menu::SEPARATOR_HEIGHT), Sense::hover());
                    crate::menu::paint_separator(ui.painter(), p, r.left(), r.right(), r.top());
                    continue;
                };
                let (r, resp) = ui.allocate_exact_size(vec2(220.0, crate::menu::ITEM_HEIGHT), Sense::click());
                if resp.hovered() && ui.input(|inp| inp.pointer.delta() != egui::Vec2::ZERO) {
                    self.more_highlight = Some(i);
                }
                let row = crate::menu::Row {
                    icon: *icon,
                    label,
                    shortcut: keys,
                    enabled: true,
                    lit: self.more_highlight == Some(i),
                    pressed: resp.is_pointer_button_down_on(),
                };
                crate::menu::paint_row(ui.painter(), p, r, row);
                if resp.clicked() {
                    actions.push(action.clone());
                }
            }
        });
    }

    /// Whether the reading translation is online and can't load the chapter being read: then
    /// `Some(retrying)`, with `self.offline` saying which chapter and why.
    fn check_offline(&mut self, cols: &[String]) -> Option<bool> {
        let online = cols.len() == 1
            && !self.focus
            && self.library.get(&cols[0]).is_some_and(|e| e.online().is_some());
        if !online {
            self.offline = None;
            return None;
        }
        let (id, c) = (&cols[0], self.reader.location.0);
        let same = self.offline.as_ref().is_some_and(|o| o.chapter == c && o.id == *id);
        match self.library.chapter(id, c) {
            bible_core::library::Chapter::Failed(message) => {
                match &mut self.offline {
                    Some(o) if same => o.message = message,
                    _ => {
                        self.offline = Some(crate::offline::Offline {
                            id: id.clone(),
                            chapter: c,
                            message,
                            dismissed: false,
                        })
                    }
                }
                Some(false)
            }
            bible_core::library::Chapter::Pending if same => Some(true),
            _ => {
                self.offline = None;
                None
            }
        }
    }

    /// Toasts at the bottom centre of the main area, 24 pt above the window edge: inverse
    /// capsules 40 high (design notes §7). Messages go after 4 s; the error toast of a chapter
    /// that can't load offers Retry and stays until clicked away or the chapter loads.
    fn toasts(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        self.toasts.retain(|(_, t)| now - t < 4.0);
        let p = self.settings.theme.palette();
        let area = if self.main_area.is_positive() { self.main_area } else { ctx.content_rect() };
        // Ink on paper; in the dark a warm near-white, with the light theme's danger.
        let dark = self.settings.theme == Theme::Dark;
        let [r, g, b, _] = if dark { [0xF2, 0xF0, 0xEB, 0xFF] } else { p.ink.to_array() };
        let fill = egui::Color32::from_rgba_unmultiplied(r, g, b, 0xF2);
        let ink = p.paper;
        let [r, g, b, _] = p.paper.to_array();
        let divider = egui::Color32::from_rgba_unmultiplied(r, g, b, 0x8C);
        let danger = if dark { Theme::Light.palette().danger } else { Theme::Dark.palette().danger };

        let mut msgs: Vec<(String, bool)> = Vec::new();
        if let Some(o) = self.offline.as_ref().filter(|o| !o.dismissed) {
            msgs.push((o.toast(&self.library), true));
        }
        msgs.extend(self.toasts.iter().map(|(m, _)| (m.clone(), false)));
        if let Some(b) = &self.busy {
            msgs.push((b.clone(), false));
        }
        let (mut retry, mut dismiss) = (false, false);
        let mut y = area.bottom() - 24.0;
        for (i, (m, error)) in msgs.iter().enumerate() {
            let id = Id::new(("toast", i));
            let resp = egui::Area::new(id)
                .order(egui::Order::Tooltip)
                .fixed_pos(egui::pos2(area.center().x, y))
                .pivot(Align2::CENTER_BOTTOM)
                .interactable(*error)
                .show(ctx, |ui| {
                    let painter = ui.painter().clone();
                    let font = theme::ui_medium(13.0);
                    let m = crate::pages::elide(&painter, m, &font, 480.0);
                    let msg = painter.layout_no_wrap(m, font, ink);
                    let action =
                        error.then(|| painter.layout_no_wrap("Retry".into(), theme::ui_semibold(13.0), ink));
                    let lead = if *error { 14.0 + 16.0 + 10.0 } else { 16.0 };
                    let tail = action.as_ref().map_or(0.0, |a| 10.0 + 1.0 + 10.0 + a.size().x);
                    let (rect, bg) =
                        ui.allocate_exact_size(vec2(lead + msg.size().x + tail + 16.0, 40.0), Sense::click());
                    painter.add(
                        egui::epaint::RectShape::filled(
                            rect.translate(vec2(0.0, 8.0)),
                            20.0,
                            egui::Color32::from_black_alpha(0x2E),
                        )
                        .with_blur_width(48.0),
                    );
                    painter.rect_filled(rect, 20.0, fill);
                    let cy = rect.center().y;
                    let mut x = rect.left() + lead;
                    if *error {
                        let icon = egui::Rect::from_center_size(
                            egui::pos2(rect.left() + 22.0, cy),
                            vec2(16.0, 16.0),
                        );
                        crate::icons::heavy(|| crate::icons::warning(&painter, icon, danger));
                    }
                    let w = msg.size().x;
                    painter.galley(egui::pos2(x, cy - msg.size().y / 2.0), msg, ink);
                    x += w;
                    if let Some(a) = action {
                        x += 10.0;
                        painter.vline(x + 0.5, (cy - 8.0)..=(cy + 8.0), Stroke::new(1.0, divider));
                        x += 1.0 + 10.0;
                        let hit = egui::Rect::from_min_size(
                            egui::pos2(x - 6.0, rect.top()),
                            vec2(a.size().x + 12.0, rect.height()),
                        );
                        let act = ui.interact(hit, id.with("action"), Sense::click());
                        painter.galley(egui::pos2(x, cy - a.size().y / 2.0), a, ink);
                        if act.clicked() {
                            retry = true;
                        } else if bg.clicked() {
                            dismiss = true;
                        }
                    }
                });
            y -= resp.response.rect.height() + 8.0;
        }
        if let Some(o) = &mut self.offline {
            if retry {
                o.retry(&self.library);
                ctx.request_repaint();
            }
            o.dismissed |= dismiss;
        }
        if !self.toasts.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }

    fn shortcuts_help(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        if !self.shortcuts_open {
            return;
        }
        let p = self.settings.theme.palette();
        if self.shortcuts_overlay.show(ctx, p, &self.settings.shortcuts, actions) {
            self.shortcuts_open = false;
        }
    }
}

/// Below this toolbar width the view tools and the search field fold into one capsule.
const COMPACT_TOOLBAR: f32 = 760.0;

/// The text & layout popover and the compact toolbar's More menu.
const TEXT_POPUP: &str = "text-popover";
const MORE_POPUP: &str = "more-menu";

/// The toolbar's field on pages (it filters the page).
const PAGE_SEARCH_WIDTH: f32 = 200.0;

/// Under the toolbar in parallel: the columns' heads (16 pt down, 34 tall, 16 pt to a hairline).
const COLUMN_HEADS: f32 = 66.0;

/// Under the toolbar while searching: the result navigator (24 pt down, 32 tall) and 16 pt below.
const SEARCH_BAND: f32 = 48.0;

/// Height of the sidebar's header (traffic lights and toggle) and the search sidebar's width.
const SIDEBAR_HEADER: f32 = 36.0;
const SEARCH_SIDEBAR_WIDTH: f32 = 352.0;

/// The sidebar's own toggle: a borderless 32 pt button in secondary ink.
fn sidebar_toggle(ui: &mut Ui, p: &theme::Palette, rect: egui::Rect) -> egui::Response {
    let resp = ui.interact(rect, Id::new("sidebar-toggle"), Sense::click());
    if let Some(fill) = widgets::hover_fill(ui, &resp, p, false) {
        ui.painter().circle_filled(rect.center(), rect.width() / 2.0, fill);
    }
    let icon = egui::Rect::from_center_size(rect.center(), vec2(theme::metrics::ICON, theme::metrics::ICON));
    crate::icons::sidebar(ui.painter(), icon, p.ink_2);
    resp.on_hover_text("Hide sidebar (⌘B)")
}

fn translation_menu(ui: &mut Ui, library: &Library, settings: &Settings, actions: &mut Vec<Action>) {
    let p = settings.theme.palette();
    for (i, e) in library.entries().iter().enumerate() {
        let m = &e.meta;
        let current = m.id == settings.translation;
        let shortcut = if i < 9 { format!("⌘{}", i + 1) } else { String::new() };
        let resp = ui.add(
            egui::Button::selectable(current, egui::RichText::new(format!("{}  {}", m.abbrev, m.name)))
                .shortcut_text(shortcut),
        );
        if resp.clicked() {
            actions.push(Action::SetTranslation(m.id.clone()));
            ui.close();
        }
    }
    crate::menu::separator(ui);
    crate::menu::header(ui, p, "Parallel view");
    for e in library.entries() {
        let m = &e.meta;
        if m.id == settings.translation {
            continue;
        }
        let on = settings.parallel_on && settings.parallel.contains(&m.id);
        let mut checked = on;
        if crate::menu::checkbox(ui, &mut checked, format!("{} — side by side", m.abbrev)).changed() {
            actions.push(if checked {
                Action::AddParallel(m.id.clone())
            } else {
                Action::RemoveParallel(m.id.clone())
            });
        }
    }
    crate::menu::separator(ui);
    if ui.button("Manage translations…").clicked() {
        actions.push(Action::OpenSettings(SettingsTab::Translations));
        ui.close();
    }
}

impl BibleApp {
    /// The end of every frame: window drag areas, actions, persistence and startup bookkeeping.
    fn end_frame(
        &mut self,
        ctx: &egui::Context,
        frame: &eframe::Frame,
        frame_start: Instant,
        actions: Vec<Action>,
    ) {
        #[cfg(not(target_os = "macos"))]
        let _ = frame;
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
        self.apply(ctx, actions);
        if had_actions {
            ctx.request_repaint();
        }
        self.persist(ctx, false);

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
        if let Some(probe) = &mut self.sidebar_probe
            && let Some(report) = probe.end(frame_start, self.reader.layouts)
        {
            println!("{report}");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            self.sidebar_probe = None;
        }
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
        if let Some(probe) = &mut self.sidebar_probe
            && probe.begin(&ctx, frame_start)
        {
            actions.push(Action::ToggleSidebar);
        }
        if self.onboarding.is_none() {
            self.shortcuts(&ctx, &mut actions);
        }
        #[cfg(target_os = "macos")]
        if let Some(menu) = &self.menu {
            menu.poll(&mut actions);
        }
        // Demo steps wait for the window to be focused (or a few seconds), so a screenshot
        // script can activate it before any popup opens; a background launch doesn't wait.
        let focused = ctx.input(|i| i.focused)
            || std::env::var_os("BIBLE_RS_BACKGROUND").is_some()
            || self.startup.t0.elapsed() > Duration::from_secs(4);
        if self.startup.frames > 2
            && focused
            && let Some(step) = self.demo.pop_front()
        {
            self.demo_step(&ctx, &step, &mut actions);
            ctx.request_repaint();
        }
        if self.settings.theme_system {
            let t = if ctx.system_theme() == Some(egui::Theme::Dark) { Theme::Dark } else { Theme::Light };
            if t != self.settings.theme {
                actions.push(Action::SetTheme(t));
            }
        }
        let p = self.settings.theme.palette();

        self.chrome = chrome(frame, &ctx);
        if let Some(onboarding) = &mut self.onboarding {
            let done = egui::CentralPanel::default()
                .frame(Frame::NONE)
                .show(ui, |ui| onboarding.show(ui, &self.library, &mut self.settings, &mut actions))
                .inner;
            if done {
                self.onboarding = None;
                self.settings.onboarded = true;
                open_reader_window(&ctx);
            }
            self.end_frame(&ctx, frame, frame_start, actions);
            return;
        }
        #[cfg(target_os = "macos")]
        self.toolbar_items.clear();
        if self.settings.sidebar_open && !self.focus {
            // A floating source list, full height: the traffic lights and the sidebar toggle sit
            // in its header.
            use theme::metrics::{SIDEBAR_INSET, SIDEBAR_WIDTH};
            let width = if self.sidebar.searching() { SEARCH_SIDEBAR_WIDTH } else { SIDEBAR_WIDTH };
            let attached = self.settings.sidebar_attached;
            let inset = if attached { 0.0 } else { SIDEBAR_INSET };
            egui::Panel::left("sidebar")
                .resizable(false)
                .exact_size(inset + width)
                .frame(Frame::NONE.fill(p.paper))
                .show_separator_line(false)
                .show(ui, |ui| {
                    let full = ui.max_rect();
                    let panel = egui::Rect::from_min_max(
                        full.min + vec2(inset, inset),
                        egui::pos2(full.right(), full.bottom() - inset),
                    );
                    if attached {
                        // Attached: the window's own edge, with a hairline beside the content.
                        ui.painter().rect_filled(panel, 0.0, p.sidebar);
                        ui.painter().vline(
                            panel.right() - 0.5,
                            panel.y_range(),
                            Stroke::new(1.0, p.separator),
                        );
                    } else {
                        let radius = theme::radius::LG;
                        p.shadows.panel.paint(ui.painter(), panel, radius);
                        ui.painter().rect_filled(panel, radius, p.sidebar);
                    }
                    // The toggle, 4 pt from the panel's edge, on the traffic lights' line.
                    let toggle = egui::Rect::from_center_size(
                        egui::pos2(panel.right() - 4.0 - 16.0, self.chrome.height / 2.0),
                        vec2(32.0, 32.0),
                    );
                    if sidebar_toggle(ui, p, toggle).clicked() {
                        actions.push(Action::ToggleSidebar);
                    }
                    #[cfg(target_os = "macos")]
                    self.toolbar_items.push(toggle);
                    let content = egui::Rect::from_min_max(panel.min + vec2(0.0, SIDEBAR_HEADER), panel.max);
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(content)
                            .layout(egui::Layout::top_down(egui::Align::Min)),
                    );
                    child.set_clip_rect(content);
                    let env = crate::sidebar::SidebarEnv {
                        library: &self.library,
                        study: &self.study,
                        translation: &self.settings.translation,
                        theme: self.settings.theme,
                        location: self.reader.location,
                        // The current place first, then where the reader came from.
                        recent: std::iter::once(self.reader.location)
                            .chain(self.history.recent().copied())
                            .fold(Vec::new(), |mut v, here| {
                                if !v.iter().any(|(c, _): &(ChapterIdx, u8)| *c == here.0) {
                                    v.push(here);
                                }
                                v
                            }),
                        selection: self.reader.selected_refs().first().copied(),
                        page: self.pages.page,
                    };
                    self.sidebar.show(&mut child, &env, &mut self.settings.whole_words, &mut actions);
                });
        }
        let first = self.startup.frames == 0;
        egui::CentralPanel::default().frame(Frame::NONE.fill(p.paper)).show(ui, |ui| {
            let full = ui.max_rect();
            self.main_area = full;
            // A page in the reader's place, under the same toolbar.
            if !self.pages.reader() {
                let query = self.pages.query.clone();
                let env = crate::pages::PageEnv {
                    library: &self.library,
                    study: &self.study,
                    settings: &self.settings,
                    query: &query,
                    toolbar: self.chrome.height,
                };
                self.pages.show(ui, full, &env, &mut actions);
                let material = ctx.animate_bool_with_time(
                    Id::new("toolbar-material"),
                    self.pages.scrolled,
                    theme::motion::FAST_MS / 1000.0,
                );
                self.toolbar(ui, full, material, &mut actions);
                return;
            }
            // The reader runs the full height, beside the inspector when one is open; the
            // toolbar floats over the top of both.
            let inspector = if self.panel.is_some() { crate::panel::INSPECTOR_WIDTH } else { 0.0 };
            let reader_rect =
                egui::Rect::from_min_max(full.min, egui::pos2(full.right() - inspector, full.bottom()));
            let cols = self.columns();
            let offline = self.check_offline(&cols);
            let searching = self.sidebar.result_position().is_some();
            // Bands under the toolbar: the parallel columns' heads, a search's result navigator.
            let heads = if cols.len() > 1 { COLUMN_HEADS } else { 0.0 };
            let band = heads + if searching { SEARCH_BAND } else { 0.0 };
            let keyboard = !ctx.egui_wants_keyboard_input() && self.palette.is_none();
            // Focus mode: two points larger, 36 pt lines at 22, a 680 pt measure, a quiet head
            // 12 pt under where the toolbar would be.
            let focus_typo;
            let (typo, top) = if self.focus {
                let mut t = self.settings.typo.clone();
                let size = (t.size + 2.0).round();
                t.line_height = (size * 36.0 / 22.0).round() / size;
                t.measure = 680.0 / 22.0;
                t.size = size;
                t.quiet_head = true;
                focus_typo = t;
                (&focus_typo, 52.0 + 12.0 - crate::reader::layout::PAGE_TOP)
            } else {
                (&self.settings.typo, self.chrome.height + band)
            };
            let env = Env {
                library: &self.library,
                study: &self.study,
                typo,
                theme: self.settings.theme,
                columns: &cols,
                keyboard,
                online_generation: &self.online_gen,
                toolbar: top,
            };
            let mut reader_ui = ui.new_child(egui::UiBuilder::new().max_rect(reader_rect));
            if let (Some(state), Some(retrying)) = (&self.offline, offline) {
                // The chapter can't be fetched: the error page in the reader's place.
                let area = egui::Rect::from_min_max(
                    egui::pos2(reader_rect.left(), reader_rect.top() + self.chrome.height),
                    reader_rect.max,
                );
                crate::offline::show(&mut reader_ui, p, area, &self.library, state, retrying, &mut actions);
            } else {
                self.reader.marker_for = self.note_popover.as_ref().map(|n| n.verse);
                self.reader.show(&mut reader_ui, &env, &mut actions);
            }
            if first {
                crate::trace(self.startup.t0, "reader laid out and painted");
            }
            // Not while the first frames are being raced to the screen, nor while nothing is read.
            if self.startup.frames > 2 && offline.is_none() {
                self.reading.tick(
                    ctx.input(|i| i.time),
                    ctx.input(|i| i.focused),
                    self.reader.location.0,
                    self.reader.furthest_in_view,
                    &self.settings.translation,
                    &self.library,
                    &self.study,
                );
            }
            if band > 0.0 {
                // egui can't blur what scrolls beneath, so the bands are solid paper.
                let r = egui::Rect::from_min_size(
                    reader_rect.min,
                    vec2(reader_rect.width(), self.chrome.height + band),
                );
                ui.painter().rect_filled(r, 0.0, p.paper);
            }
            if heads > 0.0 {
                self.column_heads(ui, reader_rect, &cols, &mut actions);
            }
            self.result_navigator(ui, reader_rect.translate(vec2(0.0, heads)), &mut actions);
            if let Some(panel) = &mut self.panel {
                let r = egui::Rect::from_min_max(
                    egui::pos2(reader_rect.right(), full.top() + self.chrome.height),
                    egui::pos2(
                        full.right() - theme::metrics::SIDEBAR_INSET,
                        full.bottom() - theme::metrics::SIDEBAR_INSET,
                    ),
                );
                p.shadows.panel.paint(ui.painter(), r, theme::radius::LG);
                ui.painter().rect_filled(r, theme::radius::LG, p.sidebar);
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
                child.set_clip_rect(r);
                if panel.show(&mut child, &self.library, &self.settings, &cols, &mut actions) {
                    self.panel = None;
                }
            }
            let material = ctx.animate_bool_with_time(
                Id::new("toolbar-material"),
                self.reader.under_toolbar && band == 0.0,
                theme::motion::FAST_MS / 1000.0,
            );
            if self.focus {
                self.focus_chrome(ui, full, &mut actions);
            } else {
                self.toolbar(ui, full, material, &mut actions);
            }
            if first {
                crate::trace(self.startup.t0, "toolbar laid out");
            }
        });

        if let Some(note) = &mut self.note_popover {
            let marker = self
                .reader
                .marker_rect
                .unwrap_or_else(|| crate::note_popover::fallback_anchor(ctx.content_rect()));
            if note.show(
                &ctx,
                p,
                &self.library,
                &mut self.study,
                &self.settings.translation,
                marker,
                &mut actions,
            ) {
                self.note_popover = None;
            }
        }
        if let Some(c) = &mut self.compare {
            let selected = self.reader.selected_refs();
            if !selected.contains(&c.verse) || !self.pages.reader() {
                self.compare = None;
            } else {
                let anchor = self
                    .reader
                    .selection_rect
                    .unwrap_or_else(|| crate::note_popover::fallback_anchor(ctx.content_rect()));
                if c.show(&ctx, p, &self.library, &self.settings, anchor, &mut actions) {
                    self.compare = None;
                }
            }
        }
        if let Some(menu) = &mut self.verse_menu {
            let refs = self.reader.selected_refs();
            if menu.show(&ctx, p, &self.study, &refs, &self.settings, &mut actions) {
                self.verse_menu = None;
            }
        }
        if let Some(pal) = &mut self.palette {
            let close = pal.show(&ctx, &self.library, &self.study, &self.settings, &mut actions);
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
                let mut sheet = crate::import_sheet::ImportSheet::with_path(path);
                sheet.set_key(key.unwrap_or_default());
                self.import_sheet = Some(sheet);
            }
            if !open {
                self.settings_ui = None;
            }
        }
        // A file dragged onto the window opens the import sheet, whose drop zone takes it.
        if self.import_sheet.is_none() && ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            self.import_sheet = Some(crate::import_sheet::ImportSheet::new());
        }
        if self.import_sheet.is_none()
            && let Some(path) = ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()))
        {
            self.import_sheet = Some(crate::import_sheet::ImportSheet::with_path(path));
        }
        if let Some(sheet) = &mut self.import_sheet {
            match sheet.show(&ctx, p) {
                crate::import_sheet::Outcome::Open => {}
                crate::import_sheet::Outcome::Close => self.import_sheet = None,
                crate::import_sheet::Outcome::Install(imported) => {
                    self.import_sheet = None;
                    self.install_import(&ctx, *imported);
                }
            }
        }
        if let Some(sheet) = &mut self.share_sheet {
            let t = self.library.load(&self.settings.translation).ok();
            match sheet.show(&ctx, p, t.as_ref(), &mut self.settings.share) {
                crate::share_sheet::Outcome::Open => {
                    if sheet.waiting() {
                        ctx.request_repaint();
                    }
                }
                crate::share_sheet::Outcome::Close => self.share_sheet = None,
                crate::share_sheet::Outcome::Toast(msg) => self.toast(&ctx, msg),
                crate::share_sheet::Outcome::Share { path, anchor } => {
                    #[cfg(target_os = "macos")]
                    crate::macos_window::share_file(frame, &path, anchor, ctx.zoom_factor());
                    #[cfg(not(target_os = "macos"))]
                    {
                        let _ = anchor;
                        self.toast(&ctx, format!("Saved the image to {}", path.display()));
                    }
                }
            }
        }
        self.shortcuts_help(&ctx, &mut actions);
        self.plan_reminder(&ctx);
        self.auto_backup(&ctx);
        self.toasts(&ctx);
        self.end_frame(&ctx, frame, frame_start, actions);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.reading.finish(&self.study);
        self.settings.chapter = self.reader.location.0.0;
        self.settings.verse = self.reader.location.1;
        self.settings.save(&self.study);
    }
}

/// After onboarding: the reader's 1280 × 800 window, centred on the screen.
fn open_reader_window(ctx: &egui::Context) {
    let size = vec2(1280.0, 800.0);
    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
    let (outer, monitor) = ctx.input(|i| (i.viewport().outer_rect, i.viewport().monitor_size));
    if let (Some(outer), Some(monitor)) = (outer, monitor) {
        let pos = ((monitor - size) / 2.0).max(egui::Vec2::ZERO);
        let _ = outer;
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos.to_pos2()));
    }
}

/// Where automatic backups go.
pub fn backups_dir(data_dir: &std::path::Path) -> PathBuf {
    data_dir.join("backups")
}

/// The newest backup: when (Unix seconds) and its size in bytes.
pub fn last_backup(data_dir: &std::path::Path) -> Option<(i64, u64)> {
    std::fs::read_dir(backups_dir(data_dir))
        .ok()?
        .filter_map(|e| e.ok()?.metadata().ok())
        .filter(|m| m.is_file())
        .filter_map(|m| {
            let at = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64;
            Some((at, m.len()))
        })
        .max_by_key(|(at, _)| *at)
}

/// Opens a folder in the system's file browser.
pub fn reveal(path: &std::path::Path) {
    let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let _ = std::process::Command::new(opener).arg(path).spawn();
}

/// Opens a web page in the browser.
pub fn open_url(url: &str) {
    let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let _ = std::process::Command::new(opener).arg(url).spawn();
}

pub fn online_counter() -> Arc<AtomicU64> {
    Arc::new(AtomicU64::new(0))
}

pub fn bump(counter: &AtomicU64) {
    counter.fetch_add(1, Ordering::Relaxed);
}
