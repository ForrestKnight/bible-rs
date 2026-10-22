//! The common verse-indexed representation every translation is converted into.
//!
//! A translation is one big UTF-8 string of verse text plus three flat arrays:
//!
//! * `verses` — per verse: number, text range, and ranges into `marks` and `spans`;
//! * `marks` — structural breaks inside or before a verse (paragraph, poetry line, heading, …);
//! * `spans` — character styles within a verse (words of Christ, translator additions, LORD, …).
//!
//! Offsets inside a verse are `u16` byte offsets relative to the verse start. `chapter_start`
//! maps each of the 1,189 canonical chapters to its slice of `verses`. The layout is designed so
//! that loading is a handful of memcpys (see [`crate::format`]) and rendering a chapter never
//! allocates per verse.

use std::ops::Range;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::books::{BOOK_COUNT, BookId, CHAPTER_COUNT, ChapterIdx, VerseRef};
use crate::search::SearchIndex;

/// Where a translation came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Origin {
    /// Compiled into the application binary (public domain only).
    #[default]
    Bundled,
    /// Imported by the user from a local file into their data directory.
    Imported { format: String, file_name: String },
    /// Fetched on demand from an online API with the user's own key.
    Online { provider: String, remote_id: String },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BookTitle {
    /// Short running-head name as the translation spells it ("John").
    pub short: String,
    /// Full title ("The Gospel According to John"), possibly empty.
    pub long: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranslationMeta {
    /// Stable lower-case identifier ("kjv"); used in settings and file names.
    pub id: String,
    /// Display abbreviation ("KJV").
    pub abbrev: String,
    /// Full name ("King James Version").
    pub name: String,
    /// BCP-47 language tag.
    pub language: String,
    /// Licence summary ("Public Domain", "© Crossway — licensed via ESV API", …).
    pub license: String,
    /// Attribution / copyright notice that must accompany the text.
    pub copyright: String,
    /// Provenance: where the text was obtained.
    pub source: String,
    #[serde(default)]
    pub origin: Origin,
    /// Book titles indexed by [`BookId`]; empty strings where the translation has no title.
    #[serde(default)]
    pub books: Vec<BookTitle>,
    #[serde(default)]
    pub red_letters: bool,
    #[serde(default)]
    pub paragraphs: bool,
    #[serde(default)]
    pub headings: bool,
}

impl TranslationMeta {
    pub fn book_short(&self, book: BookId) -> &str {
        match self.books.get(book.index()) {
            Some(t) if !t.short.is_empty() => &t.short,
            _ => book.name(),
        }
    }

    /// Book name for a chapter or verse reference ("Psalm 23", not "Psalms 23").
    pub fn ref_name(&self, book: BookId) -> &str {
        let short = self.book_short(book);
        if book == BookId::PSALMS && short.starts_with("Psalm") { "Psalm" } else { short }
    }
}

/// Structural marks. Stored as `u8` in the binary format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MarkKind {
    /// Normal prose paragraph (first line indented).
    Paragraph = 0,
    /// Paragraph without first-line indent (`\m`, `\pmo`, `\mi`).
    ParagraphFlush = 1,
    /// Poetry line; `level` 1–4.
    Poetry = 2,
    /// Right-aligned poetry line (`\qr`).
    PoetryRight = 3,
    /// Centered line (`\qc`, `\pc`).
    Centered = 4,
    /// List item; `level` 1–4.
    List = 5,
    /// Stanza / blank-line break.
    Blank = 6,
    /// Section heading; `level` 0 = major (`\ms`), 1 = `\s1`, 2 = `\s2`, 3 = acrostic letter (`\qa`).
    Heading = 7,
    /// Psalm superscription (`\d`).
    PsalmTitle = 8,
    /// Speaker label (`\sp`).
    Speaker = 9,
}

