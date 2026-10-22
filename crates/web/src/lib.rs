//! bible-core in the browser, for the web reader at `/read` (`site/`). The page fetches `.bbl`
//! files and hands their bytes over; decoding, chapter layout, search, references, copy formats
//! and the study store run here, as they do on the Mac and the phone. Only drawing is left to the
//! page.
//!
//! Values cross as plain JavaScript objects with camelCase fields (`site/src/lib/read/core.ts`
//! types them). Text ranges are UTF-16 offsets, which is how JavaScript strings count.

use std::collections::HashMap;
use std::ops::Range;

use bible_core::books::{BOOKS, CHAPTER_COUNT, FIRST_NT_BOOK};
use bible_core::content::{self, ContentOptions};
use bible_core::highlight::{chars_from_utf16, utf16_from_chars};
use bible_core::model::{Translation, TranslationMeta};
use bible_core::search::{Query, Scope};
use bible_core::study::{HighlightColor, Include, ReadingSession, StudyStore, TextSpan};
use bible_core::{BookId, ChapterIdx, VerseRef, bundled, format, nlt_html, plans};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[cfg(feature = "import")]
mod import;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TranslationInfo {
    id: String,
    abbrev: String,
    name: String,
    language: String,
    license: String,
    copyright: String,
    /// The year of the edition a bundled text follows.
    year: Option<u16>,
    /// A line about a bundled text ("Red letters · modern English").
    description: Option<&'static str>,
    red_letters: bool,
    headings: bool,
    /// The decoded text's size in bytes, from the file's header.
    size: Option<u64>,
}

