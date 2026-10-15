//! Optional online translations fetched with the user's own API key.
//!
//! Licensed translations (ESV, NASB, LSB, NLT, …) are never bundled. Instead the user adds an
//! ESV API or API.Bible key; chapters are fetched on demand by a background worker, shown with
//! the publisher's attribution, and cached locally only as far as each API's terms allow:
//!
//! * **ESV API** — at most 500 verses stored and never more than half of any book; at most 60
//!   requests a minute (we space requests ≥ 1 s apart).
//! * **API.Bible** — fewer than 500 consecutive verses cached, refreshed within 14 days, and every
//!   displayed chapter reported through FUMS (Fair Use Management System).
//!
//! The memory cache obeys the same verse limits as the disk cache.

pub mod apibible;
pub mod esv;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::books::{BookId, ChapterIdx};
use crate::model::{Origin, Translation, TranslationMeta};

/// Persisted description of an online translation (no secrets: keys live in the keychain).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnlineConfig {
    pub id: String,
    pub abbrev: String,
    pub name: String,
    pub provider: ProviderKind,
    #[serde(default)]
    pub copyright: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProviderKind {
    Esv,
    ApiBible { bible_id: String },
}

impl ProviderKind {
    pub fn secret_name(&self) -> &'static str {
        match self {
            ProviderKind::Esv => "esv-api-key",
            ProviderKind::ApiBible { .. } => "api-bible-key",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ProviderKind::Esv => "ESV API (Crossway)",
            ProviderKind::ApiBible { .. } => "API.Bible",
        }
    }
}

/// How much fetched text may be kept, per the provider's terms.
#[derive(Debug, Clone, Copy)]
pub struct CachePolicy {
    pub max_verses: usize,
    /// Never keep more than half of any book (ESV).
    pub half_book_limit: bool,
    pub max_age_secs: i64,
    pub min_request_interval: Duration,
}

/// A fetched chapter plus provider metadata.
pub struct FetchedChapter {
    pub chapter: Translation,
    pub copyright: Option<String>,
    pub fums_token: Option<String>,
}

pub trait Source: Send + Sync {
    fn policy(&self) -> CachePolicy;
    fn fetch(&self, key: &str, chapter: ChapterIdx) -> Result<FetchedChapter>;
    /// Called when a chapter becomes visible (API.Bible FUMS reporting).
    fn report_view(&self, _fums_token: &str) {}
}

pub fn http_agent() -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .http_status_as_error(false)
        .user_agent(concat!("bible-rs/", env!("CARGO_PKG_VERSION")))
        .build();
    config.into()
}

#[derive(Clone)]
pub enum ChapterState {
    Ready(Arc<Translation>),
    Pending,
    Failed(String),
    NoKey,
}

type KeyFn = Arc<dyn Fn() -> Option<String> + Send + Sync>;
type NotifyFn = Arc<dyn Fn() + Send + Sync>;

struct Shared {
    mem: VecDeque<(ChapterIdx, Arc<Translation>, Option<String>)>,
    inflight: HashSet<ChapterIdx>,
    failed: HashMap<ChapterIdx, (String, Instant)>,
    reported: HashSet<ChapterIdx>,
    copyright: Option<String>,
}

pub struct OnlineTranslation {
    pub config: OnlineConfig,
    pub meta: TranslationMeta,
    source: Arc<dyn Source>,
    shared: Arc<Mutex<Shared>>,
    key: KeyFn,
    queue: Sender<ChapterIdx>,
}

