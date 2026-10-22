//! Words that differ between systems: what the computer is called, its file browser, where API
//! keys are kept. The macOS words are the design's.

/// "this Mac" ("this PC" on Windows), as a literal for `concat!`.
#[cfg(target_os = "macos")]
macro_rules! this_computer {
    () => {
        "this Mac"
    };
}
#[cfg(windows)]
macro_rules! this_computer {
    () => {
        "this PC"
    };
}
#[cfg(not(any(target_os = "macos", windows)))]
macro_rules! this_computer {
    () => {
        "this computer"
    };
}
pub(crate) use this_computer;

/// The button that opens the data folder.
pub const SHOW_IN_FILE_BROWSER: &str = if cfg!(target_os = "macos") {
    "Show in Finder"
} else if cfg!(windows) {
    "Show in Explorer"
} else {
    "Show in Folder"
};

/// Where an online translation's API key is kept, in its row of the Translations page.
pub const KEY_SAVED: &str = if cfg!(target_os = "macos") {
    "key in Keychain"
} else if cfg!(windows) {
    "key in Credential Manager"
} else {
    "key saved"
};

/// The About pane's line under the version.
pub const TAGLINE: &str = if cfg!(target_os = "macos") {
    "A very fast, native Bible reader for the Mac."
} else if cfg!(windows) {
    "A very fast, native Bible reader for Windows."
} else {
    "A very fast, native Bible reader."
};
