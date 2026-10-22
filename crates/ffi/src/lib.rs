//! The interface the phone apps use to reach bible-core, exported with UniFFI.
//!
//! `scripts/build-ios.sh` generates Swift bindings from this crate (Kotlin would come from the
//! same place). Everything that isn't drawing happens here: a chapter arrives as blocks of styled
//! runs ([`content`]), search hits with their match ranges, and study data keyed by canonical
//! verse, so it follows the reader across translations. Calls are coarse — a chapter, a page of
//! hits — and safe from any thread.

use std::ops::Range;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard};
use std::time::Instant;

use bible_core::books::{BOOKS, CHAPTER_COUNT, FIRST_NT_BOOK};
use bible_core::library::{Chapter, Library};
use bible_core::model::{Origin, Translation};
use bible_core::search::{Query, Scope};
use bible_core::study::{self, StudyStore};
use bible_core::{BookId, ChapterIdx};

pub mod content;
pub mod tokens;

pub use content::{Block, BlockKind, ChapterContent, ChapterStatus, ContentOptions, Run};
pub use tokens::{Palette, ThemeName, palette};

uniffi::setup_scaffolding!();

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum BibleError {
    #[error("{message}")]
    Failed { message: String },
}

impl From<anyhow::Error> for BibleError {
    fn from(e: anyhow::Error) -> Self {
        BibleError::Failed { message: format!("{e:#}") }
    }
}

fn fail(message: impl Into<String>) -> BibleError {
    BibleError::Failed { message: message.into() }
}

/// A verse in the canonical grid: `book` 0–65 (Genesis–Revelation), `chapter` and `verse` from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Record)]
pub struct VerseRef {
    pub book: u8,
    pub chapter: u8,
    pub verse: u8,
}

impl VerseRef {
    fn core(self) -> Result<bible_core::VerseRef, BibleError> {
        let book = BookId::new(self.book as usize).ok_or_else(|| fail(format!("no book {}", self.book)))?;
        Ok(bible_core::VerseRef::new(book, self.chapter, self.verse))
    }
}

