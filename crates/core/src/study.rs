//! Local study data: highlights, notes, bookmarks, reading history, plans and settings.
//!
//! Everything is keyed by canonical verse reference (not by translation), so a highlight made
//! while reading the KJV shows up in the WEB too. A verse can hold several highlights, each over
//! part of it or all of it ([`crate::highlight`] places them in a translation's text).
//!
//! The store lives in memory — hundreds of rows, not millions — so the reader can look it up per
//! verse every frame. The native apps back it with
//! one SQLite file and write straight through (WAL mode, a few hundred microseconds each); the web
//! app has no SQLite and saves the store's JSON export instead.

use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::{Context, Result, bail};
#[cfg(feature = "native")]
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::books::{BookId, ChapterIdx, VerseRef};
use crate::highlight;
pub use crate::highlight::{Highlight, TextSpan};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HighlightColor {
    Yellow,
    Green,
    Blue,
    Pink,
    Purple,
    Orange,
}

impl HighlightColor {
    /// In key order: 1–6 apply these, 0 clears. The stored value is the index, so new colours
    /// only ever go at the end.
    pub const ALL: [HighlightColor; 6] = [
        HighlightColor::Yellow,
        HighlightColor::Green,
        HighlightColor::Blue,
        HighlightColor::Pink,
        HighlightColor::Purple,
        HighlightColor::Orange,
    ];

    pub fn name(self) -> &'static str {
        match self {
            HighlightColor::Yellow => "yellow",
            HighlightColor::Green => "green",
            HighlightColor::Blue => "blue",
            HighlightColor::Pink => "pink",
            HighlightColor::Purple => "purple",
            HighlightColor::Orange => "orange",
        }
    }

    #[cfg(feature = "native")]
    fn from_i64(v: i64) -> Option<Self> {
        Self::ALL.get(usize::try_from(v).ok()?).copied()
    }

    #[cfg(feature = "native")]
    fn to_i64(self) -> i64 {
        Self::ALL.iter().position(|c| *c == self).unwrap() as i64
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub body: String,
    pub created: i64,
    pub updated: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bookmark {
    pub label: String,
    pub created: i64,
}

pub struct StudyStore {
    /// The SQLite file behind the store, when there is one.
    #[cfg(feature = "native")]
    conn: Option<Connection>,
    /// Each highlighted verse's highlights, in order, not overlapping.
    highlights: HashMap<u32, Vec<Highlight>>,
    notes: HashMap<u32, Note>,
    bookmarks: BTreeMap<u32, Bookmark>,
    /// Reading sessions by when they started.
    reading: BTreeMap<i64, ReadingSession>,
    plans: HashMap<String, PlanProgress>,
    plan_reads: HashMap<String, HashSet<(u16, u8)>>,
    settings: HashMap<String, String>,
    generation: u64,
}

#[cfg(feature = "native")]
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS highlights(
    book TEXT NOT NULL, chapter INTEGER NOT NULL, verse INTEGER NOT NULL,
    char_start INTEGER NOT NULL DEFAULT 0, char_end INTEGER, quote TEXT,
    color INTEGER NOT NULL, created INTEGER NOT NULL, edited INTEGER,
    PRIMARY KEY(book, chapter, verse, char_start));
CREATE TABLE IF NOT EXISTS notes(
    book TEXT NOT NULL, chapter INTEGER NOT NULL, verse INTEGER NOT NULL,
    body TEXT NOT NULL, created INTEGER NOT NULL, updated INTEGER NOT NULL,
    PRIMARY KEY(book, chapter, verse));
CREATE TABLE IF NOT EXISTS bookmarks(
    book TEXT NOT NULL, chapter INTEGER NOT NULL, verse INTEGER NOT NULL,
    label TEXT NOT NULL, created INTEGER NOT NULL,
    PRIMARY KEY(book, chapter, verse));
CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS reading(
    started INTEGER PRIMARY KEY, book TEXT NOT NULL, chapter INTEGER NOT NULL,
    translation TEXT NOT NULL, seconds INTEGER NOT NULL, furthest INTEGER NOT NULL,
    verses INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS plans(
    id TEXT PRIMARY KEY, started INTEGER NOT NULL, finished INTEGER);
CREATE TABLE IF NOT EXISTS plan_reads(
    plan TEXT NOT NULL, day INTEGER NOT NULL, passage INTEGER NOT NULL, at INTEGER NOT NULL,
    PRIMARY KEY(plan, day, passage));
";

/// A stretch of reading one chapter: when it started, for how long, and how far it got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingSession {
    /// Unix seconds; also the session's identity.
    pub started: i64,
    pub book: BookId,
    pub chapter: u8,
    pub translation: String,
    /// Time actually spent reading (idle time is not counted).
    pub seconds: u32,
    /// The furthest verse reached, and how many verses the chapter has.
    pub furthest: u8,
    pub verses: u8,
}

impl ReadingSession {
    pub fn chapter_idx(&self) -> Option<ChapterIdx> {
        ChapterIdx::new(self.book, self.chapter)
    }

    /// Reached the chapter's last verse.
    pub fn finished(&self) -> bool {
        self.verses > 0 && self.furthest >= self.verses
    }

    /// How much of the chapter was read, 0–1.
    pub fn progress(&self) -> f32 {
        if self.verses == 0 { 0.0 } else { (self.furthest as f32 / self.verses as f32).min(1.0) }
    }
}

/// A reading plan under way (or finished): which, and when it began (the start of that day).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanProgress {
    pub id: String,
    pub started: i64,
    pub finished: Option<i64>,
}

/// Unix seconds.
pub fn now() -> i64 {
    // The browser has no system clock for std to read: asking panics.
    #[cfg(target_arch = "wasm32")]
    return (js_sys::Date::now() / 1000.0) as i64;
    #[cfg(not(target_arch = "wasm32"))]
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(feature = "native")]
fn verse_from_row(book: &str, chapter: i64, verse: i64) -> Option<VerseRef> {
    let book = BookId::from_code(book)?;
    Some(VerseRef::new(book, u8::try_from(chapter).ok()?, u8::try_from(verse).ok()?))
}

#[cfg(feature = "native")]
fn query<T>(
    conn: &Connection,
    sql: &str,
    row: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>> {
    Ok(conn.prepare(sql)?.query_map([], row)?.collect::<rusqlite::Result<_>>()?)
}

/// Brings a file made by an older version up to date, recording its version in `user_version`.
///
/// Version 1 (2026-10): a verse holds several highlights, each over part of it or all of it. The
/// highlights table gains char offsets, the words covered and when each was last coloured (empty:
/// when it was made), and the start offset joins its key. Every highlight before was a whole
/// verse, and stays one.
#[cfg(feature = "native")]
fn migrate(conn: &Connection) -> Result<()> {
    let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if version >= 1 {
        return Ok(());
    }
    let has_parts: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('highlights') WHERE name = 'char_start'",
        [],
        |r| r.get(0),
    )?;
    let tx = conn.unchecked_transaction()?;
    if has_parts == 0 {
        // The schema makes the new table once the old one is out of its way.
        tx.execute_batch(&format!(
            "ALTER TABLE highlights RENAME TO highlights_v0;
             {SCHEMA}
             INSERT INTO highlights(book, chapter, verse, char_start, char_end, quote, color, created, edited)
                 SELECT book, chapter, verse, 0, NULL, NULL, color, created, NULL FROM highlights_v0;
             DROP TABLE highlights_v0;"
        ))
        .context("upgrading the highlights table")?;
    }
    tx.pragma_update(None, "user_version", 1)?;
    tx.commit()?;
    Ok(())
}

