//! Parsing human-typed references: "jn 3:16", "Rom 8", "1co13:4-7", "ii kings 2", "song 2",
//! "revalation 21" (typo-tolerant), "jude 3" (single-chapter books take a verse).

use crate::books::{BOOKS, BookId, ChapterIdx, VerseRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reference {
    pub book: BookId,
    pub chapter: u8,
    pub verse: Option<u8>,
    pub verse_end: Option<u8>,
}

impl Reference {
    pub fn chapter_idx(&self) -> ChapterIdx {
        ChapterIdx::new(self.book, self.chapter).unwrap_or(self.book.first_chapter())
    }

    pub fn verse_ref(&self) -> VerseRef {
        VerseRef::new(self.book, self.chapter, self.verse.unwrap_or(1))
    }

    pub fn display(&self) -> String {
        let mut s = format!("{} {}", self.book.ref_name(), self.chapter);
        if let Some(v) = self.verse {
            s.push_str(&format!(":{v}"));
            if let Some(e) = self.verse_end.filter(|&e| e > v) {
                s.push_str(&format!("–{e}"));
            }
        }
        s
    }
}

/// Formats a contiguous verse range: "John 3:16", "John 3:16–18".
pub fn format_range(from: VerseRef, to: VerseRef) -> String {
    if from == to || from.book != to.book || from.chapter != to.chapter {
        if from.book == to.book && from.chapter != to.chapter {
            return format!(
                "{} {}:{}–{}:{}",
                from.book.name(),
                from.chapter,
                from.verse,
                to.chapter,
                to.verse
            );
        }
        return from.to_string();
    }
    format!("{} {}:{}–{}", from.book.ref_name(), from.chapter, from.verse, to.verse)
}

/// The best single interpretation of `input`, if any.
pub fn parse(input: &str) -> Option<Reference> {
    suggest(input, 1).into_iter().next()
}

/// Ranked interpretations of `input` (book candidates combined with the typed numbers).
pub fn suggest(input: &str, limit: usize) -> Vec<Reference> {
    let Some(parts) = split(input) else { return Vec::new() };
    let mut out = Vec::new();
    for (book, _score) in book_candidates(&parts.book_key, parts.ordinal) {
        let r = match (parts.nums.first(), parts.nums.get(1), parts.nums.get(2)) {
            (None, _, _) => Some(Reference { book, chapter: 1, verse: None, verse_end: None }),
            // Single-chapter books: "jude 3" means verse 3.
            (Some(&n), None, _) if book.chapters() == 1 && n > 1 && n <= 255 => {
                Some(Reference { book, chapter: 1, verse: Some(n as u8), verse_end: None })
            }
            (Some(&c), v, e) if c >= 1 && c <= book.chapters() as u32 => Some(Reference {
                book,
                chapter: c as u8,
                verse: v.and_then(|&v| u8::try_from(v).ok()).filter(|&v| v > 0),
                verse_end: e.and_then(|&e| u8::try_from(e).ok()),
            }),
            _ => None,
        };
        if let Some(r) = r {
            out.push(r);
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

struct Parts {
    ordinal: Option<u8>,
    book_key: String,
    nums: Vec<u32>,
}

fn split(input: &str) -> Option<Parts> {
    let s = input.trim().to_lowercase();
    let mut rest = s.as_str();
    let mut ordinal = None;
    // Leading ordinal: "1", "1st", "first", "i", "ii", "iii".
    for (prefix, n) in [
        ("first ", 1),
        ("second ", 2),
        ("third ", 3),
        ("1st", 1),
        ("2nd", 2),
        ("3rd", 3),
        ("iii ", 3),
        ("ii ", 2),
        ("i ", 1),
        ("1", 1),
        ("2", 2),
        ("3", 3),
    ] {
        if let Some(r) = rest.strip_prefix(prefix) {
            // "1" alone or "1 3" is not an ordinal; "1co", "1 cor" are.
            if r.trim_start().starts_with(|c: char| c.is_alphabetic()) {
                ordinal = Some(n);
                rest = r.trim_start();
                break;
            }
        }
    }
    let letters_end = rest.find(|c: char| c.is_ascii_digit()).unwrap_or(rest.len());
    let book_key: String = rest[..letters_end].chars().filter(|c| c.is_alphanumeric()).collect();
    if book_key.is_empty() {
        return None;
    }
    let nums: Vec<u32> = rest[letters_end..]
        .split(|c: char| !c.is_ascii_digit())
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.parse().ok())
        .take(3)
        .collect();
    Some(Parts { ordinal, book_key, nums })
}

fn normalized_name(b: &crate::books::BookInfo) -> String {
    b.name.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

/// Book candidates for a typed key, best first.
pub fn book_candidates(key: &str, ordinal: Option<u8>) -> Vec<(BookId, i32)> {
    let full_key = match ordinal {
        Some(n) => format!("{n}{key}"),
        None => key.to_string(),
    };
    let mut scored: Vec<(BookId, i32)> = Vec::new();
    for (i, info) in BOOKS.iter().enumerate() {
        let book = BookId(i as u8);
        let book_ordinal = info.name.chars().next().and_then(|c| c.to_digit(10)).map(|d| d as u8);
        if ordinal.is_some() && ordinal != book_ordinal {
            continue;
        }
        let name = normalized_name(info);
        // Name without its ordinal digit, compared against the typed letters.
        let bare = name.trim_start_matches(|c: char| c.is_ascii_digit());
        let popularity = info.chapters.min(40) as i32 / 4 + if book.0 >= 39 { 3 } else { 0 };
        let mut best = i32::MIN;
        if info.aliases.contains(&full_key.as_str()) || name == full_key {
            best = 1000;
        } else if bare.starts_with(key) {
            best = 900 - (bare.len() as i32 - key.len() as i32) + popularity;
        } else if info
            .aliases
            .iter()
            .any(|a| a.trim_start_matches(|c: char| c.is_ascii_digit()).starts_with(key))
        {
            best = 800 + popularity;
        } else if key.len() >= 2 && is_subsequence(key, bare) && bare.starts_with(&key[..1]) {
            best = 600 - (bare.len() as i32 - key.len() as i32) + popularity;
        } else if key.len() >= 4 {
            let d = damerau_levenshtein(key, bare)
                .min(damerau_levenshtein(key, &bare[..bare.len().min(key.len())]));
            if d <= 2 {
                best = 500 - 60 * d as i32 + popularity;
            }
        }
        if best > i32::MIN {
            scored.push((book, best));
        }
    }
    scored.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    scored
}

fn is_subsequence(needle: &str, hay: &str) -> bool {
    let mut it = hay.chars();
    needle.chars().all(|c| it.any(|h| h == c))
}

fn damerau_levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (n, m) = (a.len(), b.len());
    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for j in 0..=m {
        d[0][j] = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            d[i][j] = (d[i - 1][j] + 1).min(d[i][j - 1] + 1).min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[n][m]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> String {
        parse(s).map(|r| r.display()).unwrap_or_else(|| "-".into())
    }

    #[test]
    fn common_forms() {
        assert_eq!(p("jn 3:16"), "John 3:16");
        assert_eq!(p("John 3.16"), "John 3:16");
        assert_eq!(p("jhn3:16"), "John 3:16");
        assert_eq!(p("Rom 8"), "Romans 8");
        assert_eq!(p("rom"), "Romans 1");
        assert_eq!(p("1co13:4-7"), "1 Corinthians 13:4–7");
        assert_eq!(p("1 cor 13"), "1 Corinthians 13");
        assert_eq!(p("i cor 13"), "1 Corinthians 13");
        assert_eq!(p("II Kings 2"), "2 Kings 2");
        assert_eq!(p("second samuel 7"), "2 Samuel 7");
        assert_eq!(p("3 john"), "3 John 1");
        assert_eq!(p("ps 23"), "Psalm 23");
        assert_eq!(p("psalm 119:105"), "Psalm 119:105");
        assert_eq!(p("song 2"), "Song of Solomon 2");
        assert_eq!(p("Song of Songs 2"), "Song of Solomon 2");
        assert_eq!(p("jude 3"), "Jude 1:3");
        assert_eq!(p("phil 4:13"), "Philippians 4:13");
        assert_eq!(p("phlm"), "Philemon 1");
        assert_eq!(p("mt 5"), "Matthew 5");
        assert_eq!(p("mk 1"), "Mark 1");
        assert_eq!(p("rev 21"), "Revelation 21");
        assert_eq!(p("gen 1:1"), "Genesis 1:1");
        assert_eq!(p("deut 6:4"), "Deuteronomy 6:4");
        assert_eq!(p("ecc 3"), "Ecclesiastes 3");
        assert_eq!(p("eph 2:8-9"), "Ephesians 2:8–9");
    }

    #[test]
    fn fuzzy_forms() {
        assert_eq!(p("revalation 21"), "Revelation 21");
        assert_eq!(p("mathew 5"), "Matthew 5");
        assert_eq!(p("phillipians 4"), "Philippians 4");
        assert_eq!(p("hbr 11"), "Hebrews 11");
        assert_eq!(p("lmnt 3"), "Lamentations 3");
        assert_eq!(p("isiah 53"), "Isaiah 53");
    }

    #[test]
    fn invalid_and_ambiguous() {
        assert_eq!(p(""), "-");
        assert_eq!(p("3:16"), "-");
        assert_eq!(p("xyzzy"), "-");
        // Genesis has 50 chapters; the next candidate that fits wins or nothing.
        assert!(parse("gen 51").is_none_or(|r| r.book != BookId::GENESIS));
        let s = suggest("jo 3", 5);
        assert_eq!(s[0].book.name(), "John");
        assert!(s.len() >= 3);
    }

    #[test]
    fn range_formatting() {
        let a = VerseRef::new(BookId(42), 3, 16);
        assert_eq!(format_range(a, a), "John 3:16");
        assert_eq!(format_range(a, VerseRef::new(BookId(42), 3, 18)), "John 3:16–18");
        assert_eq!(format_range(a, VerseRef::new(BookId(42), 4, 2)), "John 3:16–4:2");
    }
}
