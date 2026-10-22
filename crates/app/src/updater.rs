//! Updating the installed app.
//!
//! On macOS `scripts/bundle-macos.sh` embeds Sparkle.framework, and the bundle's Info.plist names
//! the update feed and the key every update must be signed with (RELEASING.md). Sparkle is loaded
//! once the first frame is on screen, so it adds nothing to launch. Builds without it (`cargo
//! run`, tests, Linux) open the latest release's page instead.
//!
//! On Windows the app asks GitHub for the latest release, at most once a day and whenever "Check
//! for Updates…" is chosen, and offers its installer (or its page) when it is newer (WINDOWS.md).

/// The project's releases on GitHub.
pub const RELEASES: &str = "https://github.com/michaelshimeles/bible-rs/releases";

/// Starts Sparkle, which checks for updates once a day without asking first
/// (`SUEnableAutomaticChecks`) and offers to install a new release, and moves a copy of the app
/// left under its old name to the Trash. Main thread; later calls do nothing.
///
/// Windows: checks GitHub in the background if the last check was a day ago or more.
pub fn start() {
    #[cfg(target_os = "macos")]
    {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            sparkle::start();
            let _ = std::thread::Builder::new().name("old-copy".into()).spawn(old_copy::retire);
        });
    }
    #[cfg(windows)]
    {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| github::check(false));
    }
}

/// "Check for Updates…": Sparkle's update window, or the latest release's page without Sparkle.
/// Windows: GitHub's latest release, offered if newer.
pub fn check_for_updates() {
    #[cfg(windows)]
    github::check(true);
    #[cfg(not(windows))]
    {
        #[cfg(target_os = "macos")]
        if sparkle::check() {
            return;
        }
        crate::app::open_url(&format!("{RELEASES}/latest"));
    }
}

/// The latest release, as much of GitHub's answer as the Windows update check reads.
#[cfg(any(windows, test))]
#[derive(serde::Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[cfg(any(windows, test))]
#[derive(serde::Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// The Windows installer's file name ends with this (`scripts/package-windows.sh`).
#[cfg(any(windows, test))]
const INSTALLER_SUFFIX: &str = "-windows-x64-setup.exe";

/// "v1.2.3" or "1.2.3" (a "-beta" suffix ignored) as numbers.
#[cfg(any(windows, test))]
fn version(tag: &str) -> Option<(u64, u64, u64)> {
    let core = tag.trim().trim_start_matches('v').split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    let v = (parts.next()??, parts.next().unwrap_or(Some(0))?, parts.next().unwrap_or(Some(0))?);
    parts.next().is_none().then_some(v)
}

/// The release's version and where to get it (its installer, else its page) when it is newer than
/// `current`.
#[cfg(any(windows, test))]
fn newer(release: &Release, current: &str) -> Option<(String, String)> {
    if version(&release.tag_name)? <= version(current)? {
        return None;
    }
    let url = release
        .assets
        .iter()
        .find(|a| a.name.ends_with(INSTALLER_SUFFIX))
        .map_or(&release.html_url, |a| &a.browser_download_url);
    Some((release.tag_name.trim_start_matches('v').to_string(), url.clone()))
}

#[cfg(windows)]
mod github {
    use std::time::Duration;

    use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

    use super::{RELEASES, Release, newer};

    const LATEST: &str = "https://api.github.com/repos/michaelshimeles/bible-rs/releases/latest";
    const DAY: Duration = Duration::from_secs(24 * 60 * 60);
    const CURRENT: &str = env!("CARGO_PKG_VERSION");