impl Default for StudyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl StudyStore {
    /// An empty store that keeps nothing on disk.
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "native")]
            conn: None,
            highlights: HashMap::new(),
            notes: HashMap::new(),
            bookmarks: BTreeMap::new(),
            reading: BTreeMap::new(),
            plans: HashMap::new(),
            plan_reads: HashMap::new(),
            settings: HashMap::new(),
            generation: 1,
        }
    }

    #[cfg(feature = "native")]
    pub fn open(path: &std::path::Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        Self::with_connection(conn)
    }

    #[cfg(feature = "native")]
    pub fn open_in_memory() -> Result<Self> {
        Self::with_connection(Connection::open_in_memory()?)
    }

    #[cfg(feature = "native")]
    fn with_connection(conn: Connection) -> Result<Self> {
        conn.execute_batch(SCHEMA)?;
        migrate(&conn)?;
        let mut s = Self { conn: Some(conn), ..Self::new() };
        s.load()?;
        Ok(s)
    }

    /// Reads every table into memory.
    #[cfg(feature = "native")]
    fn load(&mut self) -> Result<()> {
        let Some(conn) = &self.conn else { return Ok(()) };
        let verse = |r: &rusqlite::Row<'_>| -> rusqlite::Result<_> {
            Ok(verse_from_row(&r.get::<_, String>(0)?, r.get(1)?, r.get(2)?))
        };
        let highlights = query(
            conn,
            "SELECT book, chapter, verse, char_start, char_end, quote, color, created, edited
             FROM highlights ORDER BY book, chapter, verse, char_start",
            |r| {
                let (start, end, quote, created) = (r.get(3)?, r.get(4)?, r.get(5)?, r.get(7)?);
                let edited = r.get::<_, Option<i64>>(8)?.unwrap_or(created);
                let h = HighlightColor::from_i64(r.get(6)?).map(|color| Highlight {
                    color,
                    start,
                    end,
                    quote,
                    created,
                    edited,
                });
                Ok((verse(r)?, h))
            },
        )?;
        self.highlights.clear();
        for (v, h) in highlights {
            if let (Some(v), Some(h)) = (v, h) {
                self.highlights.entry(v.key()).or_default().push(h);
            }
        }
        let notes = query(conn, "SELECT book, chapter, verse, body, created, updated FROM notes", |r| {
            Ok((verse(r)?, Note { body: r.get(3)?, created: r.get(4)?, updated: r.get(5)? }))
        })?;
        self.notes = notes.into_iter().filter_map(|(v, n)| Some((v?.key(), n))).collect();
        let bookmarks = query(conn, "SELECT book, chapter, verse, label, created FROM bookmarks", |r| {
            Ok((verse(r)?, Bookmark { label: r.get(3)?, created: r.get(4)? }))
        })?;
        self.bookmarks = bookmarks.into_iter().filter_map(|(v, b)| Some((v?.key(), b))).collect();
        let reading = query(
            conn,
            "SELECT started, book, chapter, translation, seconds, furthest, verses FROM reading",
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, i64>(6)?,
                ))
            },
        )?;
        self.reading = reading
            .into_iter()
            .filter_map(|(started, book, chapter, translation, seconds, furthest, verses)| {
                let session = ReadingSession {
                    started,
                    book: BookId::from_code(&book)?,
                    chapter: u8::try_from(chapter).ok()?,
                    translation,
                    seconds: u32::try_from(seconds).ok()?,
                    furthest: u8::try_from(furthest).ok()?,
                    verses: u8::try_from(verses).ok()?,
                };
                Some((started, session))
            })
            .collect();
        let plans = query(conn, "SELECT id, started, finished FROM plans", |r| {
            Ok(PlanProgress { id: r.get(0)?, started: r.get(1)?, finished: r.get(2)? })
        })?;
        self.plans = plans.into_iter().map(|p| (p.id.clone(), p)).collect();
        let reads = query(conn, "SELECT plan, day, passage FROM plan_reads", |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
        })?;
        self.plan_reads.clear();
        for (plan, day, passage) in reads {
            if let (Ok(day), Ok(passage)) = (u16::try_from(day), u8::try_from(passage)) {
                self.plan_reads.entry(plan).or_default().insert((day, passage));
            }
        }
        self.settings = query(conn, "SELECT key, value FROM settings", |r| Ok((r.get(0)?, r.get(1)?)))?
            .into_iter()
            .collect();
        self.generation += 1;
        Ok(())
    }

    /// Runs a statement on the SQLite file, when the store has one.
    #[cfg(feature = "native")]
    fn exec(&self, sql: &str, params: impl rusqlite::Params) -> Result<()> {
        if let Some(conn) = &self.conn {
            conn.execute(sql, params)?;
        }
        Ok(())
    }

    /// Runs `f` as one transaction: when it fails, what it wrote is undone on disk and in memory.
    fn transaction(&mut self, f: impl FnOnce(&mut Self) -> Result<()>) -> Result<()> {
        #[cfg(feature = "native")]
        self.exec("BEGIN", [])?;
        let result = f(self);
        #[cfg(feature = "native")]
        if result.is_ok() {
            self.exec("COMMIT", [])?;
        } else {
            self.exec("ROLLBACK", [])?;
            self.load()?;
        }
        result
    }

    /// Increments whenever study data changes (the reader uses it to invalidate layouts).
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// A verse's highlight colour: the newest highlight's, when parts of it have different ones.
    pub fn highlight(&self, v: VerseRef) -> Option<HighlightColor> {
        self.highlight_parts(v).iter().max_by_key(|h| h.created).map(|h| h.color)
    }

    /// A verse's highlights as stored (offsets in the text each was made in).
    pub fn highlight_parts(&self, v: VerseRef) -> &[Highlight] {
        self.highlights.get(&v.key()).map_or(&[], Vec::as_slice)
    }

    /// What a verse whose text is `text` shows: its coloured char ranges, in order.
    pub fn highlight_ranges(&self, v: VerseRef, text: &str) -> Vec<(std::ops::Range<usize>, HighlightColor)> {
        highlight::ranges(self.highlight_parts(v), text)
    }

    /// The colour that covers every letter of `span`, if one does. `text` gives the verse texts
    /// the span counts in; a verse it hasn't counts as its whole-verse highlight.
    pub fn span_color(
        &self,
        span: &TextSpan,
        text: impl Fn(VerseRef) -> Option<String>,
    ) -> Option<HighlightColor> {
        let mut color = None;
        for v in span.refs() {
            let c = match text(v) {
                Some(t) => highlight::color_of(self.highlight_parts(v), &t, span.range_of(v))?,
                None => self.highlight_parts(v).iter().find(|h| h.is_whole())?.color,
            };
            if color.is_some_and(|k| k != c) {
                return None;
            }
            color = Some(c);
        }
        color
    }

    pub fn note(&self, v: VerseRef) -> Option<&Note> {
        self.notes.get(&v.key())
    }

    pub fn bookmark(&self, v: VerseRef) -> Option<&Bookmark> {
        self.bookmarks.get(&v.key())
    }

    pub fn is_bookmarked(&self, v: VerseRef) -> bool {
        self.bookmarks.contains_key(&v.key())
    }

    /// Highlights a whole verse in `color`, replacing what it had, or (with `None`) clears all of
    /// it. A verse already highlighted keeps when it was first highlighted.
    pub fn set_highlight(&mut self, v: VerseRef, color: Option<HighlightColor>) -> Result<()> {
        let parts = match color {
            Some(c) => {
                let t = now();
                let created = self.highlight_parts(v).iter().map(|h| h.created).min().unwrap_or(t);
                vec![Highlight { edited: t, ..Highlight::whole(c, created) }]
            }
            None => Vec::new(),
        };
        self.transaction(|s| s.put_highlights(v, parts))?;
        self.generation += 1;
        Ok(())
    }

    /// Colours a stretch of text (`None` clears it): over the highlights there, which it replaces
    /// where they overlap. `text` gives the verse texts the span counts in; a verse it hasn't is
    /// coloured (or cleared) whole.
    pub fn highlight_span(
        &mut self,
        span: &TextSpan,
        color: Option<HighlightColor>,
        text: impl Fn(VerseRef) -> Option<String>,
    ) -> Result<()> {
        let t = now();
        self.transaction(|s| {
            for v in span.refs() {
                let parts = match text(v) {
                    Some(text) => highlight::paint(s.highlight_parts(v), &text, span.range_of(v), color, t),
                    None => color.map(|c| Highlight::whole(c, t)).into_iter().collect(),
                };
                s.put_highlights(v, parts)?;
            }
            Ok(())
        })?;
        self.generation += 1;
        Ok(())
    }

    /// Removes one of a verse's highlights: the one stored from char `start`.
    pub fn remove_highlight(&mut self, v: VerseRef, start: u32) -> Result<()> {
        let parts: Vec<Highlight> =
            self.highlight_parts(v).iter().filter(|h| h.start != start).cloned().collect();
        self.transaction(|s| s.put_highlights(v, parts))?;
        self.generation += 1;
        Ok(())
    }

    /// Replaces a verse's highlights.
    fn put_highlights(&mut self, v: VerseRef, mut parts: Vec<Highlight>) -> Result<()> {
        parts.sort_by_key(|h| h.start);
        parts.dedup_by_key(|h| h.start);
        #[cfg(feature = "native")]
        {
            let code = v.book.info().code;
            self.exec(
                "DELETE FROM highlights WHERE book = ?1 AND chapter = ?2 AND verse = ?3",
                params![code, v.chapter, v.verse],
            )?;
            for h in &parts {
                self.exec(
                    "INSERT INTO highlights(book, chapter, verse, char_start, char_end, quote, color, created, edited)
                     VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        code,
                        v.chapter,
                        v.verse,
                        h.start,
                        h.end,
                        h.quote,
                        h.color.to_i64(),
                        h.created,
                        (h.edited != h.created).then_some(h.edited)
                    ],
                )?;
            }
        }
        if parts.is_empty() {
            self.highlights.remove(&v.key());
        } else {
            self.highlights.insert(v.key(), parts);
        }
        Ok(())
    }

    /// Saves a note; an empty (whitespace-only) body deletes it.
    pub fn set_note(&mut self, v: VerseRef, body: &str) -> Result<()> {
        if body.trim().is_empty() {
            #[cfg(feature = "native")]
            self.exec(
                "DELETE FROM notes WHERE book = ?1 AND chapter = ?2 AND verse = ?3",
                params![v.book.info().code, v.chapter, v.verse],
            )?;
            self.notes.remove(&v.key());
        } else {
            let t = now();
            self.put_note(v, body, t, t)?;
        }
        self.generation += 1;
        Ok(())
    }

    /// A verse that already has a note keeps when it was first written.
    fn put_note(&mut self, v: VerseRef, body: &str, created: i64, updated: i64) -> Result<()> {
        #[cfg(feature = "native")]
        self.exec(
            "INSERT INTO notes(book, chapter, verse, body, created, updated) VALUES(?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(book, chapter, verse) DO UPDATE SET body = excluded.body, updated = excluded.updated",
            params![v.book.info().code, v.chapter, v.verse, body, created, updated],
        )?;
        let created = self.notes.get(&v.key()).map_or(created, |n| n.created);
        self.notes.insert(v.key(), Note { body: body.to_string(), created, updated });
        Ok(())
    }

    pub fn set_bookmark(&mut self, v: VerseRef, label: Option<&str>) -> Result<()> {
        match label {
            Some(label) => self.put_bookmark(v, label, now())?,
            None => {
                #[cfg(feature = "native")]
                self.exec(
                    "DELETE FROM bookmarks WHERE book = ?1 AND chapter = ?2 AND verse = ?3",
                    params![v.book.info().code, v.chapter, v.verse],
                )?;
                self.bookmarks.remove(&v.key());
            }
        }
        self.generation += 1;
        Ok(())
    }

    /// A verse already bookmarked keeps when it was first bookmarked.
    fn put_bookmark(&mut self, v: VerseRef, label: &str, created: i64) -> Result<()> {
        #[cfg(feature = "native")]
        self.exec(
            "INSERT INTO bookmarks(book, chapter, verse, label, created) VALUES(?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(book, chapter, verse) DO UPDATE SET label = excluded.label",
            params![v.book.info().code, v.chapter, v.verse, label, created],
        )?;
        let created = self.bookmarks.get(&v.key()).map_or(created, |b| b.created);
        self.bookmarks.insert(v.key(), Bookmark { label: label.to_string(), created });
        Ok(())
    }

    pub fn toggle_bookmark(&mut self, v: VerseRef) -> Result<bool> {
        let on = !self.is_bookmarked(v);
        self.set_bookmark(v, on.then_some(""))?;
        Ok(on)
    }

    /// Every highlight in canonical order (a verse's in the order they lie in it).
    pub fn highlights(&self) -> Vec<(VerseRef, &Highlight)> {
        let mut v: Vec<_> = self
            .highlights
            .iter()
            .filter_map(|(k, hs)| Some((VerseRef::from_key(*k)?, hs)))
            .flat_map(|(r, hs)| hs.iter().map(move |h| (r, h)))
            .collect();
        v.sort_by_key(|(r, h)| (*r, h.start));
        v
    }

    /// Every highlight, the most recently made first.
    pub fn highlights_dated(&self) -> Vec<(VerseRef, &Highlight)> {
        let mut v = self.highlights();
        v.sort_by_key(|(r, h)| (std::cmp::Reverse(h.created), *r, h.start));
        v
    }

    pub fn notes(&self) -> Vec<(VerseRef, &Note)> {
        let mut v: Vec<_> =
            self.notes.iter().filter_map(|(k, n)| Some((VerseRef::from_key(*k)?, n))).collect();
        v.sort_by_key(|(r, _)| *r);
        v
    }

    pub fn bookmarks(&self) -> Vec<(VerseRef, &Bookmark)> {
        self.bookmarks.iter().filter_map(|(k, b)| Some((VerseRef::from_key(*k)?, b))).collect()
    }

    /// How many highlights (a verse's parts counting one each), notes and bookmarks there are.
    pub fn counts(&self) -> (usize, usize, usize) {
        (self.highlights.values().map(Vec::len).sum(), self.notes.len(), self.bookmarks.len())
    }

    /// Saves a reading session, replacing an earlier save of the same session.
    pub fn record_reading(&mut self, s: &ReadingSession) -> Result<()> {
        #[cfg(feature = "native")]
        self.exec(
            "INSERT INTO reading(started, book, chapter, translation, seconds, furthest, verses)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(started) DO UPDATE SET seconds = excluded.seconds, furthest = excluded.furthest,
               verses = excluded.verses, translation = excluded.translation",
            params![s.started, s.book.info().code, s.chapter, s.translation, s.seconds, s.furthest, s.verses],
        )?;
        self.reading
            .entry(s.started)
            .and_modify(|e| {
                e.seconds = s.seconds;
                e.furthest = s.furthest;
                e.verses = s.verses;
                e.translation.clone_from(&s.translation);
            })
            .or_insert_with(|| s.clone());
        Ok(())
    }

    /// Reading sessions that started at or after `since` (Unix seconds), newest first.
    pub fn reading_since(&self, since: i64) -> Vec<ReadingSession> {
        self.reading.range(since..).rev().map(|(_, s)| s.clone()).collect()
    }

    /// Forgets reading history from `since` on (all of it for 0).
    pub fn clear_reading(&mut self, since: i64) -> Result<()> {
        #[cfg(feature = "native")]
        self.exec("DELETE FROM reading WHERE started >= ?1", [since])?;
        self.reading.retain(|&started, _| started < since);
        Ok(())
    }

    /// Plans started, newest first.
    pub fn plans(&self) -> Vec<PlanProgress> {
        let mut v: Vec<_> = self.plans.values().cloned().collect();
        v.sort_by(|a, b| b.started.cmp(&a.started).then_with(|| a.id.cmp(&b.id)));
        v
    }

    /// Starts (or restarts) a plan on the day containing `started`, forgetting earlier progress.
    pub fn start_plan(&mut self, id: &str, started: i64) -> Result<()> {
        #[cfg(feature = "native")]
        self.exec("DELETE FROM plan_reads WHERE plan = ?1", [id])?;
        self.plan_reads.remove(id);
        self.put_plan(id, started, None)?;
        self.generation += 1;
        Ok(())
    }

    fn put_plan(&mut self, id: &str, started: i64, finished: Option<i64>) -> Result<()> {
        #[cfg(feature = "native")]
        self.exec(
            "INSERT OR REPLACE INTO plans(id, started, finished) VALUES(?1, ?2, ?3)",
            params![id, started, finished],
        )?;
        self.plans.insert(id.to_string(), PlanProgress { id: id.to_string(), started, finished });
        Ok(())
    }

    pub fn finish_plan(&mut self, id: &str, at: Option<i64>) -> Result<()> {
        #[cfg(feature = "native")]
        self.exec("UPDATE plans SET finished = ?2 WHERE id = ?1", params![id, at])?;
        if let Some(p) = self.plans.get_mut(id) {
            p.finished = at;
        }
        self.generation += 1;
        Ok(())
    }

    pub fn stop_plan(&mut self, id: &str) -> Result<()> {
        #[cfg(feature = "native")]
        {
            self.exec("DELETE FROM plans WHERE id = ?1", [id])?;
            self.exec("DELETE FROM plan_reads WHERE plan = ?1", [id])?;
        }
        self.plans.remove(id);
        self.plan_reads.remove(id);
        self.generation += 1;
        Ok(())
    }

    /// The passages ticked off in a plan, as (day, passage index) pairs.
    pub fn plan_reads(&self, id: &str) -> HashSet<(u16, u8)> {
        self.plan_reads.get(id).cloned().unwrap_or_default()
    }

    pub fn set_plan_read(&mut self, id: &str, day: u16, passage: u8, read: bool) -> Result<()> {
        if read {
            #[cfg(feature = "native")]
            self.exec(
                "INSERT OR REPLACE INTO plan_reads(plan, day, passage, at) VALUES(?1, ?2, ?3, ?4)",
                params![id, day, passage, now()],
            )?;
            self.plan_reads.entry(id.to_string()).or_default().insert((day, passage));
        } else {
            #[cfg(feature = "native")]
            self.exec(
                "DELETE FROM plan_reads WHERE plan = ?1 AND day = ?2 AND passage = ?3",
                params![id, day, passage],
            )?;
            if let Some(reads) = self.plan_reads.get_mut(id) {
                reads.remove(&(day, passage));
            }
        }
        self.generation += 1;
        Ok(())
    }

    pub fn setting(&self, key: &str) -> Option<String> {
        self.settings.get(key).cloned()
    }

    pub fn set_setting(&mut self, key: &str, value: &str) -> Result<()> {
        #[cfg(feature = "native")]
        self.exec(
            "INSERT INTO settings(key, value) VALUES(?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        self.settings.insert(key.to_string(), value.to_string());
        Ok(())
    }

    /// JSON export of all study data. `text` supplies verse text for context (may return `None`).
    pub fn export_json(
        &self,
        translation: &str,
        text: impl Fn(VerseRef) -> Option<String>,
    ) -> Result<String> {
        self.export_json_with(translation, text, Include::STUDY)
    }

    /// The study data as JSON (re-importable), with what `include` asks for.
    pub fn export_json_with(
        &self,
        translation: &str,
        text: impl Fn(VerseRef) -> Option<String>,
        include: Include,
    ) -> Result<String> {
        let entry = |v: VerseRef| ExportVerse {
            reference: v.to_string(),
            book: v.book.info().code.to_string(),
            chapter: v.chapter,
            verse: v.verse,
            text: text(v),
        };
        let mut doc = ExportDoc {
            app: "bible-rs".into(),
            format: 1,
            exported_at: rfc3339(now()),
            translation: translation.to_string(),
            history: Vec::new(),
            plans: Vec::new(),
            highlights: self
                .highlights()
                .into_iter()
                .map(|(v, h)| ExportHighlight {
                    verse: entry(v),
                    color: h.color,
                    created: rfc3339(h.created),
                    start: h.start,
                    end: h.end,
                    quote: h.quote.clone(),
                    edited: (h.edited != h.created).then(|| rfc3339(h.edited)),
                })
                .collect(),
            notes: self
                .notes()
                .into_iter()
                .map(|(v, n)| ExportNote {
                    verse: entry(v),
                    note: n.body.clone(),
                    created: rfc3339(n.created),
                    updated: rfc3339(n.updated),
                })
                .collect(),
            bookmarks: self
                .bookmarks()
                .into_iter()
                .map(|(v, b)| ExportBookmark {
                    verse: entry(v),
                    label: b.label.clone(),
                    created: rfc3339(b.created),
                })
                .collect(),
        };
        if !include.highlights {
            doc.highlights.clear();
        }
        if !include.notes {
            doc.notes.clear();
        }
        if !include.bookmarks {
            doc.bookmarks.clear();
        }
        if include.history {
            doc.history = self
                .reading_since(0)
                .into_iter()
                .map(|s| ExportReading {
                    started: rfc3339(s.started),
                    book: s.book.info().code.to_string(),
                    chapter: s.chapter,
                    translation: s.translation,
                    seconds: s.seconds,
                    furthest: s.furthest,
                    verses: s.verses,
                })
                .collect();
            doc.plans = self
                .plans()
                .into_iter()
                .map(|p| ExportPlan {
                    read: self.plan_reads(&p.id).into_iter().collect(),
                    id: p.id,
                    started: rfc3339(p.started),
                    finished: p.finished.map(rfc3339),
                })
                .collect();
        }
        Ok(serde_json::to_string_pretty(&doc)?)
    }

    pub fn export_markdown(&self, translation: &str, text: impl Fn(VerseRef) -> Option<String>) -> String {
        self.export_markdown_with(translation, text, Include::STUDY)
    }

    /// The study data as readable Markdown, with what `include` asks for.
    pub fn export_markdown_with(
        &self,
        translation: &str,
        text: impl Fn(VerseRef) -> Option<String>,
        include: Include,
    ) -> String {
        use std::fmt::Write;
        let mut md = String::new();
        let _ = writeln!(md, "# Bible study notes\n");
        let _ =
            writeln!(md, "Exported from Bible on {} · verse text: {translation}\n", &rfc3339(now())[..10]);
        let bookmarks = if include.bookmarks { self.bookmarks() } else { Vec::new() };
        if !bookmarks.is_empty() {
            let _ = writeln!(md, "## Bookmarks\n");
            for (v, b) in bookmarks {
                if b.label.is_empty() {
                    let _ = writeln!(md, "- **{v}**");
                } else {
                    let _ = writeln!(md, "- **{v}** — {}", b.label);
                }
            }
            md.push('\n');
        }
        let highlights = if include.highlights { self.highlights() } else { Vec::new() };
        if !highlights.is_empty() {
            let _ = writeln!(md, "## Highlights\n");
            let mut book = None;
            for (v, h) in highlights {
                if book != Some(v.book) {
                    book = Some(v.book);
                    let _ = writeln!(md, "### {}\n", v.book.name());
                }
                // Part of a verse: the words it covers.
                let t = match text(v) {
                    Some(t) if !h.is_whole() => h.excerpt(&t),
                    Some(t) => t,
                    None => h.quote.clone().unwrap_or_default(),
                };
                let _ = writeln!(md, "- **{v}** ({}) {t}", h.color.name());
            }
            md.push('\n');
        }
        let notes = if include.notes { self.notes() } else { Vec::new() };
        if !notes.is_empty() {
            let _ = writeln!(md, "## Notes\n");
            for (v, n) in notes {
                let _ = writeln!(md, "### {v}\n");
                if let Some(t) = text(v) {
                    let _ = writeln!(md, "> {t}\n");
                }
                let _ = writeln!(md, "{}\n", n.body.trim_end());
            }
        }
        let history = if include.history { self.reading_since(0) } else { Vec::new() };
        if !history.is_empty() {
            let _ = writeln!(md, "## Reading history\n");
            for s in history {
                let minutes = (s.seconds + 30) / 60;
                let done = if s.finished() {
                    "finished".to_string()
                } else {
                    format!("{:.0} %", s.progress() * 100.0)
                };
                let _ = writeln!(
                    md,
                    "- {} — {} {} ({}), {minutes} min, {done}",
                    &rfc3339(s.started)[..10],
                    s.book.name(),
                    s.chapter,
                    s.translation
                );
            }
            md.push('\n');
        }
        md
    }

    /// Merges a JSON export back in (existing entries for the same verse are overwritten).
    pub fn import_json(&mut self, json: &str) -> Result<(usize, usize, usize)> {
        let doc: ExportDoc = serde_json::from_str(json).context("not a bible-rs export")?;
        if doc.app != "bible-rs" {
            bail!("not a bible-rs export");
        }
        let verse = |e: &ExportVerse| -> Result<VerseRef> {
            let book = BookId::from_code(&e.book).with_context(|| format!("unknown book {}", e.book))?;
            Ok(VerseRef::new(book, e.chapter, e.verse))
        };
        // Every verse is checked before anything is written.
        let highlights =
            doc.highlights.iter().map(|h| Ok((verse(&h.verse)?, h))).collect::<Result<Vec<_>>>()?;
        let notes = doc.notes.iter().map(|n| Ok((verse(&n.verse)?, n))).collect::<Result<Vec<_>>>()?;
        let bookmarks =
            doc.bookmarks.iter().map(|b| Ok((verse(&b.verse)?, b))).collect::<Result<Vec<_>>>()?;
        let date = |s: &str| parse_rfc3339(s).unwrap_or_else(now);
        // A verse's highlights in the export replace the ones it has.
        let mut by_verse: BTreeMap<VerseRef, Vec<Highlight>> = BTreeMap::new();
        for (v, h) in &highlights {
            let created = date(&h.created);
            by_verse.entry(*v).or_default().push(Highlight {
                color: h.color,
                start: h.start,
                end: h.end,
                quote: h.quote.clone().filter(|_| h.start > 0 || h.end.is_some()),
                created,
                edited: h.edited.as_deref().and_then(parse_rfc3339).unwrap_or(created),
            });
        }
        self.transaction(|s| {
            for (v, parts) in by_verse {
                s.put_highlights(v, parts)?;
            }
            for (v, n) in &notes {
                s.put_note(*v, &n.note, date(&n.created), date(&n.updated))?;
            }
            for (v, b) in &bookmarks {
                s.put_bookmark(*v, &b.label, date(&b.created))?;
            }
            for r in &doc.history {
                let Some(book) = BookId::from_code(&r.book) else { continue };
                s.record_reading(&ReadingSession {
                    started: date(&r.started),
                    book,
                    chapter: r.chapter,
                    translation: r.translation.clone(),
                    seconds: r.seconds,
                    furthest: r.furthest,
                    verses: r.verses,
                })?;
            }
            for p in &doc.plans {
                s.put_plan(&p.id, date(&p.started), p.finished.as_deref().and_then(parse_rfc3339))?;
                for &(day, passage) in &p.read {
                    s.set_plan_read(&p.id, day, passage, true)?;
                }
            }
            Ok(())
        })?;
        self.generation += 1;
        Ok((doc.highlights.len(), doc.notes.len(), doc.bookmarks.len()))
    }

    /// Erases every highlight, note, bookmark, plan and the reading history.
    pub fn erase_all(&mut self) -> Result<()> {
        #[cfg(feature = "native")]
        if let Some(conn) = &self.conn {
            conn.execute_batch(
                "DELETE FROM highlights; DELETE FROM notes; DELETE FROM bookmarks;
                 DELETE FROM plans; DELETE FROM plan_reads; DELETE FROM reading;",
            )?;
        }
        self.highlights.clear();
        self.notes.clear();
        self.bookmarks.clear();
        self.plans.clear();
        self.plan_reads.clear();
        self.reading.clear();
        self.generation += 1;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct ExportDoc {
    app: String,
    format: u32,
    exported_at: String,
    translation: String,
    #[serde(default)]
    highlights: Vec<ExportHighlight>,
    #[serde(default)]
    notes: Vec<ExportNote>,
    #[serde(default)]
    bookmarks: Vec<ExportBookmark>,
    /// Reading sessions and plans, when the export includes history.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    history: Vec<ExportReading>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    plans: Vec<ExportPlan>,
}

#[derive(Serialize, Deserialize)]
struct ExportReading {
    started: String,
    book: String,
    chapter: u8,
    translation: String,
    seconds: u32,
    furthest: u8,
    verses: u8,
}

#[derive(Serialize, Deserialize)]
struct ExportPlan {
    id: String,
    started: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    finished: Option<String>,
    /// Passages ticked, as (day, index).
    read: Vec<(u16, u8)>,
}

/// What an export carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Include {
    pub highlights: bool,
    pub notes: bool,
    pub bookmarks: bool,
    /// Reading history and plans.
    pub history: bool,
}

impl Include {
    /// Highlights, notes and bookmarks.
    pub const STUDY: Include = Include { highlights: true, notes: true, bookmarks: true, history: false };
    pub const EVERYTHING: Include = Include { highlights: true, notes: true, bookmarks: true, history: true };

    pub const fn default_none() -> Include {
        Include { highlights: false, notes: false, bookmarks: false, history: false }
    }
}

#[derive(Serialize, Deserialize)]
struct ExportVerse {
    reference: String,
    book: String,
    chapter: u8,
    verse: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    text: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ExportHighlight {
    #[serde(flatten)]
    verse: ExportVerse,
    color: HighlightColor,
    created: String,
    /// Part of a verse: from this char of its text (exports before 2026-10 have whole verses).
    #[serde(default, skip_serializing_if = "is_zero")]
    start: u32,
    /// To before this char; left out, to the verse's end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    end: Option<u32>,
    /// The words covered, to place them in another translation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    quote: Option<String>,
    /// When it was last coloured, if after it was made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    edited: Option<String>,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

#[derive(Serialize, Deserialize)]
struct ExportNote {
    #[serde(flatten)]
    verse: ExportVerse,
    note: String,
    created: String,
    updated: String,
}

#[derive(Serialize, Deserialize)]
struct ExportBookmark {
    #[serde(flatten)]
    verse: ExportVerse,
    label: String,
    created: String,
}

/// Unix seconds → `YYYY-MM-DDTHH:MM:SSZ` (proleptic Gregorian, UTC).
pub fn rfc3339(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

fn parse_rfc3339(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 19 {
        return None;
    }
    let num = |r: std::ops::Range<usize>| s.get(r)?.parse::<i64>().ok();
    let days = days_from_civil(num(0..4)?, num(5..7)? as u32, num(8..10)? as u32);
    Some(days * 86_400 + num(11..13)? * 3600 + num(14..16)? * 60 + num(17..19)?)
}

// Howard Hinnant's date algorithms.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod plan_tests {
    use super::*;

    #[test]
    fn plans_start_tick_and_stop() {
        let mut s = StudyStore::open_in_memory().unwrap();
        s.start_plan("john-21", 1_000).unwrap();
        s.set_plan_read("john-21", 1, 0, true).unwrap();
        s.set_plan_read("john-21", 2, 0, true).unwrap();
        s.set_plan_read("john-21", 2, 0, false).unwrap();
        assert_eq!(s.plans()[0].id, "john-21");
        assert_eq!(s.plan_reads("john-21").into_iter().collect::<Vec<_>>(), vec![(1, 0)]);
        // Restarting forgets progress.
        s.start_plan("john-21", 2_000).unwrap();
        assert!(s.plan_reads("john-21").is_empty());
        s.finish_plan("john-21", Some(3_000)).unwrap();
        assert_eq!(s.plans()[0].finished, Some(3_000));
        s.stop_plan("john-21").unwrap();
        assert!(s.plans().is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jn316() -> VerseRef {
        VerseRef::new(BookId(42), 3, 16)
    }

    #[test]
    fn reading_sessions() {
        let mut s = StudyStore::open_in_memory().unwrap();
        let mut session = ReadingSession {
            started: 1_000,
            book: BookId(42),
            chapter: 3,
            translation: "kjv".into(),
            seconds: 30,
            furthest: 10,
            verses: 36,
        };
        s.record_reading(&session).unwrap();
        session.seconds = 400;
        session.furthest = 36;
        s.record_reading(&session).unwrap();
        s.record_reading(&ReadingSession { started: 2_000, chapter: 4, furthest: 3, ..session.clone() })
            .unwrap();
        let all = s.reading_since(0);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].chapter, 4, "newest first");
        assert_eq!(all[1].seconds, 400, "a session is saved in place");
        assert!(all[1].finished() && !all[0].finished());
        assert_eq!(s.reading_since(1_500).len(), 1);
        s.clear_reading(1_500).unwrap();
        assert_eq!(s.reading_since(0).len(), 1);
    }

    #[test]
    fn crud_and_persistence() {
        let dir = std::env::temp_dir().join(format!("bible-rs-study-{}", std::process::id()));
        let path = dir.join("study.sqlite");
        let _ = std::fs::remove_dir_all(&dir);
        {
            let mut s = StudyStore::open(&path).unwrap();
            let g = s.generation();
            s.set_highlight(jn316(), Some(HighlightColor::Yellow)).unwrap();
            s.set_highlight(jn316(), Some(HighlightColor::Green)).unwrap();
            s.set_note(jn316(), "God's love").unwrap();
            assert!(s.toggle_bookmark(VerseRef::new(BookId(0), 1, 1)).unwrap());
            s.set_setting("theme", "dark").unwrap();
            s.record_reading(&ReadingSession {
                started: 1_000,
                book: BookId(42),
                chapter: 3,
                translation: "kjv".into(),
                seconds: 90,
                furthest: 20,
                verses: 36,
            })
            .unwrap();
            s.start_plan("john-21", 900).unwrap();
            s.set_plan_read("john-21", 1, 0, true).unwrap();
            assert!(s.generation() > g);
        }
        let mut s = StudyStore::open(&path).unwrap();
        assert_eq!(s.highlight(jn316()), Some(HighlightColor::Green));
        assert_eq!(s.note(jn316()).unwrap().body, "God's love");
        assert!(s.is_bookmarked(VerseRef::new(BookId(0), 1, 1)));
        assert_eq!(s.setting("theme").as_deref(), Some("dark"));
        assert_eq!(s.counts(), (1, 1, 1));
        assert_eq!(s.reading_since(0)[0].furthest, 20);
        assert_eq!((s.plans()[0].started, s.plan_reads("john-21").len()), (900, 1));
        s.set_note(jn316(), "  ").unwrap();
        s.set_highlight(jn316(), None).unwrap();
        assert!(!s.toggle_bookmark(VerseRef::new(BookId(0), 1, 1)).unwrap());
        assert_eq!(s.counts(), (0, 0, 0));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn json_round_trip_and_markdown() {
        let mut a = StudyStore::open_in_memory().unwrap();
        a.set_highlight(jn316(), Some(HighlightColor::Pink)).unwrap();
        a.set_note(jn316(), "line one\nline two").unwrap();
        a.set_bookmark(VerseRef::new(BookId(18), 23, 1), Some("Shepherd")).unwrap();
        let text = |v: VerseRef| Some(format!("text of {v}"));
        let json = a.export_json("KJV", text).unwrap();
        assert!(json.contains("\"reference\": \"John 3:16\""));
        assert!(json.contains("\"color\": \"pink\""));

        let mut b = StudyStore::open_in_memory().unwrap();
        assert_eq!(b.import_json(&json).unwrap(), (1, 1, 1));
        assert_eq!(b.highlight(jn316()), Some(HighlightColor::Pink));
        assert_eq!(b.note(jn316()).unwrap().body, "line one\nline two");
        assert_eq!(b.bookmark(VerseRef::new(BookId(18), 23, 1)).unwrap().label, "Shepherd");
        assert_eq!(b.note(jn316()).unwrap().created, a.note(jn316()).unwrap().created);

        let md = b.export_markdown("KJV", text);
        assert!(md.contains("## Bookmarks"));
        assert!(md.contains("- **Psalm 23:1** — Shepherd"));
        assert!(md.contains("- **John 3:16** (pink) text of John 3:16"));
        assert!(md.contains("### John 3:16\n\n> text of John 3:16\n\nline one\nline two"));
        assert!(b.import_json("{}").is_err());
        // History and plans travel when asked for, and erasing clears everything.
        a.record_reading(&ReadingSession {
            started: 1_000,
            book: BookId(42),
            chapter: 3,
            translation: "kjv".into(),
            seconds: 300,
            furthest: 36,
            verses: 36,
        })
        .unwrap();
        a.start_plan("john-21", 900).unwrap();
        a.set_plan_read("john-21", 1, 0, true).unwrap();
        let all = a.export_json_with("KJV", text, Include::EVERYTHING).unwrap();
        let only_notes =
            a.export_json_with("KJV", text, Include { notes: true, ..Include::default_none() }).unwrap();
        assert!(!only_notes.contains("\"highlights\": [\n"), "{only_notes}");
        let mut c = StudyStore::open_in_memory().unwrap();
        c.import_json(&all).unwrap();
        assert_eq!(c.reading_since(0).len(), 1);
        assert_eq!(c.plan_reads("john-21").len(), 1);
        c.erase_all().unwrap();
        assert_eq!((c.counts(), c.reading_since(0).len(), c.plans().len()), ((0, 0, 0), 0, 0));
    }

    /// The web app's store: no SQLite, saved and restored through its JSON export.
    #[test]
    fn without_a_database() {
        let mut a = StudyStore::new();
        a.set_highlight(jn316(), Some(HighlightColor::Blue)).unwrap();
        a.set_note(jn316(), "first").unwrap();
        a.set_bookmark(VerseRef::new(BookId(18), 23, 1), Some("Shepherd")).unwrap();
        a.set_setting("web", "{}").unwrap();
        a.record_reading(&ReadingSession {
            started: 1_000,
            book: BookId(42),
            chapter: 3,
            translation: "kjv".into(),
            seconds: 300,
            furthest: 36,
            verses: 36,
        })
        .unwrap();
        a.start_plan("john-21", 900).unwrap();
        a.set_plan_read("john-21", 1, 0, true).unwrap();
        a.finish_plan("john-21", Some(2_000)).unwrap();
        assert_eq!(a.setting("web").as_deref(), Some("{}"));

        let saved = a.export_json_with("KJV", |_| None, Include::EVERYTHING).unwrap();
        let mut b = StudyStore::new();
        b.import_json(&saved).unwrap();
        assert_eq!(b.highlight(jn316()), Some(HighlightColor::Blue));
        assert_eq!(b.note(jn316()), a.note(jn316()));
        assert_eq!(b.bookmarks()[0].1, a.bookmarks()[0].1);
        assert_eq!(b.reading_since(0), a.reading_since(0));
        assert_eq!(b.plans(), a.plans());
        assert_eq!(b.plan_reads("john-21"), a.plan_reads("john-21"));

        b.clear_reading(0).unwrap();
        b.stop_plan("john-21").unwrap();
        assert!(b.reading_since(0).is_empty() && b.plans().is_empty() && b.plan_reads("john-21").is_empty());
        b.erase_all().unwrap();
        assert_eq!(b.counts(), (0, 0, 0));
    }

    fn text(v: VerseRef) -> Option<String> {
        Some(
            match v.verse {
                16 => "For God so loved the world, that he gave his only begotten Son.",
                17 => "For God sent not his Son into the world to condemn the world.",
                18 => "He that believeth on him is not condemned.",
                _ => return None,
            }
            .to_string(),
        )
    }

    fn jn3(n: u8) -> VerseRef {
        VerseRef::new(BookId(42), 3, n)
    }

    fn shown(s: &StudyStore, v: VerseRef) -> Vec<(String, HighlightColor)> {
        let t = text(v).unwrap();
        s.highlight_ranges(v, &t)
            .into_iter()
            .map(|(r, c)| (t.chars().skip(r.start).take(r.len()).collect(), c))
            .collect()
    }

    /// A file written before highlights could cover part of a verse: one row per verse.
    #[test]
    fn migrates_whole_verse_highlights() {
        let dir = std::env::temp_dir().join(format!("bible-rs-migrate-{}", std::process::id()));
        let path = dir.join("study.sqlite");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE highlights(
                     book TEXT NOT NULL, chapter INTEGER NOT NULL, verse INTEGER NOT NULL,
                     color INTEGER NOT NULL, created INTEGER NOT NULL,
                     PRIMARY KEY(book, chapter, verse));
                 INSERT INTO highlights VALUES('JHN', 3, 16, 0, 1000), ('JHN', 3, 17, 3, 2000),
                     ('PSA', 23, 1, 5, 3000), ('XXX', 1, 1, 0, 4000);
                 CREATE TABLE notes(
                     book TEXT NOT NULL, chapter INTEGER NOT NULL, verse INTEGER NOT NULL,
                     body TEXT NOT NULL, created INTEGER NOT NULL, updated INTEGER NOT NULL,
                     PRIMARY KEY(book, chapter, verse));
                 INSERT INTO notes VALUES('JHN', 3, 16, 'kept', 1, 1);",
            )
            .unwrap();
        }
        let mut s = StudyStore::open(&path).unwrap();
        let all: Vec<(VerseRef, Highlight)> =
            s.highlights().into_iter().map(|(v, h)| (v, h.clone())).collect();
        assert_eq!(
            all,
            vec![
                (VerseRef::new(BookId::PSALMS, 23, 1), Highlight::whole(HighlightColor::Orange, 3000)),
                (jn3(16), Highlight::whole(HighlightColor::Yellow, 1000)),
                (jn3(17), Highlight::whole(HighlightColor::Pink, 2000)),
            ]
        );
        assert_eq!(s.note(jn3(16)).unwrap().body, "kept");
        // The unknown book's row is carried over too, though nothing shows it.
        let conn = Connection::open(&path).unwrap();
        let rows: i64 = conn.query_row("SELECT COUNT(*) FROM highlights", [], |r| r.get(0)).unwrap();
        assert_eq!(rows, 4);
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
        assert_eq!(version, 1);
        drop(conn);
        // Parts of verses save in the new table, and opening again changes nothing.
        s.highlight_span(
            &TextSpan { first: jn3(18), start: 0, last: jn3(18), end: 14 },
            Some(HighlightColor::Blue),
            text,
        )
        .unwrap();
        drop(s);
        let s = StudyStore::open(&path).unwrap();
        assert_eq!(s.counts().0, 4);
        assert_eq!(shown(&s, jn3(18)), vec![("He that believ".into(), HighlightColor::Blue)]);
        assert_eq!(s.highlight(jn3(17)), Some(HighlightColor::Pink));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn spans_across_verses_persist() {
        let dir = std::env::temp_dir().join(format!("bible-rs-spans-{}", std::process::id()));
        let path = dir.join("study.sqlite");
        let _ = std::fs::remove_dir_all(&dir);
        {
            let mut s = StudyStore::open(&path).unwrap();
            // From "the world," in 16 to "his Son" in 17.
            let span = TextSpan { first: jn3(16), start: 17, last: jn3(17), end: 24 };
            s.highlight_span(&span, Some(HighlightColor::Yellow), text).unwrap();
            assert_eq!(
                shown(&s, jn3(16)),
                vec![("the world, that he gave his only begotten Son.".into(), HighlightColor::Yellow)]
            );
            assert_eq!(shown(&s, jn3(17)), vec![("For God sent not his Son".into(), HighlightColor::Yellow)]);
            assert_eq!(s.span_color(&span, text), Some(HighlightColor::Yellow));
            assert_eq!(s.span_color(&TextSpan::verses(jn3(16), jn3(17)), text), None, "not all of them");
            // Clearing the middle leaves both ends.
            s.highlight_span(&TextSpan { first: jn3(16), start: 28, last: jn3(17), end: 3 }, None, text)
                .unwrap();
            assert_eq!(shown(&s, jn3(16)), vec![("the world,".into(), HighlightColor::Yellow)]);
            assert_eq!(shown(&s, jn3(17)), vec![("God sent not his Son".into(), HighlightColor::Yellow)]);
        }
        let mut s = StudyStore::open(&path).unwrap();
        assert_eq!(shown(&s, jn3(16)), vec![("the world,".into(), HighlightColor::Yellow)]);
        assert_eq!(s.counts().0, 2);
        // Whole verses over parts replace them; removing one highlight leaves the verse's others.
        let made = s.highlight_parts(jn3(16))[0].created;
        s.set_highlight(jn3(16), Some(HighlightColor::Green)).unwrap();
        let parts = s.highlight_parts(jn3(16));
        assert!(parts.len() == 1 && parts[0].is_whole() && parts[0].color == HighlightColor::Green);
        assert_eq!(parts[0].created, made, "a verse already highlighted keeps when it was first highlighted");
        s.highlight_span(
            &TextSpan { first: jn3(18), start: 0, last: jn3(18), end: 7 },
            Some(HighlightColor::Blue),
            text,
        )
        .unwrap();
        s.highlight_span(
            &TextSpan { first: jn3(18), start: 28, last: jn3(18), end: 31 },
            Some(HighlightColor::Pink),
            text,
        )
        .unwrap();
        assert_eq!(s.highlight_parts(jn3(18)).len(), 2);
        s.remove_highlight(jn3(18), 28).unwrap();
        assert_eq!(shown(&s, jn3(18)), vec![("He that".into(), HighlightColor::Blue)]);
        // A verse the translation hasn't is coloured whole.
        s.highlight_span(
            &TextSpan { first: jn3(19), start: 3, last: jn3(19), end: 9 },
            Some(HighlightColor::Blue),
            text,
        )
        .unwrap();
        assert!(s.highlight_parts(jn3(19))[0].is_whole());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parts_of_verses_round_trip_through_json() {
        let mut a = StudyStore::new();
        a.highlight_span(
            &TextSpan { first: jn3(16), start: 4, last: jn3(16), end: 26 },
            Some(HighlightColor::Pink),
            text,
        )
        .unwrap();
        a.highlight_span(
            &TextSpan { first: jn3(17), start: 21, last: jn3(17), end: usize::MAX },
            Some(HighlightColor::Blue),
            text,
        )
        .unwrap();
        a.set_highlight(jn3(18), Some(HighlightColor::Green)).unwrap();
        let json = a.export_json("KJV", text).unwrap();
        assert!(json.contains("\"quote\": \"God so loved the world\""), "{json}");
        assert!(json.contains("\"start\": 4"));
        let md = a.export_markdown("KJV", text);
        assert!(md.contains("- **John 3:16** (pink) …God so loved the world…"), "{md}");
        assert!(md.contains("- **John 3:17** (blue) …Son into the world to condemn the world."), "{md}");
        assert!(md.contains("- **John 3:18** (green) He that believeth on him is not condemned."), "{md}");

        // The web app's snapshot path: no database.
        let mut b = StudyStore::new();
        assert_eq!(b.import_json(&json).unwrap().0, 3);
        for v in [jn3(16), jn3(17), jn3(18)] {
            assert_eq!(b.highlight_parts(v), a.highlight_parts(v), "{v}");
        }
        // And into a file.
        let mut c = StudyStore::open_in_memory().unwrap();
        c.import_json(&json).unwrap();
        assert_eq!(shown(&c, jn3(16)), vec![("God so loved the world".into(), HighlightColor::Pink)]);
    }

    /// When a highlight was last coloured travels with it, through JSON and the file.
    #[test]
    fn colouring_times_persist() {
        let json = r#"{"app":"bible-rs","format":1,"exported_at":"2026-10-16T12:00:00Z","translation":"KJV",
            "highlights":[{"reference":"John 3:16","book":"JHN","chapter":3,"verse":16,"color":"blue",
              "created":"2026-10-14T12:00:00Z","edited":"2026-10-15T12:00:00Z"}]}"#;
        let dir = std::env::temp_dir().join(format!("bible-rs-edited-{}", std::process::id()));
        let path = dir.join("study.sqlite");
        let _ = std::fs::remove_dir_all(&dir);
        let mut a = StudyStore::open(&path).unwrap();
        a.import_json(json).unwrap();
        let h = a.highlight_parts(jn3(16))[0].clone();
        assert_eq!((h.created, h.edited), (1_791_979_200, 1_792_065_600));
        let out = a.export_json("KJV", |_| None).unwrap();
        assert!(out.contains("\"edited\": \"2026-10-15T12:00:00Z\""), "{out}");
        drop(a);
        assert_eq!(StudyStore::open(&path).unwrap().highlight_parts(jn3(16)), std::slice::from_ref(&h));
        let mut b = StudyStore::new();
        b.import_json(&out).unwrap();
        assert_eq!(b.highlight_parts(jn3(16)), &[h]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Exports made before parts of verses could be highlighted.
    #[test]
    fn old_exports_import_as_whole_verses() {
        let json = r#"{"app":"bible-rs","format":1,"exported_at":"2026-10-14T12:00:00Z","translation":"KJV",
            "highlights":[
              {"reference":"John 3:16","book":"JHN","chapter":3,"verse":16,"color":"yellow","created":"2026-10-14T12:00:00Z"},
              {"reference":"John 3:17","book":"JHN","chapter":3,"verse":17,"text":"For God sent","color":"green","created":"2026-10-14T12:00:00Z"}
            ]}"#;
        let mut s = StudyStore::new();
        assert_eq!(s.import_json(json).unwrap(), (2, 0, 0));
        assert_eq!(s.highlight_parts(jn3(16)), &[Highlight::whole(HighlightColor::Yellow, 1_791_979_200)]);
        assert!(s.highlight_parts(jn3(17))[0].is_whole());
        // Importing replaces a verse's highlights with the export's.
        s.highlight_span(
            &TextSpan { first: jn3(16), start: 4, last: jn3(16), end: 7 },
            Some(HighlightColor::Blue),
            text,
        )
        .unwrap();
        s.import_json(json).unwrap();
        assert_eq!(shown(&s, jn3(16)), vec![(text(jn3(16)).unwrap(), HighlightColor::Yellow)]);
    }

    #[test]
    fn dates() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(1_791_158_400), "2026-10-05T00:00:00Z");
        for t in [0, 951_782_400, 1_791_158_399, 4_102_444_800] {
            assert_eq!(parse_rfc3339(&rfc3339(t)), Some(t));
        }
    }
}