impl OnlineTranslation {
    pub fn new(
        config: OnlineConfig,
        source: Arc<dyn Source>,
        cache: Option<Arc<DiskCache>>,
        key: KeyFn,
        notify: NotifyFn,
    ) -> Arc<Self> {
        let meta = TranslationMeta {
            id: config.id.clone(),
            abbrev: config.abbrev.clone(),
            name: config.name.clone(),
            language: "en".into(),
            license: format!("Licensed text fetched with your own key via {}", config.provider.label()),
            copyright: config.copyright.clone(),
            source: config.provider.label().into(),
            origin: Origin::Online {
                provider: match &config.provider {
                    ProviderKind::Esv => "esv".into(),
                    ProviderKind::ApiBible { .. } => "api.bible".into(),
                },
                remote_id: match &config.provider {
                    ProviderKind::Esv => "ESV".into(),
                    ProviderKind::ApiBible { bible_id } => bible_id.clone(),
                },
            },
            books: Vec::new(),
            red_letters: false,
            paragraphs: true,
            headings: true,
        };
        let shared = Arc::new(Mutex::new(Shared {
            mem: VecDeque::new(),
            inflight: HashSet::new(),
            failed: HashMap::new(),
            reported: HashSet::new(),
            copyright: None,
        }));
        let (tx, rx) = channel();
        let worker = Worker {
            id: config.id.clone(),
            source: source.clone(),
            shared: shared.clone(),
            cache,
            key: key.clone(),
            notify,
        };
        std::thread::Builder::new()
            .name(format!("fetch-{}", config.id))
            .spawn(move || worker.run(rx))
            .expect("spawn fetch worker");
        Arc::new(Self { config, meta, source, shared, key, queue: tx })
    }

    /// Copyright notice returned by the provider (falls back to the configured one).
    pub fn copyright(&self) -> String {
        let s = self.shared.lock().unwrap();
        s.copyright.clone().filter(|c| !c.is_empty()).unwrap_or_else(|| self.config.copyright.clone())
    }

    pub fn has_key(&self) -> bool {
        (self.key)().is_some_and(|k| !k.is_empty())
    }

    /// Non-blocking: returns the chapter if cached, otherwise queues a fetch.
    pub fn chapter(&self, c: ChapterIdx) -> ChapterState {
        let mut s = self.shared.lock().unwrap();
        if let Some(pos) = s.mem.iter().position(|(ch, _, _)| *ch == c) {
            let entry = s.mem.remove(pos).unwrap();
            let t = entry.1.clone();
            s.mem.push_back(entry);
            return ChapterState::Ready(t);
        }
        if let Some((err, at)) = s.failed.get(&c)
            && at.elapsed() < Duration::from_secs(30)
        {
            return ChapterState::Failed(err.clone());
        }
        if !self.has_key() {
            return ChapterState::NoKey;
        }
        if s.inflight.insert(c) {
            let _ = self.queue.send(c);
        }
        ChapterState::Pending
    }

    /// Records that a chapter was shown to the reader (FUMS for API.Bible), once per session.
    pub fn displayed(&self, c: ChapterIdx) {
        let token = {
            let mut s = self.shared.lock().unwrap();
            if !s.reported.insert(c) {
                return;
            }
            s.mem.iter().find(|(ch, _, _)| *ch == c).and_then(|(_, _, t)| t.clone())
        };
        if let Some(token) = token {
            let source = self.source.clone();
            std::thread::spawn(move || source.report_view(&token));
        }
    }

    pub fn retry(&self, c: ChapterIdx) {
        self.shared.lock().unwrap().failed.remove(&c);
    }
}

struct Worker {
    id: String,
    source: Arc<dyn Source>,
    shared: Arc<Mutex<Shared>>,
    cache: Option<Arc<DiskCache>>,
    key: KeyFn,
    notify: NotifyFn,
}

impl Worker {
    fn run(self, rx: Receiver<ChapterIdx>) {
        let policy = self.source.policy();
        let mut last: Option<Instant> = None;
        while let Ok(c) = rx.recv() {
            // Disk cache first (no request needed).
            if let Some(hit) = self.cache.as_ref().and_then(|d| d.get(&self.id, c, policy).ok().flatten()) {
                self.store(c, Arc::new(hit.0), hit.1, policy);
                continue;
            }
            if let Some(t) = last {
                let wait = policy.min_request_interval.saturating_sub(t.elapsed());
                std::thread::sleep(wait);
            }
            last = Some(Instant::now());
            let Some(key) = (self.key)().filter(|k| !k.is_empty()) else {
                self.shared.lock().unwrap().inflight.remove(&c);
                continue;
            };
            match self.source.fetch(&key, c) {
                Ok(f) => {
                    if let Some(cache) = &self.cache {
                        let _ = cache.put(&self.id, c, &f.chapter, f.fums_token.as_deref(), policy);
                    }
                    if let Some(cr) = &f.copyright {
                        self.shared.lock().unwrap().copyright = Some(cr.clone());
                    }
                    self.store(c, Arc::new(f.chapter), f.fums_token, policy);
                }
                Err(e) => {
                    let mut s = self.shared.lock().unwrap();
                    s.inflight.remove(&c);
                    s.failed.insert(c, (format!("{e:#}"), Instant::now()));
                }
            }
            (self.notify)();
        }
    }