impl MarkKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        use MarkKind::*;
        Some(match v {
            0 => Paragraph,
            1 => ParagraphFlush,
            2 => Poetry,
            3 => PoetryRight,
            4 => Centered,
            5 => List,
            6 => Blank,
            7 => Heading,
            8 => PsalmTitle,
            9 => Speaker,
            _ => return None,
        })
    }

    /// Marks that carry their own text (stored in `extra`).
    pub fn has_text(self) -> bool {
        matches!(self, MarkKind::Heading | MarkKind::PsalmTitle | MarkKind::Speaker)
    }

    /// Marks that start a new line of verse text.
    pub fn breaks_line(self) -> bool {
        !self.has_text()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mark {
    /// Byte offset within the verse text where the mark applies.
    pub offset: u16,
    pub kind: MarkKind,
    pub level: u8,
    /// Range into the translation's `extra` string (heading text).
    pub payload: (u32, u32),
}

/// Character styles. Stored as `u8`; several may overlap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Style {
    /// Words of Christ (`\wj`), rendered in red when red letters are on.
    WordsOfJesus = 0,
    /// Words supplied by the translators (`\add`), traditionally italic in the KJV.
    Added = 1,
    /// The divine name (`\nd`) — "LORD", rendered in small capitals.
    DivineName = 2,
    /// "Selah" (`\qs`).
    Selah = 3,
    Italic = 4,
    Bold = 5,
    SmallCaps = 6,
}

impl Style {
    pub const ALL: [Style; 7] = [
        Style::WordsOfJesus,
        Style::Added,
        Style::DivineName,
        Style::Selah,
        Style::Italic,
        Style::Bold,
        Style::SmallCaps,
    ];

    pub fn from_u8(v: u8) -> Option<Self> {
        Style::ALL.get(v as usize).copied()
    }

    pub fn bit(self) -> u8 {
        1 << self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: u16,
    pub end: u16,
    pub style: Style,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Verse {
    pub number: u8,
    /// Last verse number for bridged verses ("3-4"); equal to `number` otherwise.
    pub number_end: u8,
    pub(crate) text_start: u32,
    pub(crate) text_len: u16,
    pub(crate) marks_start: u32,
    pub(crate) marks_len: u16,
    pub(crate) spans_start: u32,
    pub(crate) spans_len: u16,
}

impl Verse {
    pub fn covers(&self, n: u8) -> bool {
        n >= self.number && n <= self.number_end
    }
}

pub struct Translation {
    pub meta: TranslationMeta,
    pub(crate) text: String,
    pub(crate) extra: String,
    pub(crate) verses: Vec<Verse>,
    /// `CHAPTER_COUNT + 1` entries; chapter `c` owns `verses[chapter_start[c]..chapter_start[c+1]]`.
    pub(crate) chapter_start: Vec<u32>,
    pub(crate) marks: Vec<Mark>,
    pub(crate) spans: Vec<Span>,
    pub(crate) search: OnceLock<SearchIndex>,
}

impl std::fmt::Debug for Translation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Translation")
            .field("id", &self.meta.id)
            .field("abbrev", &self.meta.abbrev)
            .field("verses", &self.verses.len())
            .finish_non_exhaustive()
    }
}

/// A borrowed view of one verse.
#[derive(Clone, Copy)]
pub struct VerseView<'a> {
    pub chapter: ChapterIdx,
    pub index: u32,
    pub number: u8,
    pub number_end: u8,
    pub text: &'a str,
    pub marks: &'a [Mark],
    pub spans: &'a [Span],
}

impl VerseView<'_> {
    pub fn reference(&self) -> VerseRef {
        VerseRef::new(self.chapter.book(), self.chapter.chapter(), self.number)
    }

    /// Verse text with line-breaking marks turned into spaces (they are stored without one).
    pub fn plain_text(&self) -> String {
        let mut out = String::with_capacity(self.text.len() + 8);
        let mut last = 0usize;
        for m in self.marks.iter().filter(|m| m.kind.breaks_line() && m.offset > 0) {
            let at = m.offset as usize;
            if at > last && at < self.text.len() {
                out.push_str(&self.text[last..at]);
                if !out.ends_with(' ') {
                    out.push(' ');
                }
                last = at;
            }
        }
        out.push_str(&self.text[last..]);
        out
    }

    /// Chars `range` of the verse's text, a line break inside it made a space as in
    /// [`plain_text`](Self::plain_text).
    pub fn plain_slice(&self, range: std::ops::Range<usize>) -> String {
        let breaks: Vec<usize> = self
            .marks
            .iter()
            .filter(|m| m.kind.breaks_line() && m.offset > 0)
            .filter_map(|m| self.text.get(..m.offset as usize).map(|t| t.chars().count()))
            .collect();
        let mut out = String::new();
        for (i, c) in
            self.text.chars().enumerate().skip(range.start).take(range.end.saturating_sub(range.start))
        {
            if i > range.start && breaks.contains(&i) && !out.ends_with(' ') && c != ' ' {
                out.push(' ');
            }
            out.push(c);
        }
        out
    }
}

