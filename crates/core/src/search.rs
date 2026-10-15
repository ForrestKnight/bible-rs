//! Full-text search.
//!
//! Each translation lazily builds a normalized corpus — lower-case, diacritics folded,
//! apostrophes joined ("Father’s" → `fathers`), everything else a single space — laid out as
//! `" w1 w2 … "` per verse with a parallel array of verse start offsets. A term is a SIMD
//! substring scan (`memchr::memmem`) for `" term"` (word prefix) or `" term "` (whole word);
//! phrases are just longer needles. Because every verse is wrapped in spaces and the needle never
//! contains two spaces in a row, a match can never straddle two verses. Mapping a hit back to its
//! verse is a binary search, after which the scan skips straight to the next verse.
//!
//! On the KJV (4.3 MB) a single-term query takes well under a millisecond; see `benches/`.

use std::ops::Range;
use std::time::{Duration, Instant};

use memchr::memmem;

use crate::books::{BookId, FIRST_NT_BOOK};
use crate::model::Translation;

pub struct SearchIndex {
    corpus: Vec<u8>,
    /// `starts[i]` = offset of verse `i`'s leading space; one extra entry at the end.
    starts: Vec<u32>,
}

impl SearchIndex {
    pub fn build(t: &Translation) -> Self {
        let mut corpus = Vec::with_capacity(t.text.len() + t.verses.len() * 2);
        let mut starts = Vec::with_capacity(t.verses.len() + 1);
        for i in 0..t.verses.len() as u32 {
            let v = t.verse_view(i).expect("in range");
            starts.push(corpus.len() as u32);
            corpus.push(b' ');
            let mut last = 0usize;
            for m in v.marks.iter().filter(|m| m.kind.breaks_line()) {
                let at = (m.offset as usize).min(v.text.len());
                if at > last {
                    normalize_into(&v.text[last..at], &mut corpus);
                    push_space(&mut corpus);
                    last = at;
                }
            }
            normalize_into(&v.text[last..], &mut corpus);
            push_space(&mut corpus);
        }
        starts.push(corpus.len() as u32);
        corpus.shrink_to_fit();
        Self { corpus, starts }
    }

    pub fn corpus_len(&self) -> usize {
        self.corpus.len()
    }

    fn verse_at(&self, pos: usize) -> u32 {
        (self.starts.partition_point(|&s| s as usize <= pos) - 1) as u32
    }

    /// Verses (global indices, canonical order) matching every term of `query` within `scope`.
    pub fn search(&self, t: &Translation, query: &Query, scope: Scope) -> Vec<u32> {
        let verse_range = match scope {
            Scope::All => 0..t.verses.len() as u32,
            Scope::OldTestament => 0..t.book_verse_range(FIRST_NT_BOOK).start,
            Scope::NewTestament => t.book_verse_range(FIRST_NT_BOOK).start..t.verses.len() as u32,
            Scope::Book(b) => t.book_verse_range(b),
        };
        if query.terms.is_empty() || verse_range.is_empty() {
            return Vec::new();
        }
        let mut needles: Vec<Vec<u8>> = query.terms.iter().map(Term::needle).collect();
        // Longer needles are rarer: scanning for them first keeps the candidate set small.
        needles.sort_by_key(|n| std::cmp::Reverse(n.len()));
        let mut hits = self.scan(&needles[0], verse_range.clone());
        for needle in &needles[1..] {
            if hits.is_empty() {
                break;
            }
            if hits.len() > 4_000 {
                let other = self.scan(needle, verse_range.clone());
                hits = intersect(&hits, &other);
            } else {
                let finder = memmem::Finder::new(needle);
                hits.retain(|&v| finder.find(self.verse_bytes(v)).is_some());
            }
        }
        for term in &query.excluded {
            let finder = memmem::Finder::new(&term.needle()).into_owned();
            hits.retain(|&v| finder.find(self.verse_bytes(v)).is_none());
        }
        hits
    }

    fn verse_bytes(&self, v: u32) -> &[u8] {
        &self.corpus[self.starts[v as usize] as usize..self.starts[v as usize + 1] as usize]
    }

