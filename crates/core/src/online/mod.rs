//! Optional online translations fetched from publishers' own APIs.
//!
//! Licensed translations (ESV, NLT, NASB, LSB, …) are never bundled. Instead chapters are fetched
//! on demand by a background worker from an API the publisher provides, shown with the
//! publisher's attribution, and cached locally only as far as each API's terms allow:
//!
//! * **ESV API** (user's free key): at most 500 verses stored and never more than half of any
//!   book; at most 60 requests a minute (we space requests ≥ 1 s apart).
//! * **NLT API** (no key needed; an optional free key raises the limits): at most 500 verses
//!   stored and never a complete book; 50 verses a request and 500 requests a day without a key,
//!   500 and 5,000 with one; requests ≥ 1 s apart.
//! * **NET Bible API** (bible.org, no key): the NET's terms set no storage limit; requests ≥ 1 s
//!   apart.
//! * **API.Bible** (user's key): fewer than 500 consecutive verses cached, refreshed within 14
//!   days, and every displayed chapter reported through FUMS (Fair Use Management System).
//!
//! The memory cache obeys the same verse limits as the disk cache.

pub mod apibible;
pub mod esv;
pub mod net;
pub mod nlt;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
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
    Nlt,
    Net,
}

impl ProviderKind {
    pub fn secret_name(&self) -> &'static str {
        match self {
            ProviderKind::Esv => "esv-api-key",
            ProviderKind::ApiBible { .. } => "api-bible-key",
            ProviderKind::Nlt => "nlt-api-key",
            ProviderKind::Net => "net-api-key",
        }
    }

    /// The server chapters come from, as shown to the reader.
    pub fn host(&self) -> &'static str {
        match self {
            ProviderKind::Esv => "api.esv.org",
            ProviderKind::ApiBible { .. } => "api.scripture.api.bible",
            ProviderKind::Nlt => "api.nlt.to",
            ProviderKind::Net => "labs.bible.org",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ProviderKind::Esv => "ESV API (Crossway)",
            ProviderKind::ApiBible { .. } => "API.Bible",
            ProviderKind::Nlt => "NLT API (Tyndale)",
            ProviderKind::Net => "NET Bible API (bible.org)",
        }
    }

    /// The site the attribution under the text links to, as each provider's terms ask.
    pub fn website(&self) -> &'static str {
        match self {
            ProviderKind::Esv => "https://www.esv.org",
            ProviderKind::ApiBible { .. } => "https://api.bible",
            ProviderKind::Nlt => "https://www.newlivingtranslation.com",
            ProviderKind::Net => "http://netbible.org",
        }
    }
}

/// How much of one book may be kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookShare {
    /// No limit per book.
    Any,
    /// Never more than half of any book (ESV).
    Half,
    /// Never a complete book (NLT).
    LessThanWhole,
}