impl Translation {
    pub fn id(&self) -> &str {
        &self.meta.id
    }

    pub fn verse_count(&self) -> usize {
        self.verses.len()
    }

    pub fn has_chapter(&self, c: ChapterIdx) -> bool {
        !self.chapter_range(c).is_empty()
    }

    pub fn has_book(&self, book: BookId) -> bool {
        let first = book.first_chapter();
        (0..book.chapters() as u16).any(|i| self.has_chapter(ChapterIdx(first.0 + i)))
    }

    pub(crate) fn chapter_range(&self, c: ChapterIdx) -> Range<usize> {
        let i = c.index();
        if i + 1 >= self.chapter_start.len() {
            return 0..0;
        }
        self.chapter_start[i] as usize..self.chapter_start[i + 1] as usize
    }

    /// Global verse index range (into the translation's verse array) for a chapter.
    pub fn chapter_verse_range(&self, c: ChapterIdx) -> Range<u32> {
        let r = self.chapter_range(c);
        r.start as u32..r.end as u32
    }

    /// Global verse index range for a whole book.
    pub fn book_verse_range(&self, book: BookId) -> Range<u32> {
        let first = book.first_chapter().index();
        let last = first + book.chapters() as usize;
        self.chapter_start[first]..self.chapter_start[last]
    }

    pub fn verses_in(&self, c: ChapterIdx) -> impl ExactSizeIterator<Item = VerseView<'_>> + '_ {
        self.chapter_range(c).map(move |i| self.view(c, i as u32))
    }

    /// Number of the last verse in a chapter (0 if the chapter is missing).
    pub fn last_verse(&self, c: ChapterIdx) -> u8 {
        let r = self.chapter_range(c);
        if r.is_empty() { 0 } else { self.verses[r.end - 1].number_end }
    }

    pub fn verse_view(&self, index: u32) -> Option<VerseView<'_>> {
        if index as usize >= self.verses.len() {
            return None;
        }
        Some(self.view(self.chapter_of(index), index))
    }

    fn view(&self, chapter: ChapterIdx, index: u32) -> VerseView<'_> {
        let v = &self.verses[index as usize];
        let ts = v.text_start as usize;
        let ms = v.marks_start as usize;
        let ss = v.spans_start as usize;
        VerseView {
            chapter,
            index,
            number: v.number,
            number_end: v.number_end,
            text: &self.text[ts..ts + v.text_len as usize],
            marks: &self.marks[ms..ms + v.marks_len as usize],
            spans: &self.spans[ss..ss + v.spans_len as usize],
        }
    }

    /// Chapter that a global verse index belongs to.
    pub fn chapter_of(&self, index: u32) -> ChapterIdx {
        let next = self.chapter_start.partition_point(|&s| s <= index);
        ChapterIdx((next.max(1) - 1).min(CHAPTER_COUNT - 1) as u16)
    }

    pub fn find(&self, r: VerseRef) -> Option<VerseView<'_>> {
        let c = ChapterIdx::new(r.book, r.chapter)?;
        let range = self.chapter_range(c);
        let slice = &self.verses[range.clone()];
        let pos = slice.iter().position(|v| v.covers(r.verse))?;
        Some(self.view(c, (range.start + pos) as u32))
    }

    pub fn mark_text(&self, m: &Mark) -> &str {
        let (s, l) = m.payload;
        self.extra.get(s as usize..(s + l) as usize).unwrap_or("")
    }

    pub fn total_text_bytes(&self) -> usize {
        self.text.len()
    }

    /// The lazily built full-text search index.
    pub fn search_index(&self) -> &SearchIndex {
        self.search.get_or_init(|| SearchIndex::build(self))
    }

    pub fn is_search_ready(&self) -> bool {
        self.search.get().is_some()
    }
}

/// Incrementally converts a stream of book/chapter/verse/text events into a [`Translation`].
///
/// Used by every importer (USFM, OSIS, Zefania, SWORD, online APIs). Whitespace is collapsed,
/// structural marks that arrive between verses attach to the following verse, and character
/// styles that cross verse boundaries are split per verse.
pub struct TranslationBuilder {
    meta: TranslationMeta,
    text: String,
    extra: String,
    verses: Vec<(ChapterIdx, Verse)>,
    marks: Vec<Mark>,
    spans: Vec<Span>,