    fn store(&self, c: ChapterIdx, t: Arc<Translation>, fums: Option<String>, policy: CachePolicy) {
        let mut s = self.shared.lock().unwrap();
        s.inflight.remove(&c);
        s.failed.remove(&c);
        s.mem.retain(|(ch, _, _)| *ch != c);
        s.mem.push_back((c, t, fums));
        // Text held for display obeys the provider's total verse cap. (The half-a-book storage
        // rule applies to the disk cache; applying it here would make two adjacent chapters on
        // screen evict and refetch each other forever.)
        let display = CachePolicy { half_book_limit: false, ..policy };
        while s.mem.len() > 1 && over_limit(s.mem.iter().map(|(ch, t, _)| (*ch, t.verse_count())), display) {
            s.mem.pop_front();
        }
        drop(s);
        (self.notify)();
    }
}

/// Would keeping these chapters exceed the policy?
fn over_limit(chapters: impl Iterator<Item = (ChapterIdx, usize)>, policy: CachePolicy) -> bool {
    let mut total = 0usize;
    let mut per_book: HashMap<BookId, usize> = HashMap::new();
    for (c, n) in chapters {
        total += n;
        *per_book.entry(c.book()).or_default() += n;
    }
    if total > policy.max_verses {
        return true;
    }
    policy.half_book_limit && per_book.iter().any(|(b, n)| *n * 2 > book_verses(*b))
}

fn book_verses(b: BookId) -> usize {
    let first = b.first_chapter().0;
    (0..b.chapters() as u16).map(|i| ChapterIdx(first + i).kjv_verses() as usize).sum()
}

/// On-disk cache of fetched chapters (separate SQLite file), pruned on every insert.
pub struct DiskCache {
    conn: Mutex<Connection>,
}

impl DiskCache {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS chapters(
                translation TEXT NOT NULL, chapter INTEGER NOT NULL, fetched INTEGER NOT NULL,
                verses INTEGER NOT NULL, payload BLOB NOT NULL, fums TEXT,
                PRIMARY KEY(translation, chapter));",
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn get(
        &self,
        id: &str,
        c: ChapterIdx,
        policy: CachePolicy,
    ) -> Result<Option<(Translation, Option<String>)>> {
        let conn = self.conn.lock().unwrap();
        let cutoff = crate::study::now() - policy.max_age_secs;
        conn.execute("DELETE FROM chapters WHERE translation = ?1 AND fetched < ?2", params![id, cutoff])?;
        let row: Option<(Vec<u8>, Option<String>)> = conn
            .query_row(
                "SELECT payload, fums FROM chapters WHERE translation = ?1 AND chapter = ?2",
                params![id, c.0],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        Ok(match row {
            Some((bytes, fums)) => Some((crate::format::decode(&bytes)?, fums)),
            None => None,
        })
    }

    pub fn put(
        &self,
        id: &str,
        c: ChapterIdx,
        t: &Translation,
        fums: Option<&str>,
        policy: CachePolicy,
    ) -> Result<()> {
        // A chapter that on its own exceeds the limits (e.g. a one-chapter book under the ESV's
        // half-a-book rule) may be displayed but is never stored.
        if over_limit(std::iter::once((c, t.verse_count())), policy) {
            return Ok(());
        }
        let bytes = crate::format::encode(t, 3)?;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO chapters(translation, chapter, fetched, verses, payload, fums)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, c.0, crate::study::now(), t.verse_count() as i64, bytes, fums],
        )?;
        // Evict the oldest other chapters until the stored text is within the provider's limits.
        loop {
            let mut st = conn.prepare(
                "SELECT chapter, verses FROM chapters WHERE translation = ?1 ORDER BY fetched, rowid",
            )?;
            let rows: Vec<(u16, usize)> = st
                .query_map([id], |r| Ok((r.get::<_, u16>(0)?, r.get::<_, i64>(1)? as usize)))?
                .collect::<rusqlite::Result<_>>()?;
            if !over_limit(rows.iter().map(|(ch, n)| (ChapterIdx(*ch), *n)), policy) {
                break;
            }
            let Some((oldest, _)) = rows.iter().find(|(ch, _)| *ch != c.0) else { break };
            conn.execute(
                "DELETE FROM chapters WHERE translation = ?1 AND chapter = ?2",
                params![id, oldest],
            )?;
        }
        Ok(())
    }

    pub fn stored_verses(&self, id: &str) -> usize {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COALESCE(SUM(verses), 0) FROM chapters WHERE translation = ?1", [id], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap_or(0) as usize
    }

    pub fn clear(&self, id: &str) -> Result<()> {
        self.conn.lock().unwrap().execute("DELETE FROM chapters WHERE translation = ?1", [id])?;
        Ok(())
    }
}

