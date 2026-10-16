//! macOS window chrome: a unified title bar whose traffic lights share the toolbar's centre line.
//!
//! The window runs its content under a transparent title bar (full-size content view). An empty
//! `NSToolbar` in the unified style makes AppKit lay the title bar out at the toolbar height and
//! centre the traffic lights in it, exactly as in Notes or Safari; AppKit keeps them there through
//! resizing, full screen and appearance changes. The egui toolbar then reads the buttons' actual
//! geometry back ([`traffic_lights`]) instead of assuming where they are.

use eframe::egui::{Rect, pos2};
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, define_class};
use objc2_app_kit::{
    NSTitlebarSeparatorStyle, NSToolbar, NSView, NSWindow, NSWindowButton, NSWindowStyleMask,
    NSWindowToolbarStyle,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

fn ns_view(handle: &impl HasWindowHandle) -> Option<&NSView> {
    let RawWindowHandle::AppKit(h) = handle.window_handle().ok()?.as_raw() else { return None };
    // SAFETY: winit hands out the window's live content view; we only use it on the main thread.
    Some(unsafe { h.ns_view.cast::<NSView>().as_ref() })
}

fn ns_window(handle: &impl HasWindowHandle) -> Option<Retained<NSWindow>> {
    ns_view(handle)?.window()
}

/// Gives the window a unified toolbar so AppKit centres the traffic lights in it.
pub fn install_unified_toolbar(handle: &impl HasWindowHandle) {
    let (Some(window), Some(mtm)) = (ns_window(handle), MainThreadMarker::new()) else { return };
    let toolbar = NSToolbar::new(mtm);
    toolbar.setAllowsUserCustomization(false);
    window.setToolbar(Some(&toolbar));
    window.setToolbarStyle(NSWindowToolbarStyle::Unified);
    // The egui toolbar draws its own hairline.
    window.setTitlebarSeparatorStyle(NSTitlebarSeparatorStyle::None);
}

/// Gives the app's window titled `title` (another viewport, such as Settings) the same unified
/// 52 pt title bar as the main window, once.
pub fn unify_window_titled(title: &str) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
    for window in app.windows().iter() {
        if window.title().to_string() == title && window.toolbar().is_none() {
            let toolbar = NSToolbar::new(mtm);
            toolbar.setAllowsUserCustomization(false);
            window.setToolbar(Some(&toolbar));
            window.setToolbarStyle(NSWindowToolbarStyle::Unified);
            window.setTitlebarSeparatorStyle(NSTitlebarSeparatorStyle::None);
        }
    }
}

/// The traffic lights' bounding box in window points, measured from the top-left corner;
/// `None` in full screen, where they are hidden.
///
/// Called every frame: it also hides the toolbar in full screen, where AppKit would otherwise
/// show the empty toolbar as an opaque strip over the app's own, and shows it again on the way out.
pub fn traffic_lights(handle: &impl HasWindowHandle) -> Option<Rect> {
    let window = ns_window(handle)?;
    let full_screen = window.styleMask().contains(NSWindowStyleMask::FullScreen);
    if let Some(toolbar) = window.toolbar()
        && toolbar.isVisible() == full_screen
    {
        toolbar.setVisible(!full_screen);
    }
    if full_screen {
        return None;
    }
    let height = window.contentView()?.frame().size.height;
    let mut out: Option<Rect> = None;
    for b in [NSWindowButton::CloseButton, NSWindowButton::MiniaturizeButton, NSWindowButton::ZoomButton] {
        let button = window.standardWindowButton(b)?;
        if button.isHidden() {
            continue;
        }
        // Window coordinates have their origin at the bottom left.
        let r = button.convertRect_toView(button.bounds(), None);
        let top = (height - r.origin.y - r.size.height) as f32;
        let rect = Rect::from_min_max(
            pos2(r.origin.x as f32, top),
            pos2((r.origin.x + r.size.width) as f32, top + r.size.height as f32),
        );
        out = Some(out.map_or(rect, |o| o.union(rect)));
    }
    out
}