    book: Option<BookId>,
    chapter: Option<ChapterIdx>,
    in_verse: bool,
    verse_start: usize,
    cur_marks: Vec<Mark>,
    cur_spans: Vec<Span>,
    pending_space: bool,
    pending_marks: Vec<(MarkKind, u8, (u32, u32))>,
    open_styles: Vec<(Style, Option<u16>)>,
    seen: std::collections::HashSet<(u16, u8)>,
    pub warnings: Vec<String>,
    /// Books in the source outside the 66-book canon, by the source's own id ("TOB", "Sir").
    pub skipped_books: Vec<String>,
    /// Called as each book begins (import progress).
    pub on_book: Option<Box<dyn FnMut(BookId) + Send>>,
}

impl TranslationBuilder {
    pub fn new(meta: TranslationMeta) -> Self {
        let mut meta = meta;
        if meta.books.len() != BOOK_COUNT {
            meta.books = vec![BookTitle::default(); BOOK_COUNT];
        }
        Self {
            meta,
            text: String::with_capacity(4 << 20),
            extra: String::new(),
            verses: Vec::with_capacity(31_200),
            marks: Vec::new(),
            spans: Vec::new(),
            book: None,
            chapter: None,
            in_verse: false,
            verse_start: 0,
            cur_marks: Vec::new(),
            cur_spans: Vec::new(),
            pending_space: false,
            pending_marks: Vec::new(),
            open_styles: Vec::new(),
            seen: Default::default(),
            warnings: Vec::new(),
            skipped_books: Vec::new(),
            on_book: None,
        }
    }

    /// Notes a book that isn't part of the canon and so isn't imported.
    pub fn skip_book(&mut self, id: &str) {
        let id = id.trim();
        if !id.is_empty() && !self.skipped_books.iter().any(|b| b == id) {
            self.skipped_books.push(id.to_string());
        }
    }

    pub fn meta_mut(&mut self) -> &mut TranslationMeta {
        &mut self.meta
    }

    pub fn current_book(&self) -> Option<BookId> {
        self.book
    }

    pub fn current_chapter(&self) -> Option<ChapterIdx> {
        self.chapter
    }

    pub fn in_verse(&self) -> bool {
        self.in_verse
    }

    pub fn begin_book(&mut self, book: BookId) {
        if let Some(f) = &mut self.on_book {
            f(book);
        }
        self.end_verse();
        self.pending_marks.clear();
        self.book = Some(book);
        self.chapter = None;
    }

    pub fn set_book_title(&mut self, book: BookId, short: Option<&str>, long: Option<&str>) {
        let t = &mut self.meta.books[book.index()];
        if let Some(s) = short.map(str::trim).filter(|s| !s.is_empty()) {
            t.short = collapse_ws(s);
        }
        if let Some(l) = long.map(str::trim).filter(|s| !s.is_empty()) {
            t.long = collapse_ws(l);
        }
    }

    /// Starts a chapter of the current book. Returns `false` (and ignores the chapter's text)
    /// if it falls outside the canonical grid.
    pub fn begin_chapter(&mut self, chapter: u32) -> bool {
        self.end_verse();
        // Breaks left over from the previous chapter are meaningless; headings may precede `\c`.
        self.pending_marks.retain(|(k, _, _)| k.has_text());
        self.chapter = self.book.and_then(|b| u8::try_from(chapter).ok().and_then(|c| ChapterIdx::new(b, c)));
        if self.chapter.is_none()
            && let Some(b) = self.book
        {
            self.warnings
                .push(format!("{} {chapter}: chapter outside the canonical grid, skipped", b.name()));
        }
        self.chapter.is_some()
    }

