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
    AddParallel(String),
    RemoveParallel(String),
    Highlight(Option<HighlightColor>),
    EditNote(VerseRef),
    EditSelectedNote,
    ToggleBookmark,
    RemoveBookmark(VerseRef),
    RemoveHighlight(VerseRef),
    RemoveNote(VerseRef),
    Copy,
    Compare,
    ClearSelection,
    SelectNext,
    SelectPrev,
    OpenPalette,
    OpenSearch(Option<String>),
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
    ShowShortcuts,
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
    Translations,
    Data,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    Json,
    Markdown,
}
