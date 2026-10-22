//! Windows window chrome: the standard title bar, drawn light or dark to match the app's theme.

use std::cell::Cell;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::Foundation::{HWND, LPARAM};
use windows_sys::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{EnumThreadWindows, GetWindowTextW};
use windows_sys::core::BOOL;

/// Asks the window manager for a dark (or light) title bar. Windows 10 20H1 and later honour it;
/// earlier versions keep their light one.
fn apply(hwnd: HWND, dark: bool) {
    let value: BOOL = dark.into();
    // SAFETY: a live window of this app; DWM copies the 4-byte BOOL it is given.
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            (&value as *const BOOL).cast(),
            size_of::<BOOL>() as u32,
        );
    }
}

/// The main window's title bar, dark or light.
pub fn set_dark_title_bar(handle: &impl HasWindowHandle, dark: bool) {
    if let Some(RawWindowHandle::Win32(h)) = handle.window_handle().ok().map(|w| w.as_raw()) {
        apply(h.hwnd.get() as HWND, dark);
    }
}

thread_local! {
    /// The window [`set_dark_title_bar_titled`] last drew, how, and whether it had the focus.
    static TITLED: Cell<Option<(isize, bool, bool)>> = const { Cell::new(None) };
}

/// The title bar of the app's window titled `title` (another viewport, such as Settings), dark or
/// light. Called every frame that window draws, with whether it has the focus; it only asks the
/// window manager again when the window or the theme changed, or the window got the focus back.
pub fn set_dark_title_bar_titled(title: &str, dark: bool, focused: bool) {
    let Some(hwnd) = thread_window_titled(title) else { return };
    let last = TITLED.get();
    let refocused = focused && !last.is_some_and(|(_, _, f)| f);
    if last.map(|(h, d, _)| (h, d)) != Some((hwnd as isize, dark)) || refocused {
        apply(hwnd, dark);
    }
    TITLED.set(Some((hwnd as isize, dark, focused)));
}

/// This thread's top-level window with exactly this title.
fn thread_window_titled(title: &str) -> Option<HWND> {
    struct Search {
        title: Vec<u16>,
        found: HWND,
    }
    unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> BOOL {
        // SAFETY: `data` is the `Search` passed to `EnumThreadWindows` below, alive for the call.
        let search = unsafe { &mut *(data as *mut Search) };
        let mut buf = [0u16; 64];
        // SAFETY: `buf` holds `buf.len()` characters.
        let n = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
        if buf[..n.max(0) as usize] == search.title[..] {
            search.found = hwnd;
            return 0;
        }
        1
    }
    let mut search = Search { title: title.encode_utf16().collect(), found: std::ptr::null_mut() };
    // SAFETY: `visit` only reads the windows' titles and writes into `search`.
    unsafe { EnumThreadWindows(GetCurrentThreadId(), Some(visit), &mut search as *mut Search as LPARAM) };
    (!search.found.is_null()).then_some(search.found)
}