    /// Starts a verse (or a bridged verse range).
    pub fn begin_verse(&mut self, number: u32, number_end: u32) {
        self.end_verse();
        let Some(c) = self.chapter else { return };
        let (Ok(n), Ok(e)) = (u8::try_from(number), u8::try_from(number_end.max(number))) else {
            self.warnings.push(format!("{}: verse {number} out of range", c.display()));
            return;
        };
        if n == 0 {
            return;
        }
        if !self.seen.insert((c.0, n)) {
            self.warnings.push(format!("{}:{n} appears twice; later copy skipped", c.display()));
            return;
        }
        self.in_verse = true;
        self.verse_start = self.text.len();
        self.pending_space = false;
        for (kind, level, payload) in self.pending_marks.drain(..) {
            self.cur_marks.push(Mark { offset: 0, kind, level, payload });
        }
        self.verses.push((
            c,
            Verse { number: n, number_end: e, text_start: self.verse_start as u32, ..Default::default() },
        ));
    }

    fn verse_len(&self) -> usize {
        self.text.len() - self.verse_start
    }

    /// Appends verse text. Text outside a verse is ignored.
    pub fn text(&mut self, s: &str) {
        if !self.in_verse {
            return;
        }
        for ch in s.chars() {
            if ch == '¶' {
                // The KJV's pilcrows are paragraph indicators, not text.
                if self.verse_len() == 0
                    && !self.pending_marks.iter().any(|(k, _, _)| k.breaks_line())
                    && !self.cur_marks.iter().any(|m| m.kind.breaks_line())
                {
                    self.pending_marks.push((MarkKind::Paragraph, 0, (0, 0)));
                }
                continue;
            }
            if ch.is_whitespace() {
                // A pending break already separates words.
                if self.verse_len() > 0 && self.pending_marks.is_empty() {
                    self.pending_space = true;
                }
                continue;
            }
            if self.pending_space {
                self.pending_space = false;
                if self.verse_len() > 0 {
                    self.text.push(' ');
                }
            }
            if !self.pending_marks.is_empty() {
                let offset = self.verse_len() as u16;
                for (kind, level, payload) in self.pending_marks.drain(..) {
                    self.cur_marks.push(Mark { offset, kind, level, payload });
                }
            }
            let offset = self.verse_len() as u16;
            for (_, start) in self.open_styles.iter_mut().filter(|(_, s)| s.is_none()) {
                *start = Some(offset);
            }
            self.text.push(ch);
        }
    }

    /// Structural break (paragraph, poetry line, stanza break, …) at the current position.
    pub fn mark(&mut self, kind: MarkKind, level: u8) {
        self.pending_space = false;
        // Collapse consecutive breaks: the last one wins (e.g. `\p` immediately followed by `\q1`).
        if kind != MarkKind::Blank {
            self.pending_marks.retain(|(k, _, _)| !k.breaks_line() || *k == MarkKind::Blank);
        }
        self.pending_marks.push((kind, level, (0, 0)));
    }

    /// A heading-like mark with its own text, placed before the following verse text.
    pub fn heading(&mut self, kind: MarkKind, level: u8, title: &str) {
        let title = collapse_ws(title.trim());
        if title.is_empty() {
            return;
        }
        let start = self.extra.len() as u32;
        self.extra.push_str(&title);
        self.pending_space = false;
        self.pending_marks.push((kind, level, (start, title.len() as u32)));
    }

    pub fn push_style(&mut self, style: Style) {
        self.open_styles.push((style, None));
    }

    pub fn pop_style(&mut self, style: Style) {
        if let Some(i) = self.open_styles.iter().rposition(|(s, _)| *s == style) {
            let (_, start) = self.open_styles.remove(i);
            if let (Some(start), true) = (start, self.in_verse) {
                let end = self.verse_len() as u16;
                if end > start {
                    self.cur_spans.push(Span { start, end, style });
                }
            }
        }
    }

    /// Closes every open character style (end of paragraph in some formats).
    pub fn clear_styles(&mut self) {
        while let Some((style, _)) = self.open_styles.last().copied() {
            self.pop_style(style);
        }
    }

    /// Ends the current verse explicitly (OSIS `eID` milestones, closing `</VERS>`).
    pub fn end_current_verse(&mut self) {
        self.end_verse();
    }

