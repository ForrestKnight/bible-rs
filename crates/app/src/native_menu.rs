//! What the native menu bar's items do, shared by the macOS and Windows menus
//! (`macos_menu.rs`, `windows_menu.rs`): each item's id becomes the same [`Action`] the keyboard
//! path produces.

use muda::MenuEvent;

use crate::actions::{Action, ExportKind, SettingsTab};
use crate::theme::Theme;

/// The action of the item with this id.
pub fn action(id: &str) -> Option<Action> {
    Some(match id {
        "settings" => Action::OpenSettings(SettingsTab::Reading),
        "about" => Action::OpenSettings(SettingsTab::About),
        "check_updates" => Action::CheckForUpdates,
        "import_translation" => Action::ImportTranslation,
        "translations" => Action::OpenSettings(SettingsTab::Translations),
        "export_md" => Action::Export(ExportKind::Markdown),
        "export_json" => Action::Export(ExportKind::Json),
        "import_notes" => Action::ImportStudy,
        "theme_light" => Action::SetTheme(Theme::Light),
        "theme_sepia" => Action::SetTheme(Theme::Sepia),
        "theme_dark" => Action::SetTheme(Theme::Dark),
        "theme_cycle" => Action::CycleTheme,
        "bigger" => Action::FontSize(1.0),
        "smaller" => Action::FontSize(-1.0),
        "actual" => Action::FontReset,
        "text" => Action::TextFormat,
        "mode" => Action::ToggleMode,
        "red" => Action::ToggleRedLetters,
        "parallel" => Action::ToggleParallel,
        "sidebar" => Action::ToggleSidebar,
        "focus" => Action::ToggleFocus,
        "jump" => Action::OpenPalette,
        "search" => Action::OpenSearch(None),
        "back" => Action::Back,
        "forward" => Action::Forward,
        "prev" => Action::PrevChapter,
        "next" => Action::NextChapter,
        "bookmark" => Action::ToggleBookmark,
        "compare" => Action::Compare,
        "shortcuts" => Action::ShowShortcuts,
        "quit" => Action::Quit,
        _ => return None,
    })
}

/// Drains menu clicks into this frame's actions (macOS; Windows queues them itself to wake egui).
pub fn poll(actions: &mut Vec<Action>) {
    while let Ok(event) = MenuEvent::receiver().try_recv() {
        if let Some(a) = action(event.id.0.as_str()) {
            actions.push(a);
        }
    }
}
