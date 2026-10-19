//! Updating the installed app.
//!
//! On macOS `scripts/bundle-macos.sh` embeds Sparkle.framework, and the bundle's Info.plist names
//! the update feed and the key every update must be signed with (RELEASING.md). Sparkle is loaded
//! once the first frame is on screen, so it adds nothing to launch. Builds without it (`cargo
//! run`, tests, other platforms) open the latest release's page instead.

/// The project's releases on GitHub.
pub const RELEASES: &str = "https://github.com/michaelshimeles/bible-rs/releases";

/// Starts Sparkle, which checks for updates on its own schedule once the user has agreed to
/// automatic checks (it asks on the second launch), and moves a copy of the app left under its
/// old name to the Trash. Main thread; later calls do nothing.
pub fn start() {
    #[cfg(target_os = "macos")]
    {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            sparkle::start();
            let _ = std::thread::Builder::new().name("old-copy".into()).spawn(old_copy::retire);
        });
    }
}

/// "Check for Updates…": Sparkle's update window, or the latest release's page without Sparkle.
pub fn check_for_updates() {
    #[cfg(target_os = "macos")]
    if sparkle::check() {
        return;
    }
    crate::app::open_url(&format!("{RELEASES}/latest"));
}

#[cfg(target_os = "macos")]
mod sparkle {
    use std::cell::OnceCell;

    use objc2::rc::{Allocated, Retained};
    use objc2::runtime::{AnyClass, AnyObject, Bool};
    use objc2::{MainThreadMarker, msg_send};
    use objc2_foundation::{NSBundle, NSString};

    thread_local! {
        /// The `SPUStandardUpdaterController`, kept for the life of the app; `None` without Sparkle.
        static CONTROLLER: OnceCell<Option<Retained<AnyObject>>> = const { OnceCell::new() };
    }

    /// Runs `f` with the updater controller, loading Sparkle on first use. Main thread only.
    fn with<R>(f: impl FnOnce(&AnyObject) -> R) -> Option<R> {
        MainThreadMarker::new()?;
        CONTROLLER.with(|c| c.get_or_init(load).as_deref().map(f))
    }

    pub fn start() {
        with(|_| ());
    }

    pub fn check() -> bool {
        // SAFETY: `-[SPUStandardUpdaterController checkForUpdates:]` takes a nullable sender and
        // returns nothing.
        with(|controller| unsafe {
            let () = msg_send![controller, checkForUpdates: None::<&AnyObject>];
        })
        .is_some()
    }

    fn load() -> Option<Retained<AnyObject>> {
        let frameworks = NSBundle::mainBundle().privateFrameworksPath()?;
        let bundle =
            NSBundle::bundleWithPath(&NSString::from_str(&format!("{frameworks}/Sparkle.framework")))?;
        // SAFETY: the framework is sealed in this app's signature; loading it registers its
        // classes. The hardened runtime refuses it unless it is signed by the app's team, so an
        // ad-hoc signed build gets the releases page.
        if !unsafe { bundle.load() } {
            return None;
        }
        let class = AnyClass::get(c"SPUStandardUpdaterController")?;
        // SAFETY: `initWithStartingUpdater:updaterDelegate:userDriverDelegate:` takes a BOOL and two
        // nullable delegates, and returns the initialised controller.
        unsafe {
            let controller: Allocated<AnyObject> = msg_send![class, alloc];
            msg_send![
                controller,
                initWithStartingUpdater: Bool::YES,
                updaterDelegate: None::<&AnyObject>,
                userDriverDelegate: None::<&AnyObject>
            ]
        }
    }
}

/// Up to 0.1.0 the bundle was `bible-rs.app`. Dragging `Bible.app` from the disk image into
/// Applications leaves that copy beside it, so the new one moves it to the Trash.
#[cfg(target_os = "macos")]
mod old_copy {
    use std::os::unix::fs::MetadataExt;
    use std::path::{Path, PathBuf};

    use objc2_foundation::{NSBundle, NSFileManager, NSString, NSURL};

    const OLD_NAME: &str = "bible-rs.app";

    pub fn retire() {
        objc2::rc::autoreleasepool(|_| {
            let Some(old) = find() else { return };
            let url = NSURL::fileURLWithPath(&NSString::from_str(&old.to_string_lossy()));
            let _ = NSFileManager::defaultManager().trashItemAtURL_resultingItemURL_error(&url, None);
        });
    }

    /// The old copy, when this app runs from an Applications folder that also holds one with the
    /// same bundle identifier.
    fn find() -> Option<PathBuf> {
        let exe = std::env::current_exe().ok()?;
        // …/Bible.app/Contents/MacOS/Bible
        let app = exe.ancestors().nth(3).filter(|p| p.extension().is_some_and(|e| e == "app"))?;
        let dir = app.parent()?;
        if dir != Path::new("/Applications") && dir != dirs::home_dir()?.join("Applications") {
            return None;
        }
        let old = dir.join(OLD_NAME);
        let (ours, theirs) = (std::fs::metadata(app).ok()?, std::fs::metadata(&old).ok()?);
        if (ours.dev(), ours.ino()) == (theirs.dev(), theirs.ino()) {
            return None;
        }
        let id =
            |path: &Path| NSBundle::bundleWithPath(&NSString::from_str(path.to_str()?))?.bundleIdentifier();
        let running = NSBundle::mainBundle().bundleIdentifier()?;
        (id(&old)? == running).then_some(old)
    }
}
