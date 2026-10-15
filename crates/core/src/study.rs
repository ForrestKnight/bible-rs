//! Local study data: highlights, notes, bookmarks and settings, in one SQLite file.
//!
//! Everything is keyed by canonical verse reference (not by translation), so a highlight made
//! while reading the KJV shows up in the WEB too. The tables are mirrored in memory — they hold
//! hundreds of rows, not millions — so the reader can look them up per verse every frame; writes go
//! straight through to SQLite (WAL mode, a few hundred microseconds each).

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
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

    fn from_i64(v: i64) -> Option<Self> {
        Self::ALL.get(usize::try_from(v).ok()?).copied()
    }

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
    conn: Connection,
    highlights: HashMap<u32, (HighlightColor, i64)>,
    notes: HashMap<u32, Note>,
    bookmarks: BTreeMap<u32, Bookmark>,
    generation: u64,
}

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

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn verse_from_row(book: &str, chapter: i64, verse: i64) -> Option<VerseRef> {
    let book = BookId::from_code(book)?;
    Some(VerseRef::new(book, u8::try_from(chapter).ok()?, u8::try_from(verse).ok()?))
}

impl StudyStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.execute_batch(SCHEMA)?;
        let mut s = Self {
            conn,
            highlights: HashMap::new(),
            notes: HashMap::new(),
            bookmarks: BTreeMap::new(),
            generation: 1,
        };
        s.reload()?;
        Ok(s)
    }

    fn reload(&mut self) -> Result<()> {
        self.highlights.clear();
        self.notes.clear();
        self.bookmarks.clear();
        let mut st = self.conn.prepare("SELECT book, chapter, verse, color, created FROM highlights")?;
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
                r.get(4)?,
            ))
        })?;
        for row in rows {
            let (b, c, v, color, created) = row?;
            if let (Some(vr), Some(color)) = (verse_from_row(&b, c, v), HighlightColor::from_i64(color)) {
                self.highlights.insert(vr.key(), (color, created));
            }
        }
        let mut st = self.conn.prepare("SELECT book, chapter, verse, body, created, updated FROM notes")?;
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })?;
        for row in rows {
            let (b, c, v, body, created, updated) = row?;
            if let Some(vr) = verse_from_row(&b, c, v) {
                self.notes.insert(vr.key(), Note { body, created, updated });
            }
        }
        let mut st = self.conn.prepare("SELECT book, chapter, verse, label, created FROM bookmarks")?;
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
                r.get(4)?,
            ))
        })?;
        for row in rows {
            let (b, c, v, label, created) = row?;
            if let Some(vr) = verse_from_row(&b, c, v) {
                self.bookmarks.insert(vr.key(), Bookmark { label, created });
            }
        }
        self.generation += 1;
        Ok(())
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
        let code = v.book.info().code;
        match color {
            Some(c) => {
                let created = now();
                self.conn.execute(
                    "INSERT INTO highlights(book, chapter, verse, color, created) VALUES(?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(book, chapter, verse) DO UPDATE SET color = excluded.color",
                    params![code, v.chapter, v.verse, c.to_i64(), created],
                )?;
                let created = self.highlights.get(&v.key()).map_or(created, |(_, t)| *t);
                self.highlights.insert(v.key(), (c, created));
            }
            None => {
                self.conn.execute(
                    "DELETE FROM highlights WHERE book = ?1 AND chapter = ?2 AND verse = ?3",
                    params![code, v.chapter, v.verse],
                )?;
                self.highlights.remove(&v.key());
            }
        }
        self.generation += 1;
        Ok(())
    }

    /// Saves a note; an empty (whitespace-only) body deletes it.
    pub fn set_note(&mut self, v: VerseRef, body: &str) -> Result<()> {
        let code = v.book.info().code;
        if body.trim().is_empty() {
            self.conn.execute(
                "DELETE FROM notes WHERE book = ?1 AND chapter = ?2 AND verse = ?3",
                params![code, v.chapter, v.verse],
            )?;
            self.notes.remove(&v.key());
        } else {
            let t = now();
            self.conn.execute(
                "INSERT INTO notes(book, chapter, verse, body, created, updated) VALUES(?1, ?2, ?3, ?4, ?5, ?5)
                 ON CONFLICT(book, chapter, verse) DO UPDATE SET body = excluded.body, updated = excluded.updated",
                params![code, v.chapter, v.verse, body, t],
            )?;
            let created = self.notes.get(&v.key()).map_or(t, |n| n.created);
            self.notes.insert(v.key(), Note { body: body.to_string(), created, updated: t });
        }
        self.generation += 1;
        Ok(())
    }

    pub fn set_bookmark(&mut self, v: VerseRef, label: Option<&str>) -> Result<()> {
        let code = v.book.info().code;
        match label {
            Some(label) => {
                let t = now();
                self.conn.execute(
                    "INSERT INTO bookmarks(book, chapter, verse, label, created) VALUES(?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(book, chapter, verse) DO UPDATE SET label = excluded.label",
                    params![code, v.chapter, v.verse, label, t],
                )?;
                let created = self.bookmarks.get(&v.key()).map_or(t, |b| b.created);
                self.bookmarks.insert(v.key(), Bookmark { label: label.to_string(), created });
            }
            None => {
                self.conn.execute(
                    "DELETE FROM bookmarks WHERE book = ?1 AND chapter = ?2 AND verse = ?3",
                    params![code, v.chapter, v.verse],
                )?;
                self.bookmarks.remove(&v.key());
            }
        }
        self.generation += 1;
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
    pub fn record_reading(&self, s: &ReadingSession) -> Result<()> {
        self.conn.execute(
            "INSERT INTO reading(started, book, chapter, translation, seconds, furthest, verses)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(started) DO UPDATE SET seconds = excluded.seconds, furthest = excluded.furthest,
               verses = excluded.verses, translation = excluded.translation",
            params![s.started, s.book.info().code, s.chapter, s.translation, s.seconds, s.furthest, s.verses],
        )?;
        Ok(())
    }

    /// Reading sessions that started at or after `since` (Unix seconds), newest first.
    pub fn reading_since(&self, since: i64) -> Vec<ReadingSession> {
        let Ok(mut st) = self.conn.prepare_cached(
            "SELECT started, book, chapter, translation, seconds, furthest, verses FROM reading
             WHERE started >= ?1 ORDER BY started DESC",
        ) else {
            return Vec::new();
        };
        let rows = st.query_map([since], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, i64>(6)?,
            ))
        });
        let Ok(rows) = rows else { return Vec::new() };
        rows.filter_map(|r| r.ok())
            .filter_map(|(started, book, chapter, translation, seconds, furthest, verses)| {
                Some(ReadingSession {
                    started,
                    book: BookId::from_code(&book)?,
                    chapter: u8::try_from(chapter).ok()?,
                    translation,
                    seconds: u32::try_from(seconds).ok()?,
                    furthest: u8::try_from(furthest).ok()?,
                    verses: u8::try_from(verses).ok()?,
                })
            })
            .collect()
    }

    /// Forgets reading history from `since` on (all of it for 0).
    pub fn clear_reading(&self, since: i64) -> Result<()> {
        self.conn.execute("DELETE FROM reading WHERE started >= ?1", [since])?;
        Ok(())
    }

    pub fn setting(&self, key: &str) -> Option<String> {
        self.conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
            .optional()
            .ok()
            .flatten()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings(key, value) VALUES(?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// JSON export of all study data. `text` supplies verse text for context (may return `None`).
    pub fn export_json(
        &self,
        translation: &str,
        text: impl Fn(VerseRef) -> Option<String>,
    ) -> Result<String> {
        let entry = |v: VerseRef| ExportVerse {
            reference: v.to_string(),
            book: v.book.info().code.to_string(),
            chapter: v.chapter,
            verse: v.verse,
            text: text(v),
        };
        let doc = ExportDoc {
            app: "bible-rs".into(),
            format: 1,
            exported_at: rfc3339(now()),
            translation: translation.to_string(),
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
        Ok(serde_json::to_string_pretty(&doc)?)
    }

    pub fn export_markdown(&self, translation: &str, text: impl Fn(VerseRef) -> Option<String>) -> String {
        use std::fmt::Write;
        let mut md = String::new();
        let _ = writeln!(md, "# Bible study notes\n");
        let _ =
            writeln!(md, "Exported from bible-rs on {} · verse text: {translation}\n", &rfc3339(now())[..10]);
        let bookmarks = self.bookmarks();
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
        let highlights = self.highlights();
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
        let notes = self.notes();
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
        let tx = self.conn.unchecked_transaction()?;
        for h in &doc.highlights {
            let v = verse(&h.verse)?;
            tx.execute(
                "INSERT INTO highlights(book, chapter, verse, color, created) VALUES(?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(book, chapter, verse) DO UPDATE SET color = excluded.color",
                params![
                    v.book.info().code,
                    v.chapter,
                    v.verse,
                    h.color.to_i64(),
                    parse_rfc3339(&h.created).unwrap_or_else(now)
                ],
            )?;
        }
        for n in &doc.notes {
            let v = verse(&n.verse)?;
            tx.execute(
                "INSERT INTO notes(book, chapter, verse, body, created, updated) VALUES(?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(book, chapter, verse) DO UPDATE SET body = excluded.body, updated = excluded.updated",
                params![
                    v.book.info().code,
                    v.chapter,
                    v.verse,
                    n.note,
                    parse_rfc3339(&n.created).unwrap_or_else(now),
                    parse_rfc3339(&n.updated).unwrap_or_else(now)
                ],
            )?;
        }
        for b in &doc.bookmarks {
            let v = verse(&b.verse)?;
            tx.execute(
                "INSERT INTO bookmarks(book, chapter, verse, label, created) VALUES(?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(book, chapter, verse) DO UPDATE SET label = excluded.label",
                params![
                    v.book.info().code,
                    v.chapter,
                    v.verse,
                    b.label,
                    parse_rfc3339(&b.created).unwrap_or_else(now)
                ],
            )?;
        }
        tx.commit()?;
        self.reload()?;
        Ok((doc.highlights.len(), doc.notes.len(), doc.bookmarks.len()))
    }
}

#[derive(Serialize, Deserialize)]
struct ExportDoc {
    app: String,
    format: u32,
    exported_at: String,
    translation: String,
    highlights: Vec<ExportHighlight>,
    notes: Vec<ExportNote>,
    bookmarks: Vec<ExportBookmark>,
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
mod tests {
    use super::*;

    fn jn316() -> VerseRef {
        VerseRef::new(BookId(42), 3, 16)
    }

    #[test]
    fn reading_sessions() {
        let s = StudyStore::open_in_memory().unwrap();
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
            assert!(s.generation() > g);
        }
        let mut s = StudyStore::open(&path).unwrap();
        assert_eq!(s.highlight(jn316()), Some(HighlightColor::Green));
        assert_eq!(s.note(jn316()).unwrap().body, "God's love");
        assert!(s.is_bookmarked(VerseRef::new(BookId(0), 1, 1)));
        assert_eq!(s.setting("theme").as_deref(), Some("dark"));
        assert_eq!(s.counts(), (1, 1, 1));
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
