//! Native Windows menu bar (File, View, Go, Help) under the title bar.
//!
//! The items show their shortcuts, but the keys themselves still go to egui: the app installs no
//! accelerator table, so a shortcut changed in Settings › Shortcuts works and nothing fires twice.
//! Clicks become the same [`Action`]s the keyboard produces (`native_menu.rs`). Alt and the
//! underlined letters open the menus, as in any Windows program.
//!
//! Choosing an item reaches the window as `WM_COMMAND`, which winit doesn't pass on, so the click
//! also asks egui for a frame; otherwise it would wait for the next mouse move or key press.

use std::sync::Mutex;

use eframe::egui;
use muda::accelerator::Accelerator;
use muda::{Menu, MenuEvent, MenuItem, MenuTheme, PredefinedMenuItem, Submenu};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetWindowRect, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER, SetWindowPos,
};

use crate::actions::Action;

pub struct AppMenu {
    menu: Menu,
    hwnd: isize,
    dark: bool,
    shown: bool,
    focus: MenuItem,
    focus_enabled: bool,
}

fn item(id: &str, label: &str, accel: Option<&str>) -> MenuItem {
    MenuItem::with_id(id, label, true, accel.and_then(|a| a.parse::<Accelerator>().ok()))
}

/// Ids of the items chosen since the last frame.
static CHOSEN: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn theme(dark: bool) -> MenuTheme {
    if dark { MenuTheme::Dark } else { MenuTheme::Light }
}

/// The window's client area (width, height) in pixels.
fn client_size(hwnd: HWND) -> Option<(i32, i32)> {
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    // SAFETY: `hwnd` is the app's live window; `r` is ours to fill.
    (unsafe { GetClientRect(hwnd, &mut r) } != 0).then(|| (r.right - r.left, r.bottom - r.top))
}