/// Reads a response body as text, turning HTTP errors into readable messages.
pub(crate) fn read_body(
    resp: std::result::Result<ureq::http::Response<ureq::Body>, ureq::Error>,
    what: &str,
) -> Result<String> {
    let mut resp = resp.with_context(|| format!("{what}: network error"))?;
    let status = resp.status().as_u16();
    let body = resp.body_mut().read_to_string().unwrap_or_default();
    match status {
        200..=299 => Ok(body),
        401 | 403 => anyhow::bail!("{what}: the API key was rejected (HTTP {status})"),
        404 => anyhow::bail!("{what}: not found (HTTP 404)"),
        429 => anyhow::bail!("{what}: rate limit reached; try again later (HTTP 429)"),
        _ => anyhow::bail!("{what}: HTTP {status}: {}", body.chars().take(200).collect::<String>()),
    }
}

/// Random anonymous identifier (FUMS device/session ids).
pub fn random_id() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut out = String::new();
    for i in 0..2u64 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(i);
        h.write_u128(
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos(),
        );
        h.write_u32(std::process::id());
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::model::TranslationBuilder;

    pub fn chapter_with(c: ChapterIdx, n: u8) -> Translation {
        let mut b = TranslationBuilder::new(TranslationMeta::default());
        b.begin_book(c.book());
        b.begin_chapter(c.chapter() as u32);
        for v in 1..=n {
            b.begin_verse(v as u32, v as u32);
            b.text("word");
        }
        b.finish()
    }

    fn policy(max: usize, half: bool) -> CachePolicy {
        CachePolicy {
            max_verses: max,
            half_book_limit: half,
            max_age_secs: 3600,
            min_request_interval: Duration::ZERO,
        }
    }

    #[test]
    fn disk_cache_enforces_verse_and_half_book_limits() {
        let dir = std::env::temp_dir().join(format!("bible-rs-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cache = DiskCache::open(&dir.join("cache.sqlite")).unwrap();
        let p = policy(100, true);
        let genesis = |n| ChapterIdx::new(BookId::GENESIS, n).unwrap();
        for ch in 1..=4 {
            cache.put("esv", genesis(ch), &chapter_with(genesis(ch), 30), None, p).unwrap();
        }
        // 4 × 30 > 100: the oldest chapter was evicted.
        assert_eq!(cache.stored_verses("esv"), 90);
        assert!(cache.get("esv", genesis(1), p).unwrap().is_none());
        assert!(cache.get("esv", genesis(4), p).unwrap().is_some());
        // Jude is one chapter (25 verses): storing it would keep the whole book, so it isn't
        // stored, and nothing else is evicted on its account.
        let jude = ChapterIdx::new(BookId(64), 1).unwrap();
        cache.put("esv", jude, &chapter_with(jude, 25), None, p).unwrap();
        assert!(cache.get("esv", jude, p).unwrap().is_none());
        assert_eq!(cache.stored_verses("esv"), 90);
        // Ruth 1 (22 of 85 verses) is fine; Ruth 1–2 (45 of 85) would exceed half the book.
        let ruth = |n| ChapterIdx::new(BookId(7), n).unwrap();
        let q = policy(500, true);
        cache.clear("x").unwrap();
        cache.put("x", ruth(1), &chapter_with(ruth(1), 22), None, q).unwrap();
        cache.put("x", ruth(2), &chapter_with(ruth(2), 23), None, q).unwrap();
        assert_eq!(cache.stored_verses("x"), 23);
        // Expired entries disappear.
        let expired = CachePolicy { max_age_secs: -10, ..q };
        assert!(cache.get("x", ruth(2), expired).unwrap().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn random_ids_differ() {
        assert_ne!(random_id(), random_id());
        assert_eq!(random_id().len(), 32);
    }
}
