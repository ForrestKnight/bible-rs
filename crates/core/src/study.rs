//! Local study data: highlights, notes, bookmarks, reading history, plans and settings.
//!
//! Everything is keyed by canonical verse reference (not by translation), so a highlight made
//! while reading the KJV shows up in the WEB too. The store lives in memory — hundreds of rows,
//! not millions — so the reader can look it up per verse every frame. The native apps back it with
//! one SQLite file and write straight through (WAL mode, a few hundred microseconds each); the web
//! app has no SQLite and saves the store's JSON export instead.

use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::{Context, Result, bail};
#[cfg(feature = "native")]
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::books::{BookId, ChapterIdx, VerseRef};

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
    highlights: HashMap<u32, (HighlightColor, i64)>,
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
    color INTEGER NOT NULL, created INTEGER NOT NULL,
    PRIMARY KEY(book, chapter, verse));
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
        let highlights = query(conn, "SELECT book, chapter, verse, color, created FROM highlights", |r| {
            Ok((verse(r)?, HighlightColor::from_i64(r.get(3)?), r.get(4)?))
        })?;
        self.highlights = highlights.into_iter().filter_map(|(v, c, t)| Some((v?.key(), (c?, t)))).collect();
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

    pub fn highlight(&self, v: VerseRef) -> Option<HighlightColor> {
        self.highlights.get(&v.key()).map(|(c, _)| *c)
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

    pub fn set_highlight(&mut self, v: VerseRef, color: Option<HighlightColor>) -> Result<()> {
        match color {
            Some(c) => self.put_highlight(v, c, now())?,
            None => {
                #[cfg(feature = "native")]
                self.exec(
                    "DELETE FROM highlights WHERE book = ?1 AND chapter = ?2 AND verse = ?3",
                    params![v.book.info().code, v.chapter, v.verse],
                )?;
                self.highlights.remove(&v.key());
            }
        }
        self.generation += 1;
        Ok(())
    }

    /// A verse already highlighted keeps when it was first highlighted.
    fn put_highlight(&mut self, v: VerseRef, color: HighlightColor, created: i64) -> Result<()> {
        #[cfg(feature = "native")]
        self.exec(
            "INSERT INTO highlights(book, chapter, verse, color, created) VALUES(?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(book, chapter, verse) DO UPDATE SET color = excluded.color",
            params![v.book.info().code, v.chapter, v.verse, color.to_i64(), created],
        )?;
        let created = self.highlights.get(&v.key()).map_or(created, |(_, t)| *t);
        self.highlights.insert(v.key(), (color, created));
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

    pub fn highlights(&self) -> Vec<(VerseRef, HighlightColor)> {
        let mut v: Vec<_> =
            self.highlights.iter().filter_map(|(k, (c, _))| Some((VerseRef::from_key(*k)?, *c))).collect();
        v.sort_by_key(|(r, _)| *r);
        v
    }

    /// Highlights, the most recently made first.
    pub fn highlights_newest_first(&self) -> Vec<(VerseRef, HighlightColor)> {
        let mut v: Vec<_> = self
            .highlights
            .iter()
            .filter_map(|(k, (c, t))| Some((VerseRef::from_key(*k)?, *c, *t)))
            .collect();
        v.sort_by_key(|(r, _, t)| (std::cmp::Reverse(*t), *r));
        v.into_iter().map(|(r, c, _)| (r, c)).collect()
    }

    /// Highlights with when they were made, the most recent first.
    pub fn highlights_dated(&self) -> Vec<(VerseRef, HighlightColor, i64)> {
        let mut v: Vec<_> = self
            .highlights
            .iter()
            .filter_map(|(k, (c, t))| Some((VerseRef::from_key(*k)?, *c, *t)))
            .collect();
        v.sort_by_key(|(r, _, t)| (std::cmp::Reverse(*t), *r));
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

    pub fn counts(&self) -> (usize, usize, usize) {
        (self.highlights.len(), self.notes.len(), self.bookmarks.len())
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
                .map(|(v, c)| ExportHighlight {
                    verse: entry(v),
                    color: c,
                    created: rfc3339(self.highlights[&v.key()].1),
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
            for (v, c) in highlights {
                if book != Some(v.book) {
                    book = Some(v.book);
                    let _ = writeln!(md, "### {}\n", v.book.name());
                }
                let t = text(v).unwrap_or_default();
                let _ = writeln!(md, "- **{v}** ({}) {t}", c.name());
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
        self.transaction(|s| {
            for (v, h) in &highlights {
                s.put_highlight(*v, h.color, date(&h.created))?;
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

    #[test]
    fn dates() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(1_791_158_400), "2026-10-05T00:00:00Z");
        for t in [0, 951_782_400, 1_791_158_399, 4_102_444_800] {
            assert_eq!(parse_rfc3339(&rfc3339(t)), Some(t));
        }
    }
}