impl AppMenu {
    /// Builds the menu bar and attaches it to the window, keeping the window's content the size
    /// it was (the bar adds to the window's height rather than taking from the content).
    pub fn install(cc: &eframe::CreationContext<'_>, dark: bool) -> Option<Self> {
        let RawWindowHandle::Win32(h) = cc.window_handle().ok()?.as_raw() else { return None };
        let hwnd = h.hwnd.get();
        let menu = Menu::new();
        let focus = item("focus", "&Focus Mode", Some("F11"));
        let file = Submenu::with_items(
            "&File",
            true,
            &[
                &item("import_translation", "&Import Translation…", Some("CmdOrCtrl+I")),
                &item("translations", "&Manage Translations && API Keys…", None),
                &PredefinedMenuItem::separator(),
                &item("export_md", "&Export Notes as Markdown…", Some("CmdOrCtrl+E")),
                &item("export_json", "Export Notes as &JSON…", Some("CmdOrCtrl+Shift+E")),
                &item("import_notes", "Import &Notes from JSON…", None),
                &PredefinedMenuItem::separator(),
                &item("settings", "&Settings…", Some("CmdOrCtrl+Comma")),
                &PredefinedMenuItem::separator(),
                // Closes the window the usual way, so settings are saved (the predefined Quit
                // item ends the message loop without that).
                &item("quit", "E&xit", Some("Alt+F4")),
            ],
        )
        .ok()?;
        let view = Submenu::with_items(
            "&View",
            true,
            &[
                &item("theme_light", "&Light", None),
                &item("theme_sepia", "&Sepia", None),
                &item("theme_dark", "&Dark", None),
                &item("theme_cycle", "&Next Theme", Some("CmdOrCtrl+Shift+L")),
                &PredefinedMenuItem::separator(),
                &item("bigger", "&Bigger Text", Some("CmdOrCtrl+Equal")),
                &item("smaller", "S&maller Text", Some("CmdOrCtrl+Minus")),
                &item("actual", "&Actual Size", Some("CmdOrCtrl+Digit0")),
                &item("text", "&Text && Layout…", Some("CmdOrCtrl+KeyT")),
                &PredefinedMenuItem::separator(),
                &item("mode", "&Verse per Line / Paragraphs", Some("CmdOrCtrl+L")),
                &item("red", "&Red Letters", Some("CmdOrCtrl+R")),
                &item("parallel", "&Parallel Translations", Some("CmdOrCtrl+Shift+P")),
                &item("sidebar", "Side&bar", Some("CmdOrCtrl+B")),
                &PredefinedMenuItem::separator(),
                &focus,
            ],
        )
        .ok()?;
        let go = Submenu::with_items(
            "&Go",
            true,
            &[
                &item("jump", "&Jump to Reference…", Some("CmdOrCtrl+K")),
                &item("search", "&Search…", Some("CmdOrCtrl+F")),
                &PredefinedMenuItem::separator(),
                &item("back", "&Back", Some("CmdOrCtrl+BracketLeft")),
                &item("forward", "&Forward", Some("CmdOrCtrl+BracketRight")),
                &item("prev", "&Previous Chapter", None),
                &item("next", "&Next Chapter", None),
                &PredefinedMenuItem::separator(),
                &item("bookmark", "Book&mark", Some("CmdOrCtrl+D")),
                &item("compare", "&Compare Translations", None),
            ],
        )
        .ok()?;
        let help = Submenu::with_items(
            "&Help",
            true,
            &[
                &item("shortcuts", "&Keyboard Shortcuts", Some("CmdOrCtrl+Slash")),
                &PredefinedMenuItem::separator(),
                &item("check_updates", "Check for &Updates…", None),
                &PredefinedMenuItem::separator(),
                &item("about", "&About Bible", None),
            ],
        )
        .ok()?;
        menu.append_items(&[&file, &view, &go, &help]).ok()?;
        let ctx: egui::Context = cc.egui_ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            CHOSEN.lock().unwrap_or_else(|e| e.into_inner()).push(event.id.0);
            ctx.request_repaint();
        }));
        let before = client_size(hwnd as HWND);
        // SAFETY: winit's live window, on the thread that created it (the app's main thread).
        unsafe { menu.init_for_hwnd_with_theme(hwnd, theme(dark)) }.ok()?;
        if let (Some((_, h0)), Some((_, h1))) = (before, client_size(hwnd as HWND))
            && h1 < h0
        {
            let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            // SAFETY: as above; only the size changes.
            unsafe {
                if GetWindowRect(hwnd as HWND, &mut r) != 0 {
                    SetWindowPos(
                        hwnd as HWND,
                        std::ptr::null_mut(),
                        0,
                        0,
                        r.right - r.left,
                        r.bottom - r.top + (h0 - h1),
                        SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
            }
        }
        Some(Self { menu, hwnd, dark, shown: true, focus, focus_enabled: true })
    }

    /// Drains menu clicks into this frame's actions.
    pub fn poll(&self, actions: &mut Vec<Action>) {
        let chosen = std::mem::take(&mut *CHOSEN.lock().unwrap_or_else(|e| e.into_inner()));
        actions.extend(chosen.iter().filter_map(|id| crate::native_menu::action(id)));
    }

    /// Draws the bar light or dark, to match the app's theme.
    pub fn set_dark(&mut self, dark: bool) {
        if dark != self.dark {
            // SAFETY: the window the menu was attached to; it lives as long as the app.
            let _ = unsafe { self.menu.set_theme_for_hwnd(self.hwnd, theme(dark)) };
            self.dark = dark;
        }
    }

    /// Greys out Focus Mode while the first-launch screens show: there is no reader to focus on,
    /// and nothing there would lead back out of full screen.
    pub fn set_focus_enabled(&mut self, enabled: bool) {
        if enabled != self.focus_enabled {
            self.focus.set_enabled(enabled);
            self.focus_enabled = enabled;
        }
    }

    /// Hides the bar (focus mode) or shows it again.
    pub fn set_shown(&mut self, shown: bool) {
        if shown != self.shown {
            // SAFETY: as in `set_dark`.
            let _ = unsafe {
                if shown { self.menu.show_for_hwnd(self.hwnd) } else { self.menu.hide_for_hwnd(self.hwnd) }
            };
            self.shown = shown;
        }
    }
}