    /// Checks on a background thread and answers with a message box. `asked`: the user chose
    /// "Check for Updates…", so every outcome is reported; otherwise only a newer version is, and
    /// only if the last successful check was a day ago or more.
    pub fn check(asked: bool) {
        let stamp = crate::data_dir().join("update-check");
        let due = std::fs::metadata(&stamp)
            .and_then(|m| m.modified())
            .map_or(true, |t| t.elapsed().map_or(true, |age| age >= DAY));
        if !asked && !due {
            return;
        }
        let _ = std::thread::Builder::new().name("update-check".into()).spawn(move || {
            let result = latest();
            if result.is_ok() {
                let _ = std::fs::write(&stamp, CURRENT);
            }
            match result {
                Ok(release) => match newer(&release, CURRENT) {
                    Some((version, url)) => offer(&version, &url),
                    None if asked => {
                        MessageDialog::new()
                            .set_level(MessageLevel::Info)
                            .set_title("You're up to date")
                            .set_description(format!("Bible {CURRENT} is the newest version."))
                            .set_buttons(MessageButtons::Ok)
                            .show();
                    }
                    None => {}
                },
                Err(e) if asked => {
                    let open = MessageDialog::new()
                        .set_level(MessageLevel::Warning)
                        .set_title("Couldn't check for updates")
                        .set_description(format!("{e:#}\n\nOpen the releases page instead?"))
                        .set_buttons(MessageButtons::YesNo)
                        .show();
                    if open == MessageDialogResult::Yes {
                        crate::app::open_url(RELEASES);
                    }
                }
                Err(_) => {}
            }
        });
    }

    fn offer(version: &str, url: &str) {
        let what =
            if url.ends_with(".exe") { "Download the installer now?" } else { "Open its download page?" };
        let download = MessageDialog::new()
            .set_level(MessageLevel::Info)
            .set_title("A new version of Bible is available")
            .set_description(format!("Bible {version} is out; you have {CURRENT}.\n\n{what}"))
            .set_buttons(MessageButtons::YesNo)
            .show();
        if download == MessageDialogResult::Yes {
            crate::app::open_url(url);
        }
    }

    fn latest() -> anyhow::Result<Release> {
        let mut response = bible_core::online::http_agent()
            .get(LATEST)
            .header("Accept", "application/vnd.github+json")
            .call()?;
        let status = response.status();
        let body = response.body_mut().read_to_string()?;
        anyhow::ensure!(status.is_success(), "GitHub answered {status}");
        Ok(serde_json::from_str(&body)?)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, assets: &[&str]) -> Release {
        Release {
            tag_name: tag.into(),
            html_url: format!("{RELEASES}/tag/{tag}"),
            assets: assets
                .iter()
                .map(|n| Asset { name: (*n).into(), browser_download_url: format!("https://dl/{n}") })
                .collect(),
        }
    }

    #[test]
    fn versions_compare_as_numbers() {
        assert_eq!(version("v0.10.0"), Some((0, 10, 0)));
        assert_eq!(version("1.2"), Some((1, 2, 0)));
        assert_eq!(version("v1.2.3-beta.1"), Some((1, 2, 3)));
        assert_eq!(version("nightly"), None);
        assert_eq!(version("1.2.3.4"), None);
        assert!(version("v0.10.0") > version("0.9.9"));
    }

    #[test]
    fn a_newer_release_offers_its_installer_or_its_page() {
        let r = release("v0.3.0", &["bible-rs-0.3.0-universal.dmg", "bible-rs-0.3.0-windows-x64-setup.exe"]);
        assert_eq!(
            newer(&r, "0.2.0"),
            Some(("0.3.0".into(), "https://dl/bible-rs-0.3.0-windows-x64-setup.exe".into()))
        );
        let mac_only = release("v0.3.0", &["bible-rs-0.3.0-universal.dmg"]);
        assert_eq!(newer(&mac_only, "0.2.0"), Some(("0.3.0".into(), format!("{RELEASES}/tag/v0.3.0"))));
        assert_eq!(newer(&r, "0.3.0"), None);
        assert_eq!(newer(&r, "0.4.1"), None);
        assert_eq!(newer(&release("latest", &[]), "0.2.0"), None);
    }
}