impl From<bible_core::VerseRef> for VerseRef {
    fn from(v: bible_core::VerseRef) -> Self {
        VerseRef { book: v.book.0, chapter: v.chapter, verse: v.verse }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct BookInfo {
    pub index: u8,
    pub name: String,
    pub abbrev: String,
    pub chapters: u8,
    /// Index of the book's first chapter in the canonical grid.
    pub first_chapter: u16,
    pub new_testament: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TranslationOrigin {
    Bundled,
    Imported,
    Online,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TranslationInfo {
    pub id: String,
    pub abbrev: String,
    pub name: String,
    pub language: String,
    pub license: String,
    pub copyright: String,
    pub origin: TranslationOrigin,
    /// The year of the edition a bundled text follows.
    pub year: Option<u16>,
    /// A line about a bundled text ("Red letters · modern English").
    pub description: Option<String>,
    pub red_letters: bool,
    pub headings: bool,
}

/// A reference the user typed, resolved.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Reference {
    pub book: u8,
    pub chapter: u8,
    pub chapter_index: u16,
    pub verse: Option<u8>,
    pub verse_end: Option<u8>,
    /// "John 3:16", "Psalm 23", "1 Corinthians 13:4–7".
    pub label: String,
}

impl From<bible_core::reference::Reference> for Reference {
    fn from(r: bible_core::reference::Reference) -> Self {
        Reference {
            book: r.book.0,
            chapter: r.chapter,
            chapter_index: r.chapter_idx().0,
            verse: r.verse,
            verse_end: r.verse_end,
            label: r.display(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SearchScope {
    All,
    OldTestament,
    NewTestament,
    Book { book: u8 },
}

impl From<SearchScope> for Scope {
    fn from(s: SearchScope) -> Self {
        match s {
            SearchScope::All => Scope::All,
            SearchScope::OldTestament => Scope::OldTestament,
            SearchScope::NewTestament => Scope::NewTestament,
            SearchScope::Book { book } => BookId::new(book as usize).map_or(Scope::All, Scope::Book),
        }
    }
}

/// A stretch of a string in UTF-16 code units, which is how Swift's `NSString` and Kotlin's
/// `String` count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct TextRange {
    pub start: u32,
    pub length: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SearchHit {
    pub verse: VerseRef,
    pub chapter_index: u16,
    /// "John 3:16", with the book named as the translation names it.
    pub reference: String,
    pub text: String,
    /// The words that matched, in `text`.
    pub matches: Vec<TextRange>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SearchPage {
    /// Verses found in all.
    pub total: u32,
    pub elapsed_ms: f64,
    /// The hits from the page's offset on.
    pub hits: Vec<SearchHit>,
    /// When nothing was found: a query with one word respelled that finds verses.
    pub suggestion: Option<SearchSuggestion>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SearchSuggestion {
    pub query: String,
    pub total: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum HighlightColor {
    Yellow,
    Green,
    Blue,
    Pink,
    Purple,
    Orange,
}

impl From<study::HighlightColor> for HighlightColor {
    fn from(c: study::HighlightColor) -> Self {
        match c {
            study::HighlightColor::Yellow => HighlightColor::Yellow,
            study::HighlightColor::Green => HighlightColor::Green,
            study::HighlightColor::Blue => HighlightColor::Blue,
            study::HighlightColor::Pink => HighlightColor::Pink,
            study::HighlightColor::Purple => HighlightColor::Purple,
            study::HighlightColor::Orange => HighlightColor::Orange,
        }
    }
}

impl From<HighlightColor> for study::HighlightColor {
    fn from(c: HighlightColor) -> Self {
        match c {
            HighlightColor::Yellow => study::HighlightColor::Yellow,
            HighlightColor::Green => study::HighlightColor::Green,
            HighlightColor::Blue => study::HighlightColor::Blue,
            HighlightColor::Pink => study::HighlightColor::Pink,
            HighlightColor::Purple => study::HighlightColor::Purple,
            HighlightColor::Orange => study::HighlightColor::Orange,
        }
    }
}

/// What the reader marks on a verse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct VerseMarks {
    pub verse: u8,
    pub highlight: Option<HighlightColor>,
    pub note: bool,
    pub bookmark: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum StudyKind {
    Bookmarks,
    Highlights,
    Notes,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct StudyItem {
    pub verse: VerseRef,
    pub chapter_index: u16,
    pub reference: String,
    /// The verse in the translation asked for (empty if it hasn't the verse).
    pub text: String,
    pub highlight: Option<HighlightColor>,
    /// Which of its verse's highlights a highlight is (where it starts; 0 for a whole verse), for
    /// telling a verse's highlights apart and `remove_highlight`. 0 for notes and bookmarks.
    pub start: u32,
    /// A note's text, or a bookmark's label.
    pub body: String,
    /// Unix seconds.
    pub created: i64,
    pub updated: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ExportFormat {
    Markdown,
    /// Re-importable, by the Mac app too.
    Json,
}

/// The translations and the study store (highlights, notes, bookmarks, settings) in one data
/// folder, laid out as the Mac app's.
#[derive(uniffi::Object)]
pub struct Bible {
    library: RwLock<Library>,
    study: Mutex<StudyStore>,
}

#[uniffi::export]
impl Bible {
    /// Opens (or creates) the data folder. Reads the translations' headers and the study store;
    /// no translation is decoded yet.
    #[uniffi::constructor]
    pub fn new(data_dir: String) -> Result<Arc<Self>, BibleError> {
        let dir = PathBuf::from(data_dir);
        let study = StudyStore::open(&dir.join("study.sqlite"))?;
        let library = Library::open(&dir, Arc::new(|| {}));
        Ok(Arc::new(Bible { library: RwLock::new(library), study: Mutex::new(study) }))
    }

    pub fn translations(&self) -> Vec<TranslationInfo> {
        self.library()
            .entries()
            .iter()
            .map(|e| {
                let m = &e.meta;
                TranslationInfo {
                    id: m.id.clone(),
                    abbrev: m.abbrev.clone(),
                    name: m.name.clone(),
                    language: m.language.clone(),
                    license: m.license.clone(),
                    copyright: e.copyright(),
                    origin: match m.origin {
                        Origin::Bundled => TranslationOrigin::Bundled,
                        Origin::Imported { .. } => TranslationOrigin::Imported,
                        Origin::Online { .. } => TranslationOrigin::Online,
                    },
                    year: bible_core::bundled::year(&m.id),
                    description: bible_core::bundled::description(&m.id).map(str::to_string),
                    red_letters: m.red_letters,
                    headings: m.headings,
                }
            })
            .collect()
    }

    /// Decodes a local translation now (≈20 ms for a whole Bible), so its first chapter doesn't wait.
    pub fn prepare(&self, translation: String) -> Result<(), BibleError> {
        self.library().load(&translation)?;
        Ok(())
    }

    /// Decodes translations and builds their search text on a background thread.
    pub fn warm(&self, translations: Vec<String>) {
        self.library().warm(&translations);
    }

    /// Chapter `index` (0–1188) of a translation, ready to typeset.
    pub fn chapter(&self, translation: String, index: u16, options: ContentOptions) -> ChapterContent {
        let c = ChapterIdx(index.min(CHAPTER_COUNT as u16 - 1));
        let lib = self.library();
        let meta = lib.get(&translation).map(|e| e.meta.clone());
        let status = match lib.chapter(&translation, c) {
            Chapter::Ready(t) => return content::chapter(&t, c, options),
            Chapter::Missing => ChapterStatus::Missing,
            Chapter::Pending => ChapterStatus::Pending,
            Chapter::NoKey => ChapterStatus::NoKey,
            Chapter::Failed(message) => ChapterStatus::Failed { message },
        };
        ChapterContent::unavailable(meta.as_ref(), c, status)
    }

    /// Searches a local translation: every word must appear (as a word prefix unless
    /// `whole_words`), `"quoted phrases"` in order, `-words` excluded. Returns `limit` hits from
    /// `offset` on.
    pub fn search(
        &self,
        translation: String,
        query: String,
        scope: SearchScope,
        whole_words: bool,
        offset: u32,
        limit: u32,
    ) -> Result<SearchPage, BibleError> {
        let t = self.local(&translation)?;
        let start = Instant::now();
        let q = Query::parse(&query, whole_words);
        let scope = Scope::from(scope);
        let verses = t.search_index().search(&t, &q, scope);
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        let hits = verses
            .iter()
            .skip(offset as usize)
            .take(limit as usize)
            .filter_map(|&i| t.verse_view(i))
            .map(|v| {
                let text = v.plain_text();
                let r = v.reference();
                SearchHit {
                    verse: r.into(),
                    chapter_index: v.chapter.0,
                    reference: format!("{} {}:{}", t.meta.ref_name(r.book), r.chapter, r.verse),
                    matches: utf16_ranges(&text, &q.match_ranges(&text)),
                    text,
                }
            })
            .collect();
        let suggestion = (verses.is_empty() && offset == 0)
            .then(|| bible_core::search::suggest(&t, &query, scope, whole_words))
            .flatten()
            .map(|(query, total, _)| SearchSuggestion { query, total: total as u32 });
        Ok(SearchPage { total: verses.len() as u32, elapsed_ms, hits, suggestion })
    }

    /// Verses for the clipboard or a share sheet: `“text” — John 3:16 (KJV)`, or for several a
    /// reference line and numbered verses.
    pub fn copy_text(&self, translation: String, verses: Vec<VerseRef>) -> Result<String, BibleError> {
        let t = self.local(&translation)?;
        let refs = verses.into_iter().map(VerseRef::core).collect::<Result<Vec<_>, _>>()?;
        bible_core::copy::format_verses(&t, &refs)
            .ok_or_else(|| fail("those verses aren't in this translation"))
    }

    /// Highlights, notes and bookmarks on the verses of a chapter (as numbered in `translation`).
    pub fn marks(&self, translation: String, chapter: u16) -> Vec<VerseMarks> {
        let c = ChapterIdx(chapter.min(CHAPTER_COUNT as u16 - 1));
        let numbers: Vec<u8> = match self.local(&translation) {
            Ok(t) if t.has_chapter(c) => t.verses_in(c).map(|v| v.number).collect(),
            _ => (1..=c.kjv_verses()).collect(),
        };
        let study = self.study();
        numbers
            .into_iter()
            .filter_map(|n| {
                let r = bible_core::VerseRef::new(c.book(), c.chapter(), n);
                let marks = VerseMarks {
                    verse: n,
                    highlight: study.highlight(r).map(Into::into),
                    note: study.note(r).is_some(),
                    bookmark: study.is_bookmarked(r),
                };
                (marks.highlight.is_some() || marks.note || marks.bookmark).then_some(marks)
            })
            .collect()
    }

    /// Sets or (with `None`) clears a verse's highlight.
    pub fn set_highlight(&self, verse: VerseRef, color: Option<HighlightColor>) -> Result<(), BibleError> {
        Ok(self.study().set_highlight(verse.core()?, color.map(Into::into))?)
    }

    /// Removes one of a verse's highlights: the one a study item's `start` names.
    pub fn remove_highlight(&self, verse: VerseRef, start: u32) -> Result<(), BibleError> {
        Ok(self.study().remove_highlight(verse.core()?, start)?)
    }

    pub fn note(&self, verse: VerseRef) -> Result<Option<String>, BibleError> {
        Ok(self.study().note(verse.core()?).map(|n| n.body.clone()))
    }

    /// Saves a note; an empty one deletes it.
    pub fn set_note(&self, verse: VerseRef, body: String) -> Result<(), BibleError> {
        Ok(self.study().set_note(verse.core()?, &body)?)
    }

    pub fn set_bookmark(&self, verse: VerseRef, on: bool) -> Result<(), BibleError> {
        Ok(self.study().set_bookmark(verse.core()?, on.then_some(""))?)
    }

    /// Bookmarks and notes in canonical order, highlights the newest first, each with its verse's
    /// text in `translation`.
    pub fn study_items(&self, kind: StudyKind, translation: String) -> Vec<StudyItem> {
        let t = self.local(&translation).ok();
        let study = self.study();
        let text = |r| t.as_ref().and_then(|t| t.find(r)).map(|v| v.plain_text()).unwrap_or_default();
        let item = |r: bible_core::VerseRef, highlight, body: &str, created, updated| StudyItem {
            verse: r.into(),
            chapter_index: r.chapter_idx().0,
            reference: r.to_string(),
            text: text(r),
            highlight,
            start: 0,
            body: body.to_string(),
            created,
            updated,
        };
        match kind {
            StudyKind::Bookmarks => study
                .bookmarks()
                .into_iter()
                .map(|(r, b)| item(r, None, &b.label, b.created, b.created))
                .collect(),
            // Part of a verse shows the words it covers.
            StudyKind::Highlights => study
                .highlights_dated()
                .into_iter()
                .map(|(r, h)| {
                    let mut it = item(r, Some(h.color.into()), "", h.created, h.created);
                    if !h.is_whole() {
                        it.text = h.excerpt(&it.text);
                    }
                    it.start = h.start;
                    it
                })
                .collect(),
            StudyKind::Notes => study
                .notes()
                .into_iter()
                .map(|(r, n)| item(r, study.highlight(r).map(Into::into), &n.body, n.created, n.updated))
                .collect(),
        }
    }

    /// Highlights, notes and bookmarks as Markdown or (re-importable) JSON, with verse text from
    /// `translation`.
    pub fn export(&self, format: ExportFormat, translation: String) -> Result<String, BibleError> {
        let t = self.local(&translation).ok();
        let abbrev = t.as_ref().map_or_else(|| translation.clone(), |t| t.meta.abbrev.clone());
        let text = |r: bible_core::VerseRef| t.as_ref().and_then(|t| t.find(r)).map(|v| v.plain_text());
        let study = self.study();
        Ok(match format {
            ExportFormat::Markdown => study.export_markdown(&abbrev, text),
            ExportFormat::Json => study.export_json(&abbrev, text)?,
        })
    }

    pub fn setting(&self, key: String) -> Option<String> {
        self.study().setting(&key)
    }

    pub fn set_setting(&self, key: String, value: String) -> Result<(), BibleError> {
        Ok(self.study().set_setting(&key, &value)?)
    }
}

impl Bible {
    fn library(&self) -> RwLockReadGuard<'_, Library> {
        self.library.read().unwrap_or_else(|e| e.into_inner())
    }

    fn study(&self) -> MutexGuard<'_, StudyStore> {
        self.study.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn local(&self, id: &str) -> Result<Arc<Translation>, BibleError> {
        Ok(self.library().load(id)?)
    }
}

/// The 66 books of the canonical grid.
#[uniffi::export]
pub fn books() -> Vec<BookInfo> {
    BOOKS
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let id = BookId(i as u8);
            BookInfo {
                index: i as u8,
                name: b.name.to_string(),
                abbrev: b.abbrev.to_string(),
                chapters: b.chapters,
                first_chapter: id.first_chapter().0,
                new_testament: id >= FIRST_NT_BOOK,
            }
        })
        .collect()
}

/// The best reading of a typed reference ("jn 3:16", "Rom 8", "1co13:4-7", "revalation 21").
#[uniffi::export]
pub fn parse_reference(input: String) -> Option<Reference> {
    bible_core::reference::parse(&input).map(Into::into)
}

/// Readings of a typed reference, the best first.
#[uniffi::export]
pub fn suggest_references(input: String, limit: u32) -> Vec<Reference> {
    bible_core::reference::suggest(&input, limit as usize).into_iter().map(Into::into).collect()
}

/// "John 3", "Psalm 23".
#[uniffi::export]
pub fn chapter_label(index: u16) -> String {
    ChapterIdx(index.min(CHAPTER_COUNT as u16 - 1)).display()
}

/// "John 3:16", "John 3:16–18", "John 3:16–4:2".
#[uniffi::export]
pub fn range_label(from: VerseRef, to: VerseRef) -> String {
    match (from.core(), to.core()) {
        (Ok(a), Ok(b)) => bible_core::reference::format_range(a.min(b), a.max(b)),
        _ => String::new(),
    }
}

fn utf16_ranges(text: &str, ranges: &[Range<usize>]) -> Vec<TextRange> {
    ranges
        .iter()
        .filter_map(|r| {
            let start = text.get(..r.start)?.encode_utf16().count();
            let length = text.get(r.clone())?.encode_utf16().count();
            Some(TextRange { start: start as u32, length: length as u32 })
        })
        .collect()
}
