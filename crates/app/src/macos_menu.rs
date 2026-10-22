//! Native macOS menu bar.
//!
//! Items with key equivalents are dispatched by AppKit before the key reaches egui, so each menu
//! command is turned back into the same [`Action`] the keyboard path produces (`native_menu.rs`).
//! There is no Edit menu on purpose: AppKit's predefined copy/paste items would swallow ⌘C/⌘V
//! before egui's text fields see them (verse copying is ⌘C handled by the app itself).

use muda::accelerator::Accelerator;
use muda::{Menu, MenuItem, PredefinedMenuItem, Submenu};

use crate::actions::Action;

pub struct AppMenu {
    _menu: Menu,
}

fn item(id: &str, label: &str, accel: Option<&str>) -> MenuItem {
    MenuItem::with_id(id, label, true, accel.and_then(|a| a.parse::<Accelerator>().ok()))
}

impl AppMenu {
    /// Builds the menu bar and installs it as the application menu.
    pub fn install() -> Option<Self> {
        let menu = Menu::new();
        let app = Submenu::with_items(
            "Bible",
            true,
            &[
                &PredefinedMenuItem::about(Some("About Bible"), None),
                &item("check_updates", "Check for Updates…", None),
                &PredefinedMenuItem::separator(),
                &item("settings", "Settings…", Some("CmdOrCtrl+Comma")),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::services(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::hide(None),
                &PredefinedMenuItem::hide_others(None),
                &PredefinedMenuItem::show_all(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::quit(None),
            ],
        )
        .ok()?;
        let file = Submenu::with_items(
            "File",
            true,
            &[
                &item("import_translation", "Import Translation…", Some("CmdOrCtrl+I")),
                &item("translations", "Manage Translations & API Keys…", None),
                &PredefinedMenuItem::separator(),
                &item("export_md", "Export Notes as Markdown…", Some("CmdOrCtrl+E")),
                &item("export_json", "Export Notes as JSON…", Some("CmdOrCtrl+Shift+E")),
                &item("import_notes", "Import Notes from JSON…", None),
            ],
        )
        .ok()?;
        let view = Submenu::with_items(
            "View",
            true,
            &[
                &item("theme_light", "Light", None),
                &item("theme_sepia", "Sepia", None),
                &item("theme_dark", "Dark", None),
                &item("theme_cycle", "Next Theme", Some("CmdOrCtrl+Shift+L")),
                &PredefinedMenuItem::separator(),
                &item("bigger", "Bigger Text", Some("CmdOrCtrl+Equal")),
                &item("smaller", "Smaller Text", Some("CmdOrCtrl+Minus")),
                &item("actual", "Actual Size", Some("CmdOrCtrl+Digit0")),
                &item("text", "Text & Layout…", Some("CmdOrCtrl+KeyT")),
                &PredefinedMenuItem::separator(),
                &item("mode", "Verse per Line / Paragraphs", Some("CmdOrCtrl+L")),
                &item("red", "Red Letters", Some("CmdOrCtrl+R")),
                &item("parallel", "Parallel Translations", Some("CmdOrCtrl+Shift+P")),
                &item("sidebar", "Sidebar", Some("CmdOrCtrl+B")),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::fullscreen(None),
            ],
        )
        .ok()?;
        let go = Submenu::with_items(
            "Go",
            true,
            &[
                &item("jump", "Jump to Reference…", Some("CmdOrCtrl+K")),
                &item("search", "Search…", Some("CmdOrCtrl+F")),
                &PredefinedMenuItem::separator(),
                &item("back", "Back", Some("CmdOrCtrl+BracketLeft")),
                &item("forward", "Forward", Some("CmdOrCtrl+BracketRight")),
                &item("prev", "Previous Chapter", None),
                &item("next", "Next Chapter", None),
                &PredefinedMenuItem::separator(),
                &item("bookmark", "Bookmark", Some("CmdOrCtrl+D")),
                &item("compare", "Compare Translations", None),
            ],
        )
        .ok()?;
        let window = Submenu::with_items("Window", true, &[&PredefinedMenuItem::minimize(None)]).ok()?;
        let help = Submenu::with_items(
            "Help",
            true,
            &[&item("shortcuts", "Keyboard Shortcuts", Some("CmdOrCtrl+Slash"))],
        )
        .ok()?;
        menu.append_items(&[&app, &file, &view, &go, &window, &help]).ok()?;
        menu.init_for_nsapp();
        window.set_as_windows_menu_for_nsapp();
        Some(Self { _menu: menu })
    }

    /// Drains menu clicks into this frame's actions.
    pub fn poll(&self, actions: &mut Vec<Action>) {
        crate::native_menu::poll(actions);
    }
}
