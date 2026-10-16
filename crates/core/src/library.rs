//! The set of installed translations, whatever their origin, behind one interface.
//!
//! * **Bundled** — public-domain `.bbl` blobs compiled into the binary.
//! * **Imported** — `.bbl` files the user converted from their own OSIS/USFM/Zefania/SWORD
//!   files, stored in `<data dir>/translations/`.
//! * **Online** — fetched chapter by chapter from the ESV API or API.Bible with the user's key,
//!   or from the NLT API or the NET Bible API (no key needed).
//!
//! Local translations decode lazily (once, ≈10 ms) and can be warmed on a background thread.
//! [`Library::chapter`] never blocks on the network.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::{Context, Result, bail};

use crate::books::ChapterIdx;
use crate::format;
use crate::import::Imported;
use crate::model::{Origin, Translation, TranslationMeta};
use crate::online::{self, ChapterState, DiskCache, OnlineConfig, OnlineTranslation, ProviderKind};
use crate::secrets::SecretStore;

pub enum EntryKind {
    Bundled(&'static [u8]),
    Imported(PathBuf),
    Online(Arc<OnlineTranslation>),
}

pub struct Entry {
    pub meta: TranslationMeta,
    pub kind: EntryKind,
    local: OnceLock<std::result::Result<Arc<Translation>, String>>,
}

impl Entry {
    pub fn is_local(&self) -> bool {
        !matches!(self.kind, EntryKind::Online(_))
    }

    pub fn online(&self) -> Option<&Arc<OnlineTranslation>> {
        match &self.kind {
            EntryKind::Online(o) => Some(o),
            _ => None,
        }
    }

    /// Decodes (once) and returns a local translation.
    pub fn load(&self) -> Result<Arc<Translation>> {
        let r = self.local.get_or_init(|| {
            let bytes: std::borrow::Cow<'_, [u8]> = match &self.kind {
                EntryKind::Bundled(b) => std::borrow::Cow::Borrowed(*b),
                EntryKind::Imported(p) => match std::fs::read(p) {
                    Ok(b) => std::borrow::Cow::Owned(b),
                    Err(e) => return Err(format!("reading {}: {e}", p.display())),
                },
                EntryKind::Online(_) => return Err("online translations are fetched per chapter".into()),
            };
            format::decode(&bytes)
                .map(|mut t| {
                    // Keep the listing metadata (it carries the origin) authoritative.
                    t.meta.origin = self.meta.origin.clone();
                    Arc::new(t)
                })
                .map_err(|e| format!("{e:#}"))
        });
        r.clone().map_err(anyhow::Error::msg)
    }

    pub fn loaded(&self) -> Option<Arc<Translation>> {
        self.local.get().and_then(|r| r.as_ref().ok().cloned())
    }

    pub fn copyright(&self) -> String {
        match &self.kind {
            EntryKind::Online(o) => o.copyright(),
            _ => self.meta.copyright.clone(),
        }
    }
}

pub enum Chapter {
    Ready(Arc<Translation>),
    Pending,
    Failed(String),
    NoKey,
    /// The translation doesn't contain this chapter (e.g. a New Testament only import).
    Missing,
}

type Notify = Arc<dyn Fn() + Send + Sync>;

/// API keys cached in memory so the keychain isn't queried every frame.
#[derive(Default)]
struct KeyCache {
    values: Mutex<std::collections::HashMap<&'static str, Option<String>>>,
}

pub struct Library {
    data_dir: PathBuf,
    entries: Vec<Arc<Entry>>,
    secrets: SecretStore,
    keys: Arc<KeyCache>,
    cache: Option<Arc<DiskCache>>,
    notify: Notify,
    device_id: String,
    pub api_bible_base: String,
}