    fn end_verse(&mut self) {
        if !self.in_verse {
            return;
        }
        self.in_verse = false;
        self.pending_space = false;
        let len = self.verse_len();
        // Styles still open continue into the next verse; close this verse's part.
        for (style, start) in self.open_styles.iter_mut() {
            if let Some(s) = start.take()
                && (len as u16) > s
            {
                self.cur_spans.push(Span { start: s, end: len as u16, style: *style });
            }
        }
        let (_, v) = self.verses.last_mut().expect("verse in progress");
        if len > u16::MAX as usize {
            self.warnings.push("verse longer than 64 KiB truncated".into());
        }
        v.text_len = len.min(u16::MAX as usize) as u16;
        v.marks_start = self.marks.len() as u32;
        v.marks_len = self.cur_marks.len() as u16;
        v.spans_start = self.spans.len() as u32;
        self.marks.append(&mut self.cur_marks);
        self.cur_spans.sort_by_key(|s| (s.style as u8, s.start, s.end));
        // Merge same-style spans that touch or are separated only by whitespace (sources often
        // close `\wj` around a footnote and reopen it right after).
        let verse_text = &self.text[self.verse_start..];
        let mut merged: Vec<Span> = Vec::with_capacity(self.cur_spans.len());
        for s in self.cur_spans.drain(..) {
            if let Some(last) = merged.last_mut()
                && last.style == s.style
                && s.start >= last.start
                && verse_text
                    .get(last.end.min(s.start) as usize..s.start.max(last.end) as usize)
                    .is_some_and(|gap| gap.chars().all(char::is_whitespace))
            {
                last.end = last.end.max(s.end);
                continue;
            }
            merged.push(s);
        }
        merged.sort_by_key(|s| (s.start, s.end));
        v.spans_len = merged.len() as u16;
        self.spans.extend(merged);
    }

    pub fn verse_total(&self) -> usize {
        self.verses.len()
    }

    pub fn finish(mut self) -> Translation {
        self.end_verse();
        // Drop verses that never received text (e.g. omitted verses with only a footnote).
        let mut verses = std::mem::take(&mut self.verses);
        verses.retain(|(_, v)| v.text_len > 0);
        verses.sort_by_key(|(c, v)| (c.0, v.number));
        let mut chapter_start = vec![0u32; CHAPTER_COUNT + 1];
        let mut vi = 0usize;
        for (c, slot) in chapter_start.iter_mut().enumerate().take(CHAPTER_COUNT) {
            while vi < verses.len() && (verses[vi].0.0 as usize) < c {
                vi += 1;
            }
            *slot = vi as u32;
        }
        chapter_start[CHAPTER_COUNT] = verses.len() as u32;
        let has_red = self.spans.iter().any(|s| s.style == Style::WordsOfJesus);
        let has_par = self.marks.iter().filter(|m| m.kind == MarkKind::Paragraph).count() > 1_300;
        let has_head = self.marks.iter().any(|m| m.kind == MarkKind::Heading);
        let mut meta = self.meta;
        meta.red_letters = has_red;
        meta.paragraphs = has_par;
        meta.headings = has_head;
        for (i, t) in meta.books.iter_mut().enumerate() {
            if t.short.is_empty() {
                t.short = crate::books::BOOKS[i].name.to_string();
            }
        }
        Translation {
            meta,
            text: self.text,
            extra: self.extra,
            verses: verses.into_iter().map(|(_, v)| v).collect(),
            chapter_start,
            marks: self.marks,
            spans: self.spans,
            search: OnceLock::new(),
        }
    }
}

