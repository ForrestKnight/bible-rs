//! API key storage.
//!
//! On macOS keys live in the login Keychain and on Windows in the Credential Manager (both via the
//! `keyring` crate). Elsewhere — where a Secret Service daemon is often missing (headless machines,
//! CI) — they are kept in a JSON file that only the current user can read (mode 0600). Windows
//! builds used that file too until they had the Credential Manager; a key still in it moves over
//! the first time it is read.

use std::path::{Path, PathBuf};

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct SecretStore {
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    file: PathBuf,
}

impl SecretStore {
    pub fn new(data_dir: &Path) -> Self {
        Self { file: data_dir.join("secrets.json") }
    }

    pub fn backend_name(&self) -> &'static str {
        if cfg!(target_os = "macos") {
            "macOS Keychain"
        } else if cfg!(windows) {
            "Windows Credential Manager"
        } else {
            "local secrets file (mode 0600)"
        }
    }

    /// Stores a value; an empty value deletes it.
    pub fn put(&self, key: &str, value: &str) -> Result<()> {
        if value.trim().is_empty() { self.delete(key) } else { self.set(key, value.trim()) }
    }
}

#[cfg(any(target_os = "macos", windows))]
mod backend {
    use super::*;

    /// The Keychain service (on Windows, the credentials are named `<key>.<service>`). Tests must
    /// never see or overwrite the user's keys: unit tests use their own service, and
    /// `BIBLE_RS_KEYCHAIN_SERVICE` (set by the integration tests) overrides it.
    fn service() -> String {
        std::env::var("BIBLE_RS_KEYCHAIN_SERVICE")
            .unwrap_or_else(|_| if cfg!(test) { "bible-rs-tests" } else { "bible-rs" }.into())
    }

    impl SecretStore {
        pub fn get(&self, key: &str) -> Option<String> {
            match keyring::Entry::new(&service(), key).ok()?.get_password() {
                Ok(value) => Some(value),
                #[cfg(windows)]
                Err(keyring::Error::NoEntry) => self.move_from_file(key),
                Err(_) => None,
            }
        }

        pub fn set(&self, key: &str, value: &str) -> Result<()> {
            keyring::Entry::new(&service(), key)?.set_password(value)?;
            // Saved; an old copy left in the file no longer matters for reading, so tidying it
            // up can fail quietly.
            #[cfg(windows)]
            let _ = self.remove_from_file(key);
            Ok(())
        }

        pub fn delete(&self, key: &str) -> Result<()> {
            match keyring::Entry::new(&service(), key)?.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => {}
                Err(e) => return Err(e.into()),
            }
            // Otherwise the next read would bring the old key back from the file.
            #[cfg(windows)]
            self.remove_from_file(key)?;
            Ok(())
        }

        /// A key that an earlier Windows build kept in the file: saved to the Credential Manager,
        /// then taken out of the file (which keeps it if saving fails).
        #[cfg(windows)]
        fn move_from_file(&self, key: &str) -> Option<String> {
            let value = self.load().remove(key)?;
            let _ = self.set(key, &value);
            Some(value)
        }

        #[cfg(windows)]
        fn remove_from_file(&self, key: &str) -> Result<()> {
            let mut map = self.load();
            if map.remove(key).is_some() {
                if map.is_empty() { std::fs::remove_file(&self.file)? } else { self.save(&map)? }
            }
            Ok(())
        }
    }
}

/// The JSON file of keys: the store away from macOS and Windows, and on Windows the one that
/// earlier builds left behind.
#[cfg(not(target_os = "macos"))]
mod file {
    use std::collections::BTreeMap;

    use super::*;

    impl SecretStore {
        pub(super) fn load(&self) -> BTreeMap<String, String> {
            std::fs::read(&self.file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
        }

        pub(super) fn save(&self, map: &BTreeMap<String, String>) -> Result<()> {
            if let Some(dir) = self.file.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let tmp = self.file.with_extension("json.tmp");
            {
                use std::io::Write;
                let mut opts = std::fs::OpenOptions::new();
                opts.write(true).create(true).truncate(true);
                #[cfg(unix)]
                std::os::unix::fs::OpenOptionsExt::mode(&mut opts, 0o600);
                let mut f = opts.open(&tmp)?;
                f.write_all(&serde_json::to_vec_pretty(map)?)?;
            }
            std::fs::rename(&tmp, &self.file)?;
            Ok(())
        }
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
mod backend {
    use super::*;

    impl SecretStore {
        pub fn get(&self, key: &str) -> Option<String> {
            self.load().remove(key)
        }

        pub fn set(&self, key: &str, value: &str) -> Result<()> {
            let mut map = self.load();
            map.insert(key.to_string(), value.to_string());
            self.save(&map)
        }

        pub fn delete(&self, key: &str) -> Result<()> {
            let mut map = self.load();
            if map.remove(key).is_some() {
                self.save(&map)?;
            }
            Ok(())
        }
    }
}

#[cfg(all(test, not(any(target_os = "macos", windows))))]
mod tests {
    use super::*;

    #[test]
    fn file_store_round_trip_with_private_permissions() {
        let dir = std::env::temp_dir().join(format!("bible-rs-secrets-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = SecretStore::new(&dir);
        assert_eq!(store.get("esv-api-key"), None);
        store.put("esv-api-key", " abc123 ").unwrap();
        assert_eq!(store.get("esv-api-key").as_deref(), Some("abc123"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.join("secrets.json")).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        store.put("esv-api-key", "").unwrap();
        assert_eq!(store.get("esv-api-key"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// The Credential Manager round trip, under the tests' own service name.
#[cfg(all(test, windows))]
mod credential_manager_tests {
    use super::*;

    #[test]
    fn keys_left_in_the_file_move_to_the_credential_manager() {
        let dir = std::env::temp_dir().join(format!("bible-rs-secrets-move-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let (moved, cleared) =
            (format!("moved-{}", std::process::id()), format!("cleared-{}", std::process::id()));
        let json = format!(r#"{{"{moved}": "from-the-file", "{cleared}": "old"}}"#);
        std::fs::write(dir.join("secrets.json"), json).unwrap();
        let store = SecretStore::new(&dir);

        assert_eq!(store.get(&moved).as_deref(), Some("from-the-file"));
        assert!(!store.load().contains_key(&moved), "moved out of the file");
        let fresh = SecretStore::new(&std::env::temp_dir().join("bible-rs-secrets-none"));
        assert_eq!(fresh.get(&moved).as_deref(), Some("from-the-file"), "now in the Credential Manager");

        // Clearing a key that is only in the file doesn't let it come back.
        store.put(&cleared, "").unwrap();
        assert_eq!(store.get(&cleared), None);
        assert!(!dir.join("secrets.json").exists(), "the emptied file is removed");

        store.put(&moved, "").unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn credential_manager_round_trip() {
        let store = SecretStore::new(Path::new("unused"));
        let key = format!("round-trip-{}", std::process::id());
        assert_eq!(store.get(&key), None);
        store.put(&key, " abc123 ").unwrap();
        assert_eq!(store.get(&key).as_deref(), Some("abc123"));
        store.put(&key, "").unwrap();
        assert_eq!(store.get(&key), None);
        assert_eq!(store.backend_name(), "Windows Credential Manager");
    }
}
