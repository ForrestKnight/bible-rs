//! Everything the UI can ask the app to do. Widgets push actions; the app applies them once per
//! frame, so keyboard shortcuts, menus, the palette and buttons all share one code path.

use bible_core::study::HighlightColor;
use bible_core::{ChapterIdx, VerseRef};
use serde::{Deserialize, Serialize};

use crate::theme::Theme;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Go somewhere, recording the current position in history.
    Jump {
        chapter: ChapterIdx,
        verse: Option<u8>,
        flash: bool,
    },
    Back,
    Forward,
    PrevChapter,
    NextChapter,
    SetTranslation(String),
    /// Switch to the n-th installed translation (⌘1…⌘9).
    TranslationNumber(usize),
    ToggleParallel,
    /// Do something (a jump), then show it with the parallel translations.
    OpenInParallel(Box<Action>),
    /// Select a verse and compare it across translations.
    CompareVerse(VerseRef),
    AddParallel(String),
    /// Show another translation in a parallel column (0 is the first column after the reading one).
    ReplaceParallel(usize, String),
    RemoveParallel(String),
    Highlight(Option<HighlightColor>),
    EditNote(VerseRef),
    EditSelectedNote,
    ToggleBookmark,
    RemoveBookmark(VerseRef),
    /// Removes one highlight of a verse: the one stored from that char.
    RemoveHighlight(VerseRef, u32),
    RemoveNote(VerseRef),
    /// Save a note's text (an empty text removes it).
    SaveNote(VerseRef, String),
    /// Start (or restart) a reading plan today; stop one.
    StartPlan(String),
    StopPlan(String),
    /// Tick a plan's passage (day, index) read or not.
    SetPlanRead {
        plan: String,
        day: u16,
        passage: u8,
        read: bool,
    },
    SetPlanReminder(bool),
    SetAccent(crate::theme::Accent),
    SetReduceMotion(bool),
    /// Erase every highlight, note, bookmark, plan and the history (after a backup).
    EraseStudy,
    /// Show the data folder in the file browser.
    RevealDataDir,
    /// Focus mode: full screen with the chrome out of the way.
    ToggleFocus,
    /// Open the text & layout popover (⌘T).
    TextFormat,
    /// Forget the reading history from this time (Unix seconds) on.
    ClearReading(i64),
    /// Bookmark a verse, or remove its bookmark.
    ToggleBookmarkAt(VerseRef),
    /// Copy the selected verses' text; `Copy` adds the reference.
    CopyText,
    Copy,
    /// Open the verse menu at a point (a right-click on the selection).
    OpenVerseMenu(eframe::egui::Pos2),
    /// Make an image of the selected verses to share.
    ShareImage,
    Compare,
    ClearSelection,
    SelectNext,
    SelectPrev,
    OpenPalette,
    OpenSearch(Option<String>),
    /// Move to the next (1) or previous (-1) search result.
    SearchStep(i32),
    ClearSearch,
    ToggleSidebar,
    SidebarTab(SidebarTab),
    OpenSettings(SettingsTab),
    FontSize(f32),
    FontReset,
    SetTheme(Theme),
    CycleTheme,
    ToggleMode,
    ToggleRedLetters,
    Export(ExportKind),
    ImportStudy,
    ImportTranslation,
    /// Remove an imported or online translation.
    RemoveTranslation(String),
    /// Show a page in the main area (the reader, the library, a study collection).
    OpenPage(crate::pages::Page),
    ShowShortcuts,
    CheckForUpdates,
    ScrollLines(f32),
    ScrollPages(f32),
    Toast(String),
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SidebarTab {
    #[default]
    Books,
    Search,
    Study,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    Reading,
    Appearance,
    Translations,
    Shortcuts,
    Data,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    Json,
    Markdown,
}