impl Library {
    pub fn open(data_dir: &Path, notify: Notify) -> Self {
        let mut lib = Library {
            data_dir: data_dir.to_path_buf(),
            entries: Vec::new(),
            secrets: SecretStore::new(data_dir),
            keys: Arc::new(KeyCache::default()),
            cache: None,
            notify,
            device_id: String::new(),
            api_bible_base: online::apibible::DEFAULT_BASE.into(),
        };
        #[cfg(feature = "bundled")]
        for (_, bytes) in crate::bundled::BUNDLED {
            if let Ok(meta) = format::read_meta(bytes) {
                lib.entries.push(Arc::new(Entry {
                    meta,
                    kind: EntryKind::Bundled(bytes),
                    local: OnceLock::new(),
                }));
            }
        }
        if let Ok(dir) = std::fs::read_dir(crate::import::imports_dir(data_dir)) {
            let mut paths: Vec<PathBuf> = dir
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "bbl"))
                .collect();
            paths.sort();
            for p in paths {
                if let Some(meta) = read_meta_file(&p)
                    && !lib.contains(&meta.id)
                {
                    lib.entries.push(Arc::new(Entry {
                        meta,
                        kind: EntryKind::Imported(p),
                        local: OnceLock::new(),
                    }));
                }
            }
        }
        for cfg in lib.online_configs() {
            lib.attach_online(cfg);
        }
        lib
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn secrets(&self) -> &SecretStore {
        &self.secrets
    }

    pub fn entries(&self) -> &[Arc<Entry>] {
        &self.entries
    }

    pub fn get(&self, id: &str) -> Option<&Arc<Entry>> {
        self.entries.iter().find(|e| e.meta.id == id)
    }

    pub fn contains(&self, id: &str) -> bool {
        self.get(id).is_some()
    }

    pub fn load(&self, id: &str) -> Result<Arc<Translation>> {
        self.get(id).with_context(|| format!("no translation {id:?}"))?.load()
    }

    /// Decodes local translations on a background thread so switching is instant later.
    pub fn warm(&self, ids: &[String]) {
        let entries: Vec<Arc<Entry>> =
            ids.iter().filter_map(|id| self.get(id).cloned()).filter(|e| e.is_local()).collect();
        if entries.is_empty() {
            return;
        }
        let notify = self.notify.clone();
        std::thread::Builder::new()
            .name("warm-translations".into())
            .spawn(move || {
                for e in entries {
                    if let Ok(t) = e.load() {
                        t.search_index();
                    }
                }
                notify();
            })
            .ok();
    }

    /// The text of one chapter, without blocking on the network.
    pub fn chapter(&self, id: &str, c: ChapterIdx) -> Chapter {
        let Some(e) = self.get(id) else {
            return Chapter::Failed(format!("translation {id} is not installed"));
        };
        match &e.kind {
            EntryKind::Online(o) => match o.chapter(c) {
                ChapterState::Ready(t) => Chapter::Ready(t),
                ChapterState::Pending => Chapter::Pending,
                ChapterState::Failed(m) => Chapter::Failed(m),
                ChapterState::NoKey => Chapter::NoKey,
            },
            _ => match e.load() {
                Ok(t) if t.has_chapter(c) => Chapter::Ready(t),
                Ok(_) => Chapter::Missing,
                Err(err) => Chapter::Failed(format!("{err:#}")),
            },
        }
    }

    /// An online translation's chapters that can be read without a connection, the most recently
    /// fetched first.
    pub fn cached_chapters(&self, id: &str) -> Vec<ChapterIdx> {
        match (self.get(id).and_then(|e| e.online()), &self.cache) {
            (Some(o), Some(cache)) => cache.chapters(id, o.policy()),
            _ => Vec::new(),
        }
    }

    /// Saves an imported translation into the data directory and registers it.
    pub fn install(&mut self, imported: Imported) -> Result<String> {
        let mut t = imported.translation;
        let base = if t.meta.id.is_empty() { "imported".to_string() } else { t.meta.id.clone() };
        let mut id = base.clone();
        let mut n = 2;
        while self.contains(&id) {
            id = format!("{base}-{n}");
            n += 1;
        }
        t.meta.id = id.clone();
        let dir = crate::import::imports_dir(&self.data_dir);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{id}.bbl"));
        let tmp = path.with_extension("bbl.tmp");
        std::fs::write(&tmp, format::encode(&t, 9)?)?;
        std::fs::rename(&tmp, &path)?;
        let meta = t.meta.clone();
        let entry = Entry { meta, kind: EntryKind::Imported(path), local: OnceLock::new() };
        let _ = entry.local.set(Ok(Arc::new(t)));
        self.entries.push(Arc::new(entry));
        Ok(id)
    }

    /// Removes an imported or online translation (bundled ones cannot be removed).
    pub fn remove(&mut self, id: &str) -> Result<()> {
        let Some(pos) = self.entries.iter().position(|e| e.meta.id == id) else {
            bail!("no translation {id}")
        };
        match &self.entries[pos].kind {
            EntryKind::Bundled(_) => bail!("bundled translations cannot be removed"),
            EntryKind::Imported(p) => {
                std::fs::remove_file(p).with_context(|| format!("removing {}", p.display()))?;
            }
            EntryKind::Online(_) => {
                let mut cfgs = self.online_configs();
                cfgs.retain(|c| c.id != id);
                self.save_online_configs(&cfgs)?;
                if let Some(cache) = &self.cache {
                    let _ = cache.clear(id);
                }
            }
        }
        self.entries.remove(pos);
        Ok(())
    }

    fn online_path(&self) -> PathBuf {
        self.data_dir.join("online.json")
    }

    pub fn online_configs(&self) -> Vec<OnlineConfig> {
        std::fs::read(self.online_path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    fn save_online_configs(&self, cfgs: &[OnlineConfig]) -> Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::write(self.online_path(), serde_json::to_vec_pretty(cfgs)?)?;
        Ok(())
    }

    /// Adds (and persists) an online translation.
    pub fn add_online(&mut self, mut cfg: OnlineConfig) -> Result<String> {
        if self.entries.iter().any(|e| e.online().is_some_and(|o| o.config.provider == cfg.provider)) {
            bail!("{} is already added", cfg.name);
        }
        let base = cfg.id.clone();
        let mut n = 2;
        while self.contains(&cfg.id) {
            cfg.id = format!("{base}-{n}");
            n += 1;
        }
        let mut cfgs = self.online_configs();
        cfgs.push(cfg.clone());
        self.save_online_configs(&cfgs)?;
        let id = cfg.id.clone();
        self.attach_online(cfg);
        Ok(id)
    }

    fn ensure_cache(&mut self) {
        if self.cache.is_none() {
            self.cache = DiskCache::open(&self.data_dir.join("online-cache.sqlite")).ok().map(Arc::new);
        }
    }

    fn attach_online(&mut self, cfg: OnlineConfig) {
        self.ensure_cache();
        if self.device_id.is_empty() {
            let p = self.data_dir.join("device-id");
            self.device_id = std::fs::read_to_string(&p)
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| s.len() >= 16)
                .unwrap_or_else(|| {
                    let id = online::random_id();
                    let _ = std::fs::create_dir_all(&self.data_dir);
                    let _ = std::fs::write(&p, &id);
                    id
                });
        }
        let source: Arc<dyn online::Source> = match &cfg.provider {
            ProviderKind::Esv => Arc::new(online::esv::Esv::default()),
            ProviderKind::ApiBible { bible_id } => Arc::new(online::apibible::ApiBible::with_base(
                &self.api_bible_base,
                online::apibible::FUMS_BASE,
                bible_id,
                &self.device_id,
            )),
            ProviderKind::Nlt => Arc::new(online::nlt::Nlt::new(Some(self.data_dir.join("nlt-requests")))),
            ProviderKind::Net => Arc::new(online::net::Net::default()),
        };
        let name = cfg.provider.secret_name();
        let keys = self.keys.clone();
        let secrets = self.secrets.clone();
        let key = Arc::new(move || {
            let mut map = keys.values.lock().unwrap();
            map.entry(name).or_insert_with(|| secrets.get(name)).clone()
        });
        let ot = OnlineTranslation::new(cfg, source, self.cache.clone(), key, self.notify.clone());
        let meta = ot.meta.clone();
        self.entries.push(Arc::new(Entry { meta, kind: EntryKind::Online(ot), local: OnceLock::new() }));
    }

    /// Adds an online translation with a custom source (tests, alternative endpoints).
    pub fn add_online_with_source(&mut self, cfg: OnlineConfig, source: Arc<dyn online::Source>) -> String {
        self.ensure_cache();
        let name = cfg.provider.secret_name();
        let keys = self.keys.clone();
        let secrets = self.secrets.clone();
        let key = Arc::new(move || {
            let mut map = keys.values.lock().unwrap();
            map.entry(name).or_insert_with(|| secrets.get(name)).clone()
        });
        let id = cfg.id.clone();
        let ot = OnlineTranslation::new(cfg, source, self.cache.clone(), key, self.notify.clone());
        let meta = ot.meta.clone();
        self.entries.push(Arc::new(Entry { meta, kind: EntryKind::Online(ot), local: OnceLock::new() }));
        id
    }

    pub fn api_key(&self, name: &'static str) -> Option<String> {
        let mut map = self.keys.values.lock().unwrap();
        map.entry(name).or_insert_with(|| self.secrets.get(name)).clone()
    }

    /// Stores (or with an empty value, removes) an API key in the keychain.
    pub fn set_api_key(&self, name: &'static str, value: &str) -> Result<()> {
        self.secrets.put(name, value)?;
        let v = value.trim();
        self.keys.values.lock().unwrap().insert(name, (!v.is_empty()).then(|| v.to_string()));
        (self.notify)();
        Ok(())
    }

    pub fn cached_verses(&self, id: &str) -> usize {
        self.cache.as_ref().map_or(0, |c| c.stored_verses(id))
    }

    pub fn is_bundled(&self, id: &str) -> bool {
        self.get(id).is_some_and(|e| e.meta.origin == Origin::Bundled)
    }
}