    fn scan(&self, needle: &[u8], verses: Range<u32>) -> Vec<u32> {
        let finder = memmem::Finder::new(needle);
        let end = self.starts[verses.end as usize] as usize;
        let mut cursor = self.starts[verses.start as usize] as usize;
        let mut out = Vec::new();
        while cursor < end {
            let Some(rel) = finder.find(&self.corpus[cursor..end]) else { break };
            let v = self.verse_at(cursor + rel);
            out.push(v);
            cursor = self.starts[v as usize + 1] as usize;
        }
        out
    }
}

fn intersect(a: &[u32], b: &[u32]) -> Vec<u32> {
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::with_capacity(a.len().min(b.len()));
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                out.push(a[i]);
                i += 1;
                j += 1;
            }
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scope {
    #[default]
    All,
    OldTestament,
    NewTestament,
    Book(BookId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Term {
    /// Normalized words.
    pub words: Vec<String>,
    /// The last word must match a whole word (otherwise it is a prefix).
    pub whole_last: bool,
    pub phrase: bool,
}

impl Term {
    fn needle(&self) -> Vec<u8> {
        let mut n = Vec::with_capacity(self.words.iter().map(|w| w.len() + 1).sum::<usize>() + 1);
        for w in &self.words {
            n.push(b' ');
            n.extend_from_slice(w.as_bytes());
        }
        if self.whole_last {
            n.push(b' ');
        }
        n
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Query {
    pub terms: Vec<Term>,
    pub excluded: Vec<Term>,
}

impl Query {
    /// Parses user input: bare words (all must occur, as word prefixes unless `whole_words`),
    /// `"quoted phrases"` (words in order; the last may be a prefix unless `whole_words`, so
    /// "living water" also finds "living waters"), and `-excluded`.
    pub fn parse(input: &str, whole_words: bool) -> Self {
        let mut q = Query::default();
        let mut rest = input.trim_start();
        while !rest.is_empty() {
            let (negated, body) = match rest.strip_prefix('-') {
                Some(b) if !b.starts_with(char::is_whitespace) && !b.is_empty() => (true, b),
                _ => (false, rest),
            };
            let (term, next) = if let Some(b) = body.strip_prefix(['"', '“', '”']) {
                match b.find(['"', '“', '”']) {
                    Some(end) => {
                        let close_len = b[end..].chars().next().map_or(1, char::len_utf8);
                        (make_term(&b[..end], true, whole_words), &b[end + close_len..])
                    }
                    None => (make_term(b, true, whole_words), ""),
                }
            } else {
                let end = body.find(char::is_whitespace).unwrap_or(body.len());
                (make_term(&body[..end], false, whole_words || negated), &body[end..])
            };
            if let Some(term) = term {
                if term.phrase || term.words.len() == 1 {
                    if negated { q.excluded.push(term) } else { q.terms.push(term) }
                } else {
                    // An unquoted "word-word" normalizes to several words: keep them adjacent.
                    let term = Term { phrase: true, ..term };
                    if negated { q.excluded.push(term) } else { q.terms.push(term) }
                }
            }
            rest = next.trim_start();
        }
        q
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// Byte ranges in `text` (a verse's plain text) that match any term, for highlighting.
    pub fn match_ranges(&self, text: &str) -> Vec<Range<usize>> {
        let words = words_with_ranges(text);
        let mut out: Vec<Range<usize>> = Vec::new();
        for term in &self.terms {
            let n = term.words.len();
            if n == 0 || n > words.len() {
                continue;
            }
            for i in 0..=words.len() - n {
                let ok = term.words.iter().enumerate().all(|(k, w)| {
                    let (ref norm, _) = words[i + k];
                    if k + 1 == n && !term.whole_last { norm.starts_with(w.as_str()) } else { norm == w }
                });
                if ok {
                    out.push(words[i].1.start..words[i + n - 1].1.end);
                }
            }
        }
        out.sort_by_key(|r| r.start);
        let mut merged: Vec<Range<usize>> = Vec::with_capacity(out.len());
        for r in out {
            match merged.last_mut() {
                Some(last) if r.start <= last.end => last.end = last.end.max(r.end),
                _ => merged.push(r),
            }
        }
        merged
    }
}

fn make_term(raw: &str, phrase: bool, whole_last: bool) -> Option<Term> {
    let mut buf = Vec::new();
    normalize_into(raw, &mut buf);
    let s = String::from_utf8(buf).ok()?;
    let words: Vec<String> = s.split(' ').filter(|w| !w.is_empty()).map(str::to_string).collect();
    (!words.is_empty()).then_some(Term { words, whole_last, phrase })
}

/// Splits text into (normalized word, byte range) pairs using the same rules as the corpus.
pub fn words_with_ranges(text: &str) -> Vec<(String, Range<usize>)> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let mut cur = String::new();
    let mut iter = text.char_indices().peekable();
    while let Some((i, c)) = iter.next() {
        if c.is_alphanumeric() {
            if start.is_none() {
                start = Some(i);
            }
            fold_char(c, &mut cur);
        } else if is_apostrophe(c) && start.is_some() && iter.peek().is_some_and(|(_, n)| n.is_alphanumeric())
        {
            // joined: "Father’s" → "fathers"
        } else if let Some(s) = start.take() {
            out.push((std::mem::take(&mut cur), s..i));
        }
    }
    if let Some(s) = start {
        out.push((cur, s..text.len()));
    }
    out
}

fn is_apostrophe(c: char) -> bool {
    matches!(c, '\'' | '’' | 'ʼ' | '‘')
}

fn push_space(out: &mut Vec<u8>) {
    if out.last() != Some(&b' ') {
        out.push(b' ');
    }
}

/// Appends the normalized form of `text` (see module docs). Never emits two spaces in a row.
pub fn normalize_into(text: &str, out: &mut Vec<u8>) {
    let mut tmp = String::new();
    let mut chars = text.chars().peekable();
    let mut prev_alnum = false;
    while let Some(c) = chars.next() {
        if c.is_alphanumeric() {
            tmp.clear();
            fold_char(c, &mut tmp);
            out.extend_from_slice(tmp.as_bytes());
            prev_alnum = true;
        } else if is_apostrophe(c) && prev_alnum && chars.peek().is_some_and(|n| n.is_alphanumeric()) {
            // skip: joins the two halves
        } else {
            prev_alnum = false;
            push_space(out);
        }
    }
}

fn fold_char(c: char, out: &mut String) {
    if c.is_ascii() {
        out.push(c.to_ascii_lowercase());
        return;
    }
    let folded = match c {
        'À'..='Å' | 'à'..='å' | 'Ā' | 'ā' | 'Ă' | 'ă' | 'Ą' | 'ą' => "a",
        'Æ' | 'æ' => "ae",
        'Ç' | 'ç' | 'Ć' | 'ć' | 'Č' | 'č' => "c",
        'È'..='Ë' | 'è'..='ë' | 'Ē' | 'ē' | 'Ė' | 'ė' | 'Ę' | 'ę' | 'Ě' | 'ě' => "e",
        'Ì'..='Ï' | 'ì'..='ï' | 'Ī' | 'ī' | 'Į' | 'į' => "i",
        'Ñ' | 'ñ' | 'Ń' | 'ń' | 'Ň' | 'ň' => "n",
        'Ò'..='Ö' | 'Ø' | 'ò'..='ö' | 'ø' | 'Ō' | 'ō' | 'Ő' | 'ő' => "o",
        'Œ' | 'œ' => "oe",
        'Ù'..='Ü' | 'ù'..='ü' | 'Ū' | 'ū' | 'Ů' | 'ů' | 'Ű' | 'ű' => "u",
        'Ý' | 'ý' | 'ÿ' | 'Ÿ' => "y",
        'ß' => "ss",
        'Š' | 'š' | 'Ś' | 'ś' => "s",
        'Ž' | 'ž' | 'Ź' | 'ź' | 'Ż' | 'ż' => "z",
        'Ł' | 'ł' => "l",
        _ => {
            out.extend(c.to_lowercase());
            return;
        }
    };
    out.push_str(folded);
}

/// Convenience wrapper that times a search.
pub struct SearchOutcome {
    pub verses: Vec<u32>,
    pub elapsed: Duration,
}

pub fn run(t: &Translation, input: &str, scope: Scope, whole_words: bool) -> SearchOutcome {
    let start = Instant::now();
    let q = Query::parse(input, whole_words);
    let verses = t.search_index().search(t, &q, scope);
    SearchOutcome { verses, elapsed: start.elapsed() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrases_end_in_a_prefix_unless_whole_words() {
        let text = "a fountain of gardens, a well of living waters";
        assert_eq!(Query::parse("\"living water\"", false).match_ranges(text).len(), 1);
        assert!(Query::parse("\"living water\"", true).match_ranges(text).is_empty());
        assert!(Query::parse("“living water”", true).match_ranges("given thee living water.").len() == 1);
    }
    use crate::books::VerseRef;
    use crate::model::{MarkKind, TranslationBuilder, TranslationMeta};

    fn sample() -> Translation {
        let mut b = TranslationBuilder::new(TranslationMeta::default());
        b.begin_book(BookId(0));
        b.begin_chapter(1);
        b.begin_verse(1, 1);
        b.text("In the beginning God created the heaven and the earth.");
        b.begin_verse(2, 2);
        b.text("And the earth was without form, and void.");
        b.begin_book(BookId(42));
        b.begin_chapter(3);
        b.begin_verse(16, 16);
        b.text("For God so loved the world, that he gave his only begotten Son.");
        b.begin_verse(17, 17);
        b.text("For God sent not his Son into the world to condemn the");
        b.mark(MarkKind::Poetry, 2);
        b.text("world; but that the world through him might be saved. Father’s Beer-sheba Élan");
        b.finish()
    }

    fn refs(t: &Translation, q: &str, scope: Scope) -> Vec<String> {
        run(t, q, scope, false)
            .verses
            .iter()
            .map(|&i| t.verse_view(i).unwrap().reference().to_string())
            .collect()
    }

    #[test]
    fn prefix_and_whole_word_terms() {
        let t = sample();
        assert_eq!(refs(&t, "begin", Scope::All), ["Genesis 1:1"]);
        assert_eq!(refs(&t, "lov", Scope::All), ["John 3:16"]);
        assert_eq!(refs(&t, "god world", Scope::All), ["John 3:16", "John 3:17"]);
        assert_eq!(refs(&t, "earth", Scope::NewTestament), Vec::<String>::new());
        assert_eq!(refs(&t, "earth", Scope::Book(BookId(0))), ["Genesis 1:1", "Genesis 1:2"]);
        // In whole-word mode a partial word matches nothing.
        assert_eq!(run(&t, "worl", Scope::All, true).verses.len(), 0);
    }

    #[test]
    fn phrases_exclusions_and_normalization() {
        let t = sample();
        assert_eq!(refs(&t, "\"so loved the world\"", Scope::All), ["John 3:16"]);
        assert_eq!(refs(&t, "\"loved world\"", Scope::All), Vec::<String>::new());
        assert_eq!(refs(&t, "god -condemn", Scope::All), ["Genesis 1:1", "John 3:16"]);
        assert_eq!(refs(&t, "fathers", Scope::All), ["John 3:17"]);
        assert_eq!(refs(&t, "father's", Scope::All), ["John 3:17"]);
        assert_eq!(refs(&t, "beer-sheba", Scope::All), ["John 3:17"]);
        assert_eq!(refs(&t, "elan", Scope::All), ["John 3:17"]);
        // A poetry line break inside a verse still separates words.
        assert_eq!(refs(&t, "\"the world but\"", Scope::All), ["John 3:17"]);
        // Phrases never match across verse boundaries.
        assert_eq!(refs(&t, "\"earth and the\"", Scope::All), Vec::<String>::new());
        assert_eq!(refs(&t, "\"son for god\"", Scope::All), Vec::<String>::new());
    }

    #[test]
    fn query_parsing() {
        let q = Query::parse(r#"love "one anoth -hate"#, false);
        assert_eq!(q.terms.len(), 2);
        assert!(q.terms[1].phrase && !q.terms[1].whole_last);
        assert_eq!(q.terms[1].words, ["one", "anoth", "hate"]);
        let q = Query::parse("-hate love", false);
        assert_eq!(q.excluded.len(), 1);
        assert_eq!(q.terms[0].words, ["love"]);
        assert!(Query::parse("  ", false).is_empty());
        assert!(Query::parse("—", false).is_empty());
    }

    #[test]
    fn highlight_ranges() {
        let q = Query::parse("god \"the world\"", false);
        let text = "For God so loved the world, that God…";
        let r = q.match_ranges(text);
        let parts: Vec<&str> = r.iter().map(|r| &text[r.clone()]).collect();
        assert_eq!(parts, ["God", "the world", "God"]);
        let q = Query::parse("father", false);
        let text = "my Father’s house";
        assert_eq!(q.match_ranges(text).iter().map(|r| &text[r.clone()]).collect::<Vec<_>>(), ["Father’s"]);
        let _ = VerseRef::new(BookId(0), 1, 1);
    }
}