/// How much fetched text may be kept, per the provider's terms.
#[derive(Debug, Clone, Copy)]
pub struct CachePolicy {
    pub max_verses: usize,
    pub book_share: BookShare,
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
    /// Fetches a chapter; `key` is empty when the user has none and the source doesn't need one.
    fn fetch(&self, key: &str, chapter: ChapterIdx) -> Result<FetchedChapter>;
    /// Whether [`Source::fetch`] needs an API key.
    fn needs_key(&self) -> bool {
        true
    }
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
            license: if source.needs_key() {
                format!("Licensed text fetched with your own key via {}", config.provider.label())
            } else {
                format!("Licensed text fetched via {}", config.provider.label())
            },
            copyright: config.copyright.clone(),
            source: config.provider.label().into(),
            origin: Origin::Online {
                provider: match &config.provider {
                    ProviderKind::Esv => "esv".into(),
                    ProviderKind::ApiBible { .. } => "api.bible".into(),
                    ProviderKind::Nlt => "nlt".into(),
                    ProviderKind::Net => "net".into(),
                },
                remote_id: match &config.provider {
                    ProviderKind::Esv => "ESV".into(),
                    ProviderKind::ApiBible { bible_id } => bible_id.clone(),
                    ProviderKind::Nlt => "NLT".into(),
                    ProviderKind::Net => "NET".into(),
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

    /// The attribution shown under each chapter: the copyright notice, and for API.Bible the
    /// citation its free plan asks for.
    pub fn notice(&self) -> String {
        let copyright = self.copyright();
        match self.config.provider {
            ProviderKind::ApiBible { .. } => {
                format!("{copyright} Provided by API.Bible (api.bible).").trim().to_string()
            }
            _ => copyright,
        }
    }

    pub fn has_key(&self) -> bool {
        (self.key)().is_some_and(|k| !k.is_empty())
    }

    /// Whether chapters can only be fetched with an API key (the NLT API also serves anonymous
    /// requests, within lower limits).
    pub fn needs_key(&self) -> bool {
        self.source.needs_key()
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
        if self.source.needs_key() && !self.has_key() {
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

    /// Records a failed fetch as the worker would (demo captures of the offline state).
    pub fn mark_failed(&self, c: ChapterIdx, message: &str) {
        self.shared.lock().unwrap().failed.insert(c, (message.to_string(), Instant::now()));
    }

    pub fn policy(&self) -> CachePolicy {
        self.source.policy()
    }
}

/// Did a fetch fail for want of a connection (rather than, say, a rejected key)?
pub fn is_offline(message: &str) -> bool {
    message.contains("network error")
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
            let key = (self.key)().filter(|k| !k.is_empty());
            if key.is_none() && self.source.needs_key() {
                self.shared.lock().unwrap().inflight.remove(&c);
                continue;
            }
            match self.source.fetch(key.as_deref().unwrap_or(""), c) {
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
        // Text held for display obeys the provider's total verse cap. (The per-book storage rule
        // applies to the disk cache; applying it here would make two adjacent chapters on screen
        // evict and refetch each other forever.)
        let display = CachePolicy { book_share: BookShare::Any, ..policy };
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
    // Per book: verses, and which chapters.
    let mut per_book: HashMap<BookId, (usize, HashSet<ChapterIdx>)> = HashMap::new();
    for (c, n) in chapters {
        total += n;
        let (verses, chapters) = per_book.entry(c.book()).or_default();
        *verses += n;
        chapters.insert(c);
    }
    if total > policy.max_verses {
        return true;
    }
    per_book.iter().any(|(b, (n, chapters))| match policy.book_share {
        BookShare::Any => false,
        BookShare::Half => *n * 2 > book_verses(*b),
        // A whole book is every chapter of it. Verse counts can't tell: a translation may lack
        // verses the KJV has (the NLT has no Romans 16:24).
        BookShare::LessThanWhole => chapters.len() >= b.chapters() as usize,
    })
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
        // A chapter that on its own exceeds the limits (e.g. a one-chapter book, which would be
        // half or all of a book) may be displayed but is never stored.
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

    /// The chapters kept for a translation and still fresh, the most recently fetched first.
    pub fn chapters(&self, id: &str, policy: CachePolicy) -> Vec<ChapterIdx> {
        let conn = self.conn.lock().unwrap();
        let cutoff = crate::study::now() - policy.max_age_secs;
        let Ok(mut st) = conn.prepare(
            "SELECT chapter FROM chapters WHERE translation = ?1 AND fetched >= ?2 ORDER BY fetched DESC, rowid DESC",
        ) else {
            return Vec::new();
        };
        st.query_map(params![id, cutoff], |r| r.get::<_, u16>(0))
            .map(|rows| rows.flatten().map(ChapterIdx).collect())
            .unwrap_or_default()
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

/// Requests a provider allows per day (UTC), counted across sessions in a small file that every
/// app instance using the same data folder shares.
pub struct DailyBudget {
    path: Option<PathBuf>,
    /// (day number, requests made that day)
    state: Mutex<(i64, u32)>,
}

impl DailyBudget {
    pub fn new(path: Option<PathBuf>) -> Self {
        let saved = path.as_deref().and_then(read_count).unwrap_or((0, 0));
        Self { path, state: Mutex::new(saved) }
    }

    fn today() -> i64 {
        crate::study::now().div_euclid(86_400)
    }

    /// Requests made today.
    pub fn used(&self) -> u32 {
        let s = self.state.lock().unwrap();
        if s.0 == Self::today() { s.1 } else { 0 }
    }

    /// Counts one request, or fails without counting if `limit` were exceeded or the count
    /// couldn't be saved (a lost count would let the next launch go over the limit).
    pub fn spend(&self, limit: u32, what: &str) -> Result<()> {
        let mut s = self.state.lock().unwrap();
        let today = Self::today();
        let saving = |e: std::io::Error, p: &Path| {
            anyhow::Error::new(e)
                .context(format!("{what}: couldn't save today's request count to {}", p.display()))
        };
        // Another instance may be counting too: hold the file's lock while reading and updating
        // it, and start from its count, which includes theirs.
        let _lock = match &self.path {
            Some(p) => {
                let lock = std::fs::File::create(p.with_extension("lock"))
                    .and_then(|f| f.lock().map(|()| f))
                    .map_err(|e| saving(e, p))?;
                if let Some(saved) = read_count(p).filter(|(day, _)| *day == today) {
                    *s = saved;
                }
                Some(lock)
            }
            None => None,
        };
        if s.0 != today {
            *s = (today, 0);
        }
        if s.1 >= limit {
            anyhow::bail!(
                "{what}: the daily limit of {limit} requests is used up; it resets at midnight UTC"
            );
        }
        if let Some(p) = &self.path {
            // Written whole, then moved into place, so a crash never leaves half a count.
            let tmp = p.with_extension(format!("tmp-{}", std::process::id()));
            std::fs::write(&tmp, format!("{} {}", s.0, s.1 + 1))
                .and_then(|()| std::fs::rename(&tmp, p))
                .map_err(|e| saving(e, p))?;
        }
        s.1 += 1;
        Ok(())
    }
}

/// A saved count: "<day> <requests>".
fn read_count(p: &Path) -> Option<(i64, u32)> {
    let s = std::fs::read_to_string(p).ok()?;
    let (day, n) = s.trim().split_once(' ')?;
    Some((day.parse().ok()?, n.parse().ok()?))
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

    fn policy(max: usize, book_share: BookShare) -> CachePolicy {
        CachePolicy { max_verses: max, book_share, max_age_secs: 3600, min_request_interval: Duration::ZERO }
    }

    #[test]
    fn disk_cache_enforces_verse_and_half_book_limits() {
        let dir = std::env::temp_dir().join(format!("bible-rs-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cache = DiskCache::open(&dir.join("cache.sqlite")).unwrap();
        let p = policy(100, BookShare::Half);
        let genesis = |n| ChapterIdx::new(BookId::GENESIS, n).unwrap();
        for ch in 1..=4 {
            cache.put("esv", genesis(ch), &chapter_with(genesis(ch), 30), None, p).unwrap();
        }
        // 4 × 30 > 100: the oldest chapter was evicted.
        assert_eq!(cache.stored_verses("esv"), 90);
        // What can be read offline, newest first.
        assert_eq!(cache.chapters("esv", p), [genesis(4), genesis(3), genesis(2)]);
        assert!(cache.chapters("esv", CachePolicy { max_age_secs: -10, ..p }).is_empty());
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
        let q = policy(500, BookShare::Half);
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
    fn never_a_whole_book() {
        let p = policy(500, BookShare::LessThanWhole);
        let ruth = |n| ChapterIdx::new(BookId(7), n).unwrap();
        // Ruth has 85 verses: three of its four chapters may be kept, all four may not.
        let three = [(ruth(1), 22), (ruth(2), 23), (ruth(3), 18)];
        assert!(!over_limit(three.into_iter(), p));
        assert!(over_limit(three.into_iter().chain([(ruth(4), 22)]), p));
        // A one-chapter book is a whole book on its own.
        let jude = ChapterIdx::new(BookId(64), 1).unwrap();
        assert!(over_limit(std::iter::once((jude, 25)), p));
        // Every chapter is the whole book even with fewer verses than the KJV: the NLT has no
        // Romans 16:24, so all of Romans is 432 verses against the KJV's 433.
        let romans = |n| ChapterIdx::new(BookId(44), n).unwrap();
        let all: Vec<_> =
            (1..=16).map(|n| (romans(n), romans(n).kjv_verses() as usize - usize::from(n == 16))).collect();
        assert_eq!(all.iter().map(|(_, v)| v).sum::<usize>(), 432);
        assert!(over_limit(all.iter().copied(), p));
        assert!(!over_limit(all[..15].iter().copied(), p));
        // The verse cap still applies across books.
        let gen_ = |n| ChapterIdx::new(BookId::GENESIS, n).unwrap();
        assert!(over_limit((1..=20).map(|n| (gen_(n), 30)), p));
    }

    #[test]
    fn daily_budget_counts_across_sessions_and_stops_at_the_limit() {
        let dir = std::env::temp_dir().join(format!("bible-rs-budget-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("requests");
        let b = DailyBudget::new(Some(path.clone()));
        b.spend(3, "X").unwrap();
        b.spend(3, "X").unwrap();
        // A new session continues today's count.
        let b = DailyBudget::new(Some(path.clone()));
        assert_eq!(b.used(), 2);
        b.spend(3, "X").unwrap();
        let err = b.spend(3, "X").unwrap_err().to_string();
        assert!(err.contains("daily limit of 3"), "{err}");
        assert_eq!(b.used(), 3);
        // Yesterday's count doesn't carry over.
        std::fs::write(&path, format!("{} 999", DailyBudget::today() - 1)).unwrap();
        assert_eq!(DailyBudget::new(Some(path)).used(), 0);
        // Instances sharing the file count each other's requests, even all at once.
        let shared = dir.join("shared");
        let (a, b) = (DailyBudget::new(Some(shared.clone())), DailyBudget::new(Some(shared.clone())));
        a.spend(3, "X").unwrap();
        b.spend(3, "X").unwrap();
        a.spend(3, "X").unwrap();
        assert!(b.spend(3, "X").is_err(), "b sees a's two requests");
        std::fs::remove_file(&shared).unwrap();
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let shared = shared.clone();
                std::thread::spawn(move || DailyBudget::new(Some(shared)).spend(100, "X"))
            })
            .collect();
        assert!(threads.into_iter().all(|t| t.join().unwrap().is_ok()));
        assert_eq!(DailyBudget::new(Some(shared)).used(), 8);
        // A count that can't be saved allows no request (here the file's place is a folder).
        let blocked = dir.join("blocked");
        std::fs::create_dir_all(&blocked).unwrap();
        let b = DailyBudget::new(Some(blocked));
        let err = format!("{:#}", b.spend(500, "X").unwrap_err());
        assert!(err.contains("couldn't save today's request count"), "{err}");
        assert_eq!(b.used(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn random_ids_differ() {
        assert_ne!(random_id(), random_id());
        assert_eq!(random_id().len(), 32);
    }
}