impl TranslationInfo {
    fn new(m: &TranslationMeta, size: Option<u64>) -> Self {
        TranslationInfo {
            id: m.id.clone(),
            abbrev: m.abbrev.clone(),
            name: m.name.clone(),
            language: m.language.clone(),
            license: m.license.clone(),
            copyright: m.copyright.clone(),
            year: bundled::year(&m.id),
            description: bundled::description(&m.id),
            red_letters: m.red_letters,
            headings: m.headings,
            size,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
struct Verse {
    book: u8,
    chapter: u8,
    verse: u8,
}

impl Verse {
    fn core(self) -> Result<VerseRef, JsError> {
        let book =
            BookId::new(self.book as usize).ok_or_else(|| JsError::new(&format!("no book {}", self.book)))?;
        Ok(VerseRef::new(book, self.chapter, self.verse))
    }
}

impl From<VerseRef> for Verse {
    fn from(v: VerseRef) -> Self {
        Verse { book: v.book.0, chapter: v.chapter, verse: v.verse }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Reference {
    book: u8,
    chapter: u8,
    chapter_index: u16,
    verse: Option<u8>,
    verse_end: Option<u8>,
    /// "John 3:16", "Psalm 23", "1 Corinthians 13:4–7".
    label: String,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Book {
    index: u8,
    name: &'static str,
    abbrev: &'static str,
    chapters: u8,
    first_chapter: u16,
    new_testament: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchHit {
    verse: Verse,
    chapter_index: u16,
    /// "John 3:16", with the book named as the translation names it.
    reference: String,
    text: String,
    /// The words that matched, `[start, end)` in `text`.
    matches: Vec<[u32; 2]>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BookCount {
    book: u8,
    /// The book's name as the translation spells it.
    name: String,
    count: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchPage {
    /// Verses found in all.
    total: u32,
    /// The books with hits and how many each, in the order of the hits.
    books: Vec<BookCount>,
    /// The hits from the page's offset on.
    hits: Vec<SearchHit>,
    /// When nothing was found: a query with one word respelled that finds verses.
    suggestion: Option<SearchSuggestion>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchSuggestion {
    query: String,
    total: u32,
}

/// What the reader marks on a verse.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VerseMarks {
    verse: u8,
    /// The verse's highlight colour (the newest, when parts of it differ).
    highlight: Option<HighlightColor>,
    /// What's highlighted of the verse's text, in order: whole verses and parts of them.
    highlights: Vec<HighlightRange>,
    note: bool,
    bookmark: bool,
}

/// A highlighted stretch of a verse, `[start, end)` in UTF-16 units of its text (as runs'
/// `offset` counts).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HighlightRange {
    start: u32,
    end: u32,
    color: HighlightColor,
}

/// A stretch of text in one chapter: from `start` in verse `first` to before `end` in verse
/// `last`, UTF-16 offsets into the verse texts (an `end` past a verse's text runs to its end).
#[derive(Deserialize)]
struct Span {
    first: Verse,
    start: u32,
    last: Verse,
    end: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Note {
    body: String,
    created: i64,
    updated: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StudyItem {
    verse: Verse,
    chapter_index: u16,
    reference: String,
    /// The verse in the translation asked for (empty if it hasn't the verse or isn't loaded).
    text: String,
    highlight: Option<HighlightColor>,
    /// A highlight's place in its verse, for `removeHighlight` (0 for a whole verse).
    start: u32,
    /// A note's text, or a bookmark's label.
    body: String,
    /// Unix seconds.
    created: i64,
    updated: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Counts {
    highlights: usize,
    notes: usize,
    bookmarks: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Session {
    /// Unix seconds; also the session's identity.
    started: i64,
    book: u8,
    chapter: u8,
    translation: String,
    seconds: u32,
    furthest: u8,
    verses: u8,
}

impl From<ReadingSession> for Session {
    fn from(s: ReadingSession) -> Self {
        Session {
            started: s.started,
            book: s.book.0,
            chapter: s.chapter,
            translation: s.translation,
            seconds: s.seconds,
            furthest: s.furthest,
            verses: s.verses,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanProgress {
    id: String,
    started: i64,
    finished: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ExportOptions {
    highlights: bool,
    notes: bool,
    bookmarks: bool,
    /// Reading history and plans.
    history: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        let i = Include::STUDY;
        ExportOptions { highlights: i.highlights, notes: i.notes, bookmarks: i.bookmarks, history: i.history }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanInfo {
    id: &'static str,
    title: &'static str,
    /// Length and pace ("90 days · ~3 chapters a day").
    meta: &'static str,
    description: &'static str,
    icon: &'static str,
    days: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanDay {
    /// "Luke 5–6", "Psalms 1, 31", "Matthew 5:1–16".
    label: String,
    passages: Vec<Passage>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Passage {
    chapter_index: u16,
    /// "Luke 5", "Matthew 5:1–16".
    label: String,
    /// The verses read, when not the whole chapter.
    verses: Option<[u8; 2]>,
}

/// The translations the page has loaded, decoded, and the study store.
#[wasm_bindgen]
#[derive(Default)]
pub struct Bible {
    translations: HashMap<String, Translation>,
    study: StudyStore,
}

#[wasm_bindgen]
impl Bible {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Bible {
        Bible::default()
    }

    /// Decodes a `.bbl` file (≈10 ms for a whole Bible) and keeps it under its id, which is
    /// returned with its metadata.
    pub fn load(&mut self, bytes: &[u8]) -> Result<JsValue, JsError> {
        let t = format::decode(bytes).map_err(err)?;
        let info = TranslationInfo::new(&t.meta, format::decoded_len(bytes));
        self.translations.insert(t.meta.id.clone(), t);
        to_js(&info)
    }

    /// Keeps one chapter of the NLT, read from the HTML Tyndale's API returned (through the site's
    /// `/api/nlt`), in place of any chapter of it kept before: Tyndale's terms allow quoting up to
    /// 500 verses and never a whole book, so the page holds one chapter at a time and stores none.
    #[wasm_bindgen(js_name = loadNlt)]
    pub fn load_nlt(&mut self, index: u16, html: &str) -> Result<JsValue, JsError> {
        let c = chapter_idx(index);
        let mut t = nlt_html::parse_html(c, html);
        if t.verse_count() == 0 {
            return Err(JsError::new(&format!("no text came back for {}", c.display())));
        }
        let m = &mut t.meta;
        m.abbrev = "NLT".into();
        m.name = "New Living Translation".into();
        m.language = "en".into();
        m.license = "Licensed text fetched as you read via the NLT API (Tyndale)".into();
        m.copyright = nlt_html::COPYRIGHT.into();
        let info = TranslationInfo::new(&t.meta, None);
        self.translations.insert(t.meta.id.clone(), t);
        to_js(&info)
    }

    /// Lets a translation go (the NLT's chapter, once the reader moves on).
    pub fn unload(&mut self, translation: &str) {
        self.translations.remove(translation);
    }

    /// Whether a translation is loaded.
    pub fn has(&self, translation: &str) -> bool {
        self.translations.contains_key(translation)
    }

    /// Builds a translation's search text now (≈30 ms), so the first query doesn't wait for it.
    #[wasm_bindgen(js_name = prepareSearch)]
    pub fn prepare_search(&self, translation: &str) -> Result<(), JsError> {
        self.get(translation)?.search_index();
        Ok(())
    }

    /// Chapter `index` (0–1188) as blocks of styled runs, ready to typeset. A translation that
    /// isn't loaded yet gives a `pending` chapter; one without the chapter a `missing` one.
    pub fn chapter(&self, translation: &str, index: u16, options: JsValue) -> Result<JsValue, JsError> {
        let options: ContentOptions = if options.is_undefined() || options.is_null() {
            ContentOptions::default()
        } else {
            serde_wasm_bindgen::from_value(options)?
        };
        let c = chapter_idx(index);
        let out = match self.translations.get(translation) {
            Some(t) if t.has_chapter(c) => content::chapter(t, c, options),
            Some(t) => {
                content::ChapterContent::unavailable(Some(&t.meta), c, content::ChapterStatus::Missing)
            }
            None => content::ChapterContent::unavailable(None, c, content::ChapterStatus::Pending),
        };
        to_js(&out)
    }

    /// The verses of a chapter, each as one paragraph (parallel columns). Empty when the
    /// translation hasn't the chapter.
    pub fn verses(&self, translation: &str, index: u16) -> Result<JsValue, JsError> {
        let t = self.get(translation)?;
        let verses: Vec<_> = t.verses_in(chapter_idx(index)).map(|v| content::verse(t, &v)).collect();
        to_js(&verses)
    }

    /// One verse as a paragraph, or null when the translation hasn't it (comparisons).
    pub fn verse(&self, translation: &str, verse: JsValue) -> Result<JsValue, JsError> {
        let t = self.get(translation)?;
        match t.find(verse_arg(verse)?) {
            Some(v) => to_js(&content::verse(t, &v)),
            None => Ok(JsValue::NULL),
        }
    }

    /// Searches a translation: every word must appear (as a word prefix unless `whole_words`),
    /// `"quoted phrases"` in order, `-words` excluded. `scope` is "all", "ot", "nt" or a book
    /// index. With `first_book` (a book index, or -1) that book's hits come first, the rest in
    /// canonical order. Returns `limit` hits from `offset` on.
    #[allow(clippy::too_many_arguments)]
    pub fn search(
        &self,
        translation: &str,
        query: &str,
        scope: &str,
        whole_words: bool,
        first_book: i32,
        offset: u32,
        limit: u32,
    ) -> Result<JsValue, JsError> {
        let t = self.get(translation)?;
        let q = Query::parse(query, whole_words);
        let scope = match scope {
            "ot" => Scope::OldTestament,
            "nt" => Scope::NewTestament,
            s => s.parse::<usize>().ok().and_then(BookId::new).map_or(Scope::All, Scope::Book),
        };
        let mut verses = t.search_index().search(t, &q, scope);
        let book_of = |i: u32| t.verse_view(i).map(|v| v.chapter.book());
        if let Some(first) = usize::try_from(first_book).ok().and_then(BookId::new) {
            verses.sort_by_key(|&i| book_of(i) != Some(first));
        }
        let mut books: Vec<BookCount> = Vec::new();
        for &i in &verses {
            let Some(book) = book_of(i) else { continue };
            match books.last_mut() {
                Some(b) if b.book == book.0 => b.count += 1,
                _ => books.push(BookCount {
                    book: book.0,
                    name: t.meta.book_short(book).to_string(),
                    count: 1,
                }),
            }
        }
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
            .then(|| bible_core::search::suggest(t, query, scope, whole_words))
            .flatten()
            .map(|(query, total, _)| SearchSuggestion { query, total: total as u32 });
        to_js(&SearchPage { total: verses.len() as u32, books, hits, suggestion })
    }

    /// Verses as text for the clipboard or a share sheet. `format` is "plain" (the verses
    /// joined), "reference" (`“text” — John 3:16 (KJV)`, or a reference line and numbered verses
    /// for several) or "markdown" (a quote with its reference). Null when none of the verses are
    /// in the translation.
    #[wasm_bindgen(js_name = copyText)]
    pub fn copy_text(
        &self,
        translation: &str,
        verses: JsValue,
        format: &str,
    ) -> Result<Option<String>, JsError> {
        let t = self.get(translation)?;
        let mut refs = verses_arg(verses)?;
        refs.sort();
        refs.dedup();
        let plain: Vec<String> = refs.iter().filter_map(|r| t.find(*r)).map(|v| v.plain_text()).collect();
        if plain.is_empty() {
            return Ok(None);
        }
        let plain = plain.join(" ");
        let reference = match (refs.first(), refs.last()) {
            (Some(a), Some(b)) => {
                format!("{} ({})", bible_core::reference::format_range(*a, *b), t.meta.abbrev)
            }
            _ => String::new(),
        };
        Ok(Some(match format {
            "plain" => plain,
            "markdown" => format!("> {plain}\n>\n> — {reference}"),
            _ => bible_core::copy::format_verses(t, &refs)
                .unwrap_or_else(|| format!("“{plain}” — {reference}")),
        }))
    }

    /// Words chosen in `translation` for the clipboard: "plain" (just the words) or "reference"
    /// (`“so loved the world” — John 3:16 (KJV)`). Null when the translation hasn't them.
    #[wasm_bindgen(js_name = copySpan)]
    pub fn copy_span(
        &self,
        translation: &str,
        span: JsValue,
        format: &str,
    ) -> Result<Option<String>, JsError> {
        let t = self.get(translation)?;
        let span = text_span(t, span)?;
        Ok(match format {
            "plain" => bible_core::copy::span_text(t, &span),
            _ => bible_core::copy::format_span(t, &span),
        })
    }

    /// Highlights, notes and bookmarks on the verses of a chapter (as numbered in `translation`,
    /// or the KJV's numbering when it isn't loaded).
    pub fn marks(&self, translation: &str, index: u16) -> Result<JsValue, JsError> {
        let c = chapter_idx(index);
        let t = self.translations.get(translation).filter(|t| t.has_chapter(c));
        let numbers: Vec<u8> = match t {
            Some(t) => t.verses_in(c).map(|v| v.number).collect(),
            None => (1..=c.kjv_verses()).collect(),
        };
        let marks: Vec<VerseMarks> = numbers
            .into_iter()
            .filter_map(|n| {
                let r = VerseRef::new(c.book(), c.chapter(), n);
                let highlights = match t.and_then(|t| t.find(r)).filter(|_| self.study.highlight(r).is_some())
                {
                    Some(v) => self
                        .study
                        .highlight_ranges(r, v.text)
                        .into_iter()
                        .map(|(range, color)| HighlightRange {
                            start: utf16_from_chars(v.text, range.start) as u32,
                            end: utf16_from_chars(v.text, range.end) as u32,
                            color,
                        })
                        .collect(),
                    None => Vec::new(),
                };
                let marks = VerseMarks {
                    verse: n,
                    highlight: self.study.highlight(r),
                    highlights,
                    note: self.study.note(r).is_some(),
                    bookmark: self.study.is_bookmarked(r),
                };
                (marks.highlight.is_some() || marks.note || marks.bookmark).then_some(marks)
            })
            .collect();
        to_js(&marks)
    }

    /// Increments whenever study data changes.
    pub fn generation(&self) -> f64 {
        self.study.generation() as f64
    }

    /// Sets or (with null) clears a verse's highlight: "yellow", "green", "blue", "pink",
    /// "purple" or "orange".
    #[wasm_bindgen(js_name = setHighlight)]
    pub fn set_highlight(&mut self, verse: JsValue, color: Option<String>) -> Result<(), JsError> {
        let color = color_arg(color)?;
        self.study.set_highlight(verse_arg(verse)?, color).map_err(err)
    }

    /// Colours a stretch of text as read in `translation` (`{first, start, last, end}`, UTF-16
    /// offsets into the verse texts), or with null clears it: over the highlights there, which it
    /// replaces where they overlap.
    #[wasm_bindgen(js_name = highlightSpan)]
    pub fn highlight_span(
        &mut self,
        translation: &str,
        span: JsValue,
        color: Option<String>,
    ) -> Result<(), JsError> {
        let color = color_arg(color)?;
        let Bible { translations, study } = self;
        let t = translations
            .get(translation)
            .ok_or_else(|| JsError::new(&format!("{translation} isn't loaded")))?;
        let span = text_span(t, span)?;
        let text = |r: VerseRef| t.find(r).map(|v| v.text.to_string());
        study.highlight_span(&span, color, text).map_err(err)
    }

    /// The colour every letter of a stretch of text has, if one does.
    #[wasm_bindgen(js_name = spanColor)]
    pub fn span_color(&self, translation: &str, span: JsValue) -> Result<Option<String>, JsError> {
        let t = self.get(translation)?;
        let span = text_span(t, span)?;
        let text = |r: VerseRef| t.find(r).map(|v| v.text.to_string());
        Ok(self.study.span_color(&span, text).map(|c| c.name().to_string()))
    }

    /// Removes one highlight of a verse: the one a study item's `start` names.
    #[wasm_bindgen(js_name = removeHighlight)]
    pub fn remove_highlight(&mut self, verse: JsValue, start: u32) -> Result<(), JsError> {
        self.study.remove_highlight(verse_arg(verse)?, start).map_err(err)
    }

    pub fn note(&self, verse: JsValue) -> Result<JsValue, JsError> {
        match self.study.note(verse_arg(verse)?) {
            Some(n) => to_js(&Note { body: n.body.clone(), created: n.created, updated: n.updated }),
            None => Ok(JsValue::NULL),
        }
    }

    /// Saves a note; an empty one deletes it.
    #[wasm_bindgen(js_name = setNote)]
    pub fn set_note(&mut self, verse: JsValue, body: &str) -> Result<(), JsError> {
        self.study.set_note(verse_arg(verse)?, body).map_err(err)
    }

    /// Bookmarks a verse (with a label, possibly empty) or, with null, removes its bookmark.
    #[wasm_bindgen(js_name = setBookmark)]
    pub fn set_bookmark(&mut self, verse: JsValue, label: Option<String>) -> Result<(), JsError> {
        self.study.set_bookmark(verse_arg(verse)?, label.as_deref()).map_err(err)
    }

    /// Bookmarks a verse or removes its bookmark; returns whether it's bookmarked now.
    #[wasm_bindgen(js_name = toggleBookmark)]
    pub fn toggle_bookmark(&mut self, verse: JsValue) -> Result<bool, JsError> {
        self.study.toggle_bookmark(verse_arg(verse)?).map_err(err)
    }

    /// "bookmarks" and "notes" in canonical order, "highlights" the newest first, each with its
    /// verse's text in `translation`.
    #[wasm_bindgen(js_name = studyItems)]
    pub fn study_items(&self, kind: &str, translation: &str) -> Result<JsValue, JsError> {
        let t = self.translations.get(translation);
        let study = &self.study;
        let item = |r: VerseRef, highlight, body: &str, created, updated| StudyItem {
            verse: r.into(),
            chapter_index: r.chapter_idx().0,
            reference: r.to_string(),
            text: t.and_then(|t| t.find(r)).map(|v| v.plain_text()).unwrap_or_default(),
            highlight,
            start: 0,
            body: body.to_string(),
            created,
            updated,
        };
        let items: Vec<StudyItem> = match kind {
            "bookmarks" => study
                .bookmarks()
                .into_iter()
                .map(|(r, b)| item(r, None, &b.label, b.created, b.created))
                .collect(),
            // Part of a verse shows the words it covers.
            "highlights" => study
                .highlights_dated()
                .into_iter()
                .map(|(r, h)| {
                    let mut it = item(r, Some(h.color), "", h.created, h.created);
                    if !h.is_whole() {
                        it.text = h.excerpt(&it.text);
                    }
                    it.start = h.start;
                    it
                })
                .collect(),
            "notes" => study
                .notes()
                .into_iter()
                .map(|(r, n)| item(r, study.highlight(r), &n.body, n.created, n.updated))
                .collect(),
            other => return Err(JsError::new(&format!("no study list {other}"))),
        };
        to_js(&items)
    }

    pub fn counts(&self) -> Result<JsValue, JsError> {
        let (highlights, notes, bookmarks) = self.study.counts();
        to_js(&Counts { highlights, notes, bookmarks })
    }

    /// The study data as "markdown" or (re-importable, by the Mac app too) "json", with verse
    /// text from `translation`. `include` picks highlights, notes, bookmarks and history
    /// (reading sessions and plans); left out, it's the first three.
    #[wasm_bindgen(js_name = exportStudy)]
    pub fn export_study(&self, format: &str, translation: &str, include: JsValue) -> Result<String, JsError> {
        let o: ExportOptions = if include.is_undefined() || include.is_null() {
            ExportOptions::default()
        } else {
            serde_wasm_bindgen::from_value(include)?
        };
        let include =
            Include { highlights: o.highlights, notes: o.notes, bookmarks: o.bookmarks, history: o.history };
        let t = self.translations.get(translation);
        let abbrev = t.map_or_else(|| translation.to_string(), |t| t.meta.abbrev.clone());
        let text = |r: VerseRef| t.and_then(|t| t.find(r)).map(|v| v.plain_text());
        match format {
            "markdown" => Ok(self.study.export_markdown_with(&abbrev, text, include)),
            _ => self.study.export_json_with(&abbrev, text, include).map_err(err),
        }
    }

    /// Merges a JSON export in (entries for the same verse are overwritten); returns how many
    /// highlights, notes and bookmarks it held.
    #[wasm_bindgen(js_name = importStudy)]
    pub fn import_study(&mut self, json: &str) -> Result<JsValue, JsError> {
        let (highlights, notes, bookmarks) = self.study.import_json(json).map_err(err)?;
        to_js(&Counts { highlights, notes, bookmarks })
    }

    /// Everything in the study store, for the page to keep (`restore` reads it back).
    pub fn snapshot(&self) -> Result<String, JsError> {
        self.study.export_json_with("", |_| None, Include::EVERYTHING).map_err(err)
    }

    /// Replaces the study store with a `snapshot`.
    pub fn restore(&mut self, json: &str) -> Result<(), JsError> {
        let mut study = StudyStore::new();
        study.import_json(json).map_err(err)?;
        self.study = study;
        Ok(())
    }

    /// Erases every highlight, note, bookmark, plan and the reading history.
    #[wasm_bindgen(js_name = eraseStudy)]
    pub fn erase_study(&mut self) -> Result<(), JsError> {
        self.study.erase_all().map_err(err)
    }

    /// Saves a reading session, replacing an earlier save of the same session.
    #[wasm_bindgen(js_name = recordReading)]
    pub fn record_reading(&mut self, session: JsValue) -> Result<(), JsError> {
        let s: Session = serde_wasm_bindgen::from_value(session)?;
        let book =
            BookId::new(s.book as usize).ok_or_else(|| JsError::new(&format!("no book {}", s.book)))?;
        self.study
            .record_reading(&ReadingSession {
                started: s.started,
                book,
                chapter: s.chapter,
                translation: s.translation,
                seconds: s.seconds,
                furthest: s.furthest,
                verses: s.verses,
            })
            .map_err(err)
    }

    /// Reading sessions that started at or after `since` (Unix seconds), newest first.
    #[wasm_bindgen(js_name = readingSince)]
    pub fn reading_since(&self, since: f64) -> Result<JsValue, JsError> {
        let sessions: Vec<Session> =
            self.study.reading_since(since as i64).into_iter().map(Into::into).collect();
        to_js(&sessions)
    }

    /// Forgets reading history from `since` on (all of it for 0).
    #[wasm_bindgen(js_name = clearReading)]
    pub fn clear_reading(&mut self, since: f64) -> Result<(), JsError> {
        self.study.clear_reading(since as i64).map_err(err)
    }

    /// Plans started, newest first.
    pub fn plans(&self) -> Result<JsValue, JsError> {
        let plans: Vec<PlanProgress> = self
            .study
            .plans()
            .into_iter()
            .map(|p| PlanProgress { id: p.id, started: p.started, finished: p.finished })
            .collect();
        to_js(&plans)
    }

    /// Starts (or restarts) a plan on the day containing `started`, forgetting earlier progress.
    #[wasm_bindgen(js_name = startPlan)]
    pub fn start_plan(&mut self, id: &str, started: f64) -> Result<(), JsError> {
        self.study.start_plan(id, started as i64).map_err(err)
    }

    #[wasm_bindgen(js_name = finishPlan)]
    pub fn finish_plan(&mut self, id: &str, at: Option<f64>) -> Result<(), JsError> {
        self.study.finish_plan(id, at.map(|t| t as i64)).map_err(err)
    }

    #[wasm_bindgen(js_name = stopPlan)]
    pub fn stop_plan(&mut self, id: &str) -> Result<(), JsError> {
        self.study.stop_plan(id).map_err(err)
    }

    /// The passages ticked off in a plan, as `[day, passage index]` pairs, in order.
    #[wasm_bindgen(js_name = planReads)]
    pub fn plan_reads(&self, id: &str) -> Result<JsValue, JsError> {
        let mut reads: Vec<(u16, u8)> = self.study.plan_reads(id).into_iter().collect();
        reads.sort_unstable();
        to_js(&reads)
    }

    #[wasm_bindgen(js_name = setPlanRead)]
    pub fn set_plan_read(&mut self, id: &str, day: u16, passage: u8, read: bool) -> Result<(), JsError> {
        self.study.set_plan_read(id, day, passage, read).map_err(err)
    }
}

impl Bible {
    fn get(&self, id: &str) -> Result<&Translation, JsError> {
        self.translations.get(id).ok_or_else(|| JsError::new(&format!("{id} isn't loaded")))
    }
}

/// A translation's metadata from its file's header, without decoding the text.
#[wasm_bindgen(js_name = translationInfo)]
pub fn translation_info(bytes: &[u8]) -> Result<JsValue, JsError> {
    let meta = format::read_meta(bytes).map_err(err)?;
    to_js(&TranslationInfo::new(&meta, format::decoded_len(bytes)))
}

/// The bundled translations' ids, in the order they're offered.
#[wasm_bindgen(js_name = bundledTranslations)]
pub fn bundled_translations() -> Vec<String> {
    bundled::IDS.iter().map(|s| s.to_string()).collect()
}

/// The 66 books of the canonical grid.
#[wasm_bindgen]
pub fn books() -> Result<JsValue, JsError> {
    let books: Vec<Book> = BOOKS
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let id = BookId(i as u8);
            Book {
                index: i as u8,
                name: b.name,
                abbrev: b.abbrev,
                chapters: b.chapters,
                first_chapter: id.first_chapter().0,
                new_testament: id >= FIRST_NT_BOOK,
            }
        })
        .collect();
    to_js(&books)
}

/// The best reading of a typed reference ("jn 3:16", "Rom 8", "revalation 21"), or null.
#[wasm_bindgen(js_name = parseReference)]
pub fn parse_reference(input: &str) -> Result<JsValue, JsError> {
    match bible_core::reference::parse(input) {
        Some(r) => to_js(&Reference::from(r)),
        None => Ok(JsValue::NULL),
    }
}

/// Readings of a typed reference, the best first.
#[wasm_bindgen(js_name = suggestReferences)]
pub fn suggest_references(input: &str, limit: u32) -> Result<JsValue, JsError> {
    let refs: Vec<Reference> =
        bible_core::reference::suggest(input, limit as usize).into_iter().map(Into::into).collect();
    to_js(&refs)
}

/// "John 3", "Psalm 23".
#[wasm_bindgen(js_name = chapterLabel)]
pub fn chapter_label(index: u16) -> String {
    chapter_idx(index).display()
}

/// The number of verses a chapter has in the KJV's numbering.
#[wasm_bindgen(js_name = chapterVerses)]
pub fn chapter_verses(index: u16) -> u8 {
    chapter_idx(index).kjv_verses()
}

/// "John 3:16", "John 3:16–18", "John 3:16–4:2".
#[wasm_bindgen(js_name = rangeLabel)]
pub fn range_label(from: JsValue, to: JsValue) -> Result<String, JsError> {
    let (a, b) = (verse_arg(from)?, verse_arg(to)?);
    Ok(bible_core::reference::format_range(a.min(b), a.max(b)))
}

/// The reading plans on offer, in order.
#[wasm_bindgen(js_name = planCatalogue)]
pub fn plan_catalogue() -> Result<JsValue, JsError> {
    let plans: Vec<PlanInfo> = plans::catalogue()
        .iter()
        .map(|p| PlanInfo {
            id: p.id,
            title: p.title,
            meta: p.meta,
            description: p.description,
            icon: match p.icon {
                plans::PlanIcon::Calendar => "calendar",
                plans::PlanIcon::Book => "book",
                plans::PlanIcon::Quote => "quote",
                plans::PlanIcon::History => "history",
                plans::PlanIcon::Flame => "flame",
                plans::PlanIcon::Note => "note",
            },
            days: p.len(),
        })
        .collect();
    to_js(&plans)
}

/// A plan's days, each with its passages.
#[wasm_bindgen(js_name = planDays)]
pub fn plan_days(id: &str) -> Result<JsValue, JsError> {
    let plan = plans::get(id).ok_or_else(|| JsError::new(&format!("no plan {id}")))?;
    let name = |b: BookId| b.name().to_string();
    let days: Vec<PlanDay> = plan
        .days
        .iter()
        .map(|day| PlanDay {
            label: plans::label(day, name),
            passages: day
                .iter()
                .map(|p| Passage {
                    chapter_index: p.chapter.0,
                    label: plans::label(std::slice::from_ref(p), name),
                    verses: p.verses.map(|(a, b)| [a, b]),
                })
                .collect(),
        })
        .collect();
    to_js(&days)
}

fn chapter_idx(index: u16) -> ChapterIdx {
    ChapterIdx(index.min(CHAPTER_COUNT as u16 - 1))
}

fn verse_arg(v: JsValue) -> Result<VerseRef, JsError> {
    serde_wasm_bindgen::from_value::<Verse>(v)?.core()
}

fn color_arg(color: Option<String>) -> Result<Option<HighlightColor>, JsError> {
    color
        .map(|c| {
            HighlightColor::ALL
                .into_iter()
                .find(|h| h.name() == c)
                .ok_or_else(|| JsError::new(&format!("no highlight colour {c}")))
        })
        .transpose()
}

/// A span from the page, its UTF-16 offsets turned into chars of `t`'s verse texts.
fn text_span(t: &Translation, span: JsValue) -> Result<TextSpan, JsError> {
    let s: Span = serde_wasm_bindgen::from_value(span)?;
    let (first, last) = (s.first.core()?, s.last.core()?);
    let chars = |r: VerseRef, at: u32| match t.find(r) {
        Some(v) => chars_from_utf16(v.text, at as usize),
        None => at as usize,
    };
    Ok(TextSpan { first, start: chars(first, s.start), last, end: chars(last, s.end) })
}

fn verses_arg(v: JsValue) -> Result<Vec<VerseRef>, JsError> {
    serde_wasm_bindgen::from_value::<Vec<Verse>>(v)?.into_iter().map(Verse::core).collect()
}

fn err(e: anyhow::Error) -> JsError {
    JsError::new(&format!("{e:#}"))
}

fn to_js(value: &impl Serialize) -> Result<JsValue, JsError> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsError::new(&e.to_string()))
}

fn utf16_ranges(text: &str, ranges: &[Range<usize>]) -> Vec<[u32; 2]> {
    ranges
        .iter()
        .filter_map(|r| {
            let start = text.get(..r.start)?.encode_utf16().count() as u32;
            let len = text.get(r.clone())?.encode_utf16().count() as u32;
            Some([start, start + len])
        })
        .collect()
}