fn read_meta_file(p: &Path) -> Option<TranslationMeta> {
    use std::io::Read;
    let mut f = std::fs::File::open(p).ok()?;
    // The header (magic, version, JSON metadata) is at the front; 64 KiB is plenty.
    let mut buf = vec![0u8; 64 << 10];
    let n = f.read(&mut buf).ok()?;
    buf.truncate(n);
    format::read_meta(&buf).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::{BookId, VerseRef};
    use crate::import::{Format, Imported};
    use crate::model::TranslationBuilder;

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "bible-rs-lib-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn imported(abbrev: &str) -> Imported {
        let meta = TranslationMeta {
            id: abbrev.to_lowercase(),
            abbrev: abbrev.into(),
            name: format!("{abbrev} test"),
            origin: Origin::Imported { format: "USFM".into(), file_name: "x".into() },
            ..Default::default()
        };
        let mut b = TranslationBuilder::new(meta);
        b.begin_book(BookId(42));
        b.begin_chapter(3);
        b.begin_verse(16, 16);
        b.text("Imported text.");
        Imported { translation: b.finish(), format: Format::Usfm, warnings: vec![], skipped_books: vec![] }
    }

    #[test]
    fn bundled_imported_and_removal() {
        let dir = tmp();
        let mut lib = Library::open(&dir, Arc::new(|| {}));
        assert!(lib.contains("kjv") && lib.contains("web") && lib.contains("bsb"));
        assert!(lib.remove("kjv").is_err());
        let id = lib.install(imported("KJV")).unwrap();
        assert_eq!(id, "kjv-2", "ids never collide with bundled translations");
        let jn3 = ChapterIdx::new(BookId(42), 3).unwrap();
        assert!(matches!(lib.chapter(&id, jn3), Chapter::Ready(_)));
        assert!(matches!(lib.chapter(&id, ChapterIdx(0)), Chapter::Missing));
        // Reopening finds the import on disk.
        let lib2 = Library::open(&dir, Arc::new(|| {}));
        let t = lib2.load(&id).unwrap();
        assert_eq!(t.find(VerseRef::new(BookId(42), 3, 16)).unwrap().text, "Imported text.");
        assert!(matches!(t.meta.origin, Origin::Imported { .. }));
        lib.remove(&id).unwrap();
        assert!(!Library::open(&dir, Arc::new(|| {})).contains(&id));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn online_config_persists_without_keys() {
        let dir = tmp();
        let mut lib = Library::open(&dir, Arc::new(|| {}));
        let cfg = OnlineConfig {
            id: "esv".into(),
            abbrev: "ESV".into(),
            name: "English Standard Version".into(),
            provider: ProviderKind::Esv,
            copyright: online::esv::COPYRIGHT.into(),
        };
        lib.add_online(cfg.clone()).unwrap();
        assert!(lib.add_online(cfg).is_err());
        let c = ChapterIdx(0);
        assert!(matches!(lib.chapter("esv", c), Chapter::NoKey));
        let saved = std::fs::read_to_string(dir.join("online.json")).unwrap();
        assert!(saved.contains("\"esv\"") && !saved.to_lowercase().contains("key\":"));
        let lib2 = Library::open(&dir, Arc::new(|| {}));
        assert!(lib2.get("esv").unwrap().online().is_some());
        lib.remove("esv").unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