/// Words printed in capitals (LORD where a text has no divine-name markup, GOD, JESUS OF
/// NAZARETH…): two or more letters, all upper case. Readers set them in small capitals, as
/// printed Bibles do.
pub fn caps_words(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut word: Option<(usize, bool, usize)> = None;
    for (i, c) in text.char_indices().chain(std::iter::once((text.len(), ' '))) {
        if c.is_alphabetic() {
            let (start, upper, n) = word.unwrap_or((i, true, 0));
            word = Some((start, upper && c.is_uppercase(), n + 1));
        } else if let Some((start, upper, n)) = word.take()
            && upper
            && n >= 2
        {
            out.push(start..i);
        }
    }
    out
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for w in s.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(w);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> TranslationMeta {
        TranslationMeta { id: "t".into(), abbrev: "T".into(), name: "Test".into(), ..Default::default() }
    }

    #[test]
    fn builder_collapses_whitespace_and_attaches_marks() {
        let mut b = TranslationBuilder::new(meta());
        b.begin_book(BookId::PSALMS);
        b.begin_chapter(1);
        b.heading(MarkKind::PsalmTitle, 0, "  A  Psalm ");
        b.mark(MarkKind::Poetry, 1);
        b.begin_verse(1, 1);
        b.text("  Blessed is the man  ");
        b.mark(MarkKind::Poetry, 2);
        b.text(" who walks not ");
        b.begin_verse(2, 2);
        b.push_style(Style::DivineName);
        b.text("LORD");
        b.pop_style(Style::DivineName);
        b.mark(MarkKind::Blank, 0);
        let t = b.finish();
        let c = ChapterIdx::new(BookId::PSALMS, 1).unwrap();
        let vs: Vec<_> = t.verses_in(c).collect();
        assert_eq!(vs.len(), 2);
        assert_eq!(vs[0].text, "Blessed is the manwho walks not");
        assert_eq!(vs[0].plain_text(), "Blessed is the man who walks not");
        assert_eq!(vs[0].marks.len(), 3);
        assert_eq!(vs[0].marks[0].kind, MarkKind::PsalmTitle);
        assert_eq!(t.mark_text(&vs[0].marks[0]), "A Psalm");
        assert_eq!(vs[0].marks[2], Mark { offset: 18, kind: MarkKind::Poetry, level: 2, payload: (0, 0) });
        assert_eq!(vs[1].spans, &[Span { start: 0, end: 4, style: Style::DivineName }]);
        // Trailing stanza break at the end of the chapter is dropped.
        assert!(vs[1].marks.is_empty());
    }

    #[test]
    fn styles_split_across_verses() {
        let mut b = TranslationBuilder::new(meta());
        b.begin_book(BookId(42));
        b.begin_chapter(3);
        b.begin_verse(1, 1);
        b.text("He said, ");
        b.push_style(Style::WordsOfJesus);
        b.text("“Truly");
        b.begin_verse(2, 2);
        b.text("I say.”");
        b.pop_style(Style::WordsOfJesus);
        b.text(" And");
        let t = b.finish();
        let c = ChapterIdx::new(BookId(42), 3).unwrap();
        let vs: Vec<_> = t.verses_in(c).collect();
        assert_eq!(vs[0].spans, &[Span { start: 9, end: 17, style: Style::WordsOfJesus }]);
        assert_eq!(&vs[0].text[9..17], "“Truly");
        assert_eq!(vs[1].spans, &[Span { start: 0, end: 9, style: Style::WordsOfJesus }]);
        assert_eq!(vs[1].text, "I say.” And");
    }

    #[test]
    fn adjacent_spans_merge() {
        let mut b = TranslationBuilder::new(meta());
        b.begin_book(BookId(42));
        b.begin_chapter(3);
        b.begin_verse(16, 16);
        b.push_style(Style::WordsOfJesus);
        b.text("only born");
        b.pop_style(Style::WordsOfJesus);
        b.text(" ");
        b.push_style(Style::WordsOfJesus);
        b.text("Son");
        b.pop_style(Style::WordsOfJesus);
        b.text(" end");
        let t = b.finish();
        let v = t.verses_in(ChapterIdx::new(BookId(42), 3).unwrap()).next().unwrap();
        assert_eq!(v.spans, &[Span { start: 0, end: 13, style: Style::WordsOfJesus }]);
    }

    #[test]
    fn caps_words_finds_words_in_capitals() {
        let text = "the LORD GOD of Élan, JÉSUS; A man said I AM.";
        let words: Vec<&str> = caps_words(text).into_iter().map(|r| &text[r]).collect();
        assert_eq!(words, ["LORD", "GOD", "JÉSUS", "AM"]);
        assert!(caps_words("Lord God").is_empty());
    }

    #[test]
    fn find_handles_bridged_verses_and_missing_chapters() {
        let mut b = TranslationBuilder::new(meta());
        b.begin_book(BookId(0));
        b.begin_chapter(1);
        b.begin_verse(1, 1);
        b.text("a");
        b.begin_verse(2, 3);
        b.text("b");
        let t = b.finish();
        assert_eq!(t.find(VerseRef::new(BookId(0), 1, 3)).unwrap().text, "b");
        assert!(t.find(VerseRef::new(BookId(0), 2, 1)).is_none());
        assert!(!t.has_chapter(ChapterIdx(1)));
        assert_eq!(t.chapter_of(1), ChapterIdx(0));
        assert_eq!(t.last_verse(ChapterIdx(0)), 3);
    }
}