/// How often the display the window is on refreshes at its fastest (seconds per frame): 1/120 s
/// on a ProMotion panel.
pub fn refresh_interval(handle: &impl HasWindowHandle) -> Option<f64> {
    if !objc2::available!(macos = 12.0) {
        return None;
    }
    let fps = ns_window(handle)?.screen()?.maximumFramesPerSecond();
    (fps > 0).then(|| 1.0 / fps as f64)
}

define_class!(
    /// Marks its frame as "not a window drag handle" while letting clicks through to egui.
    #[unsafe(super(NSView, objc2_app_kit::NSResponder, objc2_foundation::NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "BibleRsNoDragView"]
    struct NoDragView;

    impl NoDragView {
        #[unsafe(method(mouseDownCanMoveWindow))]
        fn mouse_down_can_move_window(&self) -> bool {
            false
        }

        #[unsafe(method(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> *mut NSView {
            std::ptr::null_mut()
        }
    }
);

/// The toolbar's buttons inside the title bar, carved out of its window-drag area.
///
/// In a full-size content view the title bar's whole width moves the window and a double-click
/// on it zooms (or whatever the user chose). macOS gathers that area from the views'
/// `mouseDownCanMoveWindow` ahead of time and lets the window server act on it, so the carve-outs
/// are real (invisible, click-through) views kept over the buttons.
#[derive(Default)]
pub struct NoDragAreas {
    views: Vec<Retained<NoDragView>>,
    rects: Vec<Rect>,
}

impl NoDragAreas {
    /// `rects` are in egui points from the window's top-left corner; `zoom` is egui's zoom factor.
    pub fn set(&mut self, handle: &impl HasWindowHandle, rects: &[Rect], zoom: f32) {
        if rects == self.rects.as_slice() {
            return;
        }
        let (Some(content), Some(mtm)) = (ns_view(handle), MainThreadMarker::new()) else { return };
        while self.views.len() < rects.len() {
            // SAFETY: plain `-[NSView initWithFrame:]` on a freshly allocated instance.
            let view: Retained<NoDragView> =
                unsafe { objc2::msg_send![NoDragView::alloc(mtm), initWithFrame: NSRect::ZERO] };
            content.addSubview(&view);
            self.views.push(view);
        }
        for (i, view) in self.views.iter().enumerate() {
            // winit's view is flipped (top-left origin), like egui.
            let frame = rects.get(i).map_or(NSRect::ZERO, |r| {
                let r = *r * zoom;
                NSRect::new(
                    NSPoint::new(r.left().into(), r.top().into()),
                    NSSize::new(r.width().into(), r.height().into()),
                )
            });
            view.setFrame(frame);
        }
        self.rects = rects.to_vec();
    }
}

/// Shows the system share picker for a file, pointing at `anchor` (egui points from the window's
/// top-left; `zoom` is egui's zoom factor).
pub fn share_file(handle: &impl HasWindowHandle, path: &std::path::Path, anchor: Rect, zoom: f32) {
    use objc2::AnyThread;
    use objc2_app_kit::NSSharingServicePicker;
    use objc2_foundation::{NSArray, NSRectEdge, NSString, NSURL};
    let (Some(view), Some(path)) = (ns_view(handle), path.to_str()) else { return };
    let url = NSURL::fileURLWithPath(&NSString::from_str(path));
    let items = NSArray::from_retained_slice(&[url]);
    // SAFETY: an array of NSURLs is an array of objects; the picker accepts NSURLs (they conform
    // to NSPasteboardWriting).
    let items: Retained<NSArray> = unsafe { Retained::cast_unchecked(items) };
    let picker = unsafe { NSSharingServicePicker::initWithItems(NSSharingServicePicker::alloc(), &items) };
    let r = anchor * zoom;
    // winit's view is flipped (top-left origin), like egui.
    let rect = NSRect::new(
        NSPoint::new(r.left().into(), r.top().into()),
        NSSize::new(r.width().into(), r.height().into()),
    );
    picker.showRelativeToRect_ofView_preferredEdge(rect, view, NSRectEdge::MinY);
}
