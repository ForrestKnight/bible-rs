//! The canonical 66-book Protestant canon in English (KJV) versification.
//!
//! Every translation — bundled, imported or fetched — is mapped onto this grid of
//! 66 books and 1,189 chapters, which is what makes parallel and compare views line up.

use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BookId(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Testament {
    Old,
    New,
}

#[derive(Debug)]
pub struct BookInfo {
    /// Paratext/USFM three-letter code (`GEN`, `1CO`, …); also used by API.Bible.
    pub code: &'static str,
    /// OSIS book name (`Gen`, `1Cor`, …).
    pub osis: &'static str,
    /// Display name.
    pub name: &'static str,
    /// Compact display abbreviation.
    pub abbrev: &'static str,
    pub chapters: u8,
    /// Extra lookup aliases (lower-case, already normalized: no dots or spaces).
    pub aliases: &'static [&'static str],
}

macro_rules! book {
    ($code:literal, $osis:literal, $name:literal, $abbrev:literal, $ch:literal, [$($a:literal),*]) => {
        BookInfo { code: $code, osis: $osis, name: $name, abbrev: $abbrev, chapters: $ch, aliases: &[$($a),*] }
    };
}

pub static BOOKS: [BookInfo; 66] = [
    book!("GEN", "Gen", "Genesis", "Gen", 50, ["gen", "ge", "gn"]),
    book!("EXO", "Exod", "Exodus", "Exod", 40, ["ex", "exo", "exod", "exd"]),
    book!("LEV", "Lev", "Leviticus", "Lev", 27, ["lev", "le", "lv"]),
    book!("NUM", "Num", "Numbers", "Num", 36, ["num", "nu", "nm", "nb"]),
    book!("DEU", "Deut", "Deuteronomy", "Deut", 34, ["deut", "deu", "de", "dt"]),
    book!("JOS", "Josh", "Joshua", "Josh", 24, ["josh", "jos", "jsh"]),
    book!("JDG", "Judg", "Judges", "Judg", 21, ["judg", "jdg", "jg", "jdgs"]),
    book!("RUT", "Ruth", "Ruth", "Ruth", 4, ["ruth", "rth", "ru", "rut"]),
    book!("1SA", "1Sam", "1 Samuel", "1 Sam", 31, ["1sam", "1sa", "1sm", "1s", "1kgdms"]),
    book!("2SA", "2Sam", "2 Samuel", "2 Sam", 24, ["2sam", "2sa", "2sm", "2s"]),
    book!("1KI", "1Kgs", "1 Kings", "1 Kgs", 22, ["1kgs", "1ki", "1kin", "1k", "1kg"]),
    book!("2KI", "2Kgs", "2 Kings", "2 Kgs", 25, ["2kgs", "2ki", "2kin", "2k", "2kg"]),
    book!("1CH", "1Chr", "1 Chronicles", "1 Chr", 29, ["1chr", "1ch", "1chron", "1chro"]),
    book!("2CH", "2Chr", "2 Chronicles", "2 Chr", 36, ["2chr", "2ch", "2chron", "2chro"]),
    book!("EZR", "Ezra", "Ezra", "Ezra", 10, ["ezra", "ezr", "ez"]),
    book!("NEH", "Neh", "Nehemiah", "Neh", 13, ["neh", "ne"]),
    book!("EST", "Esth", "Esther", "Esth", 10, ["esth", "est", "es"]),
    book!("JOB", "Job", "Job", "Job", 42, ["job", "jb"]),
    book!("PSA", "Ps", "Psalms", "Ps", 150, ["ps", "psa", "psalm", "pss", "psm", "pslm"]),
    book!("PRO", "Prov", "Proverbs", "Prov", 31, ["prov", "pro", "prv", "pr", "prv"]),
    book!("ECC", "Eccl", "Ecclesiastes", "Eccl", 12, ["eccl", "ecc", "ec", "eccles", "qoh", "qoheleth"]),
    book!(
        "SNG",
        "Song",
        "Song of Solomon",
        "Song",
        8,
        ["song", "sng", "sos", "so", "songofsongs", "canticles", "cant", "canticleofcanticles"]
    ),
    book!("ISA", "Isa", "Isaiah", "Isa", 66, ["isa", "is"]),
    book!("JER", "Jer", "Jeremiah", "Jer", 52, ["jer", "je", "jr"]),
    book!("LAM", "Lam", "Lamentations", "Lam", 5, ["lam", "la"]),
    book!("EZK", "Ezek", "Ezekiel", "Ezek", 48, ["ezek", "eze", "ezk"]),
    book!("DAN", "Dan", "Daniel", "Dan", 12, ["dan", "da", "dn"]),
    book!("HOS", "Hos", "Hosea", "Hos", 14, ["hos", "ho"]),
    book!("JOL", "Joel", "Joel", "Joel", 3, ["joel", "jl", "jol"]),
    book!("AMO", "Amos", "Amos", "Amos", 9, ["amos", "am", "amo"]),
    book!("OBA", "Obad", "Obadiah", "Obad", 1, ["obad", "ob", "oba"]),
    book!("JON", "Jonah", "Jonah", "Jonah", 4, ["jonah", "jnh", "jon"]),
    book!("MIC", "Mic", "Micah", "Mic", 7, ["mic", "mc"]),
    book!("NAM", "Nah", "Nahum", "Nah", 3, ["nah", "na", "nam"]),
    book!("HAB", "Hab", "Habakkuk", "Hab", 3, ["hab", "hb"]),
    book!("ZEP", "Zeph", "Zephaniah", "Zeph", 3, ["zeph", "zep", "zp"]),
    book!("HAG", "Hag", "Haggai", "Hag", 2, ["hag", "hg"]),
    book!("ZEC", "Zech", "Zechariah", "Zech", 14, ["zech", "zec", "zc"]),
    book!("MAL", "Mal", "Malachi", "Mal", 4, ["mal", "ml"]),
    book!("MAT", "Matt", "Matthew", "Matt", 28, ["matt", "mat", "mt"]),
    book!("MRK", "Mark", "Mark", "Mark", 16, ["mark", "mrk", "mk", "mr", "mar"]),
    book!("LUK", "Luke", "Luke", "Luke", 24, ["luke", "luk", "lk", "lu"]),
    book!("JHN", "John", "John", "John", 21, ["john", "jhn", "jn", "joh", "jo"]),
    book!("ACT", "Acts", "Acts", "Acts", 28, ["acts", "act", "ac"]),
    book!("ROM", "Rom", "Romans", "Rom", 16, ["rom", "ro", "rm"]),
    book!("1CO", "1Cor", "1 Corinthians", "1 Cor", 16, ["1cor", "1co"]),
    book!("2CO", "2Cor", "2 Corinthians", "2 Cor", 13, ["2cor", "2co"]),
    book!("GAL", "Gal", "Galatians", "Gal", 6, ["gal", "ga"]),
    book!("EPH", "Eph", "Ephesians", "Eph", 6, ["eph", "ephes"]),
    book!("PHP", "Phil", "Philippians", "Phil", 4, ["phil", "php", "pp", "philip"]),
    book!("COL", "Col", "Colossians", "Col", 4, ["col", "co"]),
    book!("1TH", "1Thess", "1 Thessalonians", "1 Thess", 5, ["1thess", "1th", "1thes", "1ts"]),
    book!("2TH", "2Thess", "2 Thessalonians", "2 Thess", 3, ["2thess", "2th", "2thes", "2ts"]),
    book!("1TI", "1Tim", "1 Timothy", "1 Tim", 6, ["1tim", "1ti", "1tm"]),
    book!("2TI", "2Tim", "2 Timothy", "2 Tim", 4, ["2tim", "2ti", "2tm"]),
    book!("TIT", "Titus", "Titus", "Titus", 3, ["titus", "tit", "ti"]),
    book!("PHM", "Phlm", "Philemon", "Phlm", 1, ["phlm", "phm", "philem", "pm"]),
    book!("HEB", "Heb", "Hebrews", "Heb", 13, ["heb", "he"]),
    book!("JAS", "Jas", "James", "Jas", 5, ["jas", "jm", "jam", "james"]),
    book!("1PE", "1Pet", "1 Peter", "1 Pet", 5, ["1pet", "1pe", "1pt", "1p"]),
    book!("2PE", "2Pet", "2 Peter", "2 Pet", 3, ["2pet", "2pe", "2pt", "2p"]),
    book!("1JN", "1John", "1 John", "1 John", 5, ["1john", "1jn", "1jhn", "1j", "1jo"]),
    book!("2JN", "2John", "2 John", "2 John", 1, ["2john", "2jn", "2jhn", "2j", "2jo"]),
    book!("3JN", "3John", "3 John", "3 John", 1, ["3john", "3jn", "3jhn", "3j", "3jo"]),
    book!("JUD", "Jude", "Jude", "Jude", 1, ["jude", "jud", "jd"]),
    book!("REV", "Rev", "Revelation", "Rev", 22, ["rev", "re", "rv", "revelations", "apocalypse", "apoc"]),
];

pub const BOOK_COUNT: usize = 66;
/// Number of chapters in the canonical grid.
pub const CHAPTER_COUNT: usize = 1189;
/// First New Testament book (Matthew).
pub const FIRST_NT_BOOK: BookId = BookId(39);

impl BookId {
    pub const GENESIS: BookId = BookId(0);
    pub const PSALMS: BookId = BookId(18);
    pub const REVELATION: BookId = BookId(65);

    pub fn new(index: usize) -> Option<Self> {
        (index < BOOK_COUNT).then_some(BookId(index as u8))
    }

    pub fn info(self) -> &'static BookInfo {
        &BOOKS[self.0 as usize]
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }

    pub fn name(self) -> &'static str {
        self.info().name
    }

    pub fn chapters(self) -> u8 {
        self.info().chapters
    }

    pub fn testament(self) -> Testament {
        if self < FIRST_NT_BOOK { Testament::Old } else { Testament::New }
    }

    pub fn all() -> impl Iterator<Item = BookId> {
        (0..BOOK_COUNT as u8).map(BookId)
    }

    pub fn from_code(code: &str) -> Option<Self> {
        let code = code.trim();
        BOOKS.iter().position(|b| b.code.eq_ignore_ascii_case(code)).map(|i| BookId(i as u8))
    }

    pub fn from_osis(osis: &str) -> Option<Self> {
        BOOKS.iter().position(|b| b.osis.eq_ignore_ascii_case(osis)).map(|i| BookId(i as u8))
    }

    /// Global index of this book's first chapter in the canonical grid.
    pub fn first_chapter(self) -> ChapterIdx {
        ChapterIdx(chapter_offsets()[self.index()])
    }

    /// Is this a book whose text is mostly poetry (used for typographic defaults)?
    pub fn is_poetic(self) -> bool {
        matches!(self.0, 17..=21 | 24)
    }
}

/// Index of a chapter in the canonical grid: `0` is Genesis 1, `1188` is Revelation 22.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct ChapterIdx(pub u16);

impl ChapterIdx {
    pub const FIRST: ChapterIdx = ChapterIdx(0);
    pub const LAST: ChapterIdx = ChapterIdx(CHAPTER_COUNT as u16 - 1);

    pub fn new(book: BookId, chapter: u8) -> Option<Self> {
        (chapter >= 1 && chapter <= book.chapters()).then(|| ChapterIdx(book.first_chapter().0 + chapter as u16 - 1))
    }

    pub fn book(self) -> BookId {
        let offsets = chapter_offsets();
        // `partition_point` finds the first book whose first chapter is beyond us.
        let next = offsets[..BOOK_COUNT].partition_point(|&start| start <= self.0);
        BookId((next - 1) as u8)
    }

    /// 1-based chapter number within its book.
    pub fn chapter(self) -> u8 {
        (self.0 - self.book().first_chapter().0 + 1) as u8
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }

    pub fn next(self) -> Option<Self> {
        (self.0 < Self::LAST.0).then_some(ChapterIdx(self.0 + 1))
    }

    pub fn prev(self) -> Option<Self> {
        self.0.checked_sub(1).map(ChapterIdx)
    }

    pub fn all() -> impl Iterator<Item = ChapterIdx> {
        (0..CHAPTER_COUNT as u16).map(ChapterIdx)
    }

    pub fn display(self) -> String {
        format!("{} {}", self.book().name(), self.chapter())
    }
}

/// `offsets[b]` = global index of the first chapter of book `b`; `offsets[66]` = 1189.
fn chapter_offsets() -> &'static [u16; BOOK_COUNT + 1] {
    static OFFSETS: OnceLock<[u16; BOOK_COUNT + 1]> = OnceLock::new();
    OFFSETS.get_or_init(|| {
        let mut out = [0u16; BOOK_COUNT + 1];
        for (i, b) in BOOKS.iter().enumerate() {
            out[i + 1] = out[i] + b.chapters as u16;
        }
        out
    })
}

/// A single verse in canonical coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VerseRef {
    pub book: BookId,
    pub chapter: u8,
    pub verse: u8,
}

impl VerseRef {
    pub fn new(book: BookId, chapter: u8, verse: u8) -> Self {
        Self { book, chapter, verse }
    }

    pub fn chapter_idx(self) -> ChapterIdx {
        ChapterIdx::new(self.book, self.chapter).unwrap_or(ChapterIdx::FIRST)
    }

    /// Compact sortable key, handy for hash maps and the study database.
    pub fn key(self) -> u32 {
        (self.book.0 as u32) << 16 | (self.chapter as u32) << 8 | self.verse as u32
    }

    pub fn from_key(key: u32) -> Option<Self> {
        let book = BookId::new((key >> 16) as usize)?;
        Some(Self { book, chapter: (key >> 8) as u8, verse: key as u8 })
    }

    pub fn display(self) -> String {
        format!("{} {}:{}", self.book.name(), self.chapter, self.verse)
    }
}

impl std::fmt::Display for VerseRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}:{}", self.book.name(), self.chapter, self.verse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_has_1189_chapters() {
        assert_eq!(BOOKS.iter().map(|b| b.chapters as usize).sum::<usize>(), CHAPTER_COUNT);
        assert_eq!(BOOKS[..39].iter().map(|b| b.chapters as usize).sum::<usize>(), 929);
        assert_eq!(FIRST_NT_BOOK.name(), "Matthew");
    }

    #[test]
    fn chapter_index_round_trips() {
        for c in ChapterIdx::all() {
            let back = ChapterIdx::new(c.book(), c.chapter()).unwrap();
            assert_eq!(back, c);
        }
        assert_eq!(ChapterIdx(0).display(), "Genesis 1");
        assert_eq!(ChapterIdx::LAST.display(), "Revelation 22");
        let john3 = ChapterIdx::new(BookId::from_code("JHN").unwrap(), 3).unwrap();
        assert_eq!(john3.display(), "John 3");
        assert_eq!(ChapterIdx::new(BookId::PSALMS, 151), None);
    }

    #[test]
    fn codes_and_osis_resolve() {
        assert_eq!(BookId::from_code("1co").unwrap().name(), "1 Corinthians");
        assert_eq!(BookId::from_osis("Phlm").unwrap().name(), "Philemon");
        assert!(BOOKS.iter().all(|b| b.aliases.iter().all(|a| a.chars().all(|c| c.is_ascii_alphanumeric()))));
    }

    #[test]
    fn verse_keys_round_trip() {
        let v = VerseRef::new(BookId(42), 3, 16);
        assert_eq!(VerseRef::from_key(v.key()), Some(v));
        assert_eq!(v.to_string(), "John 3:16");
    }
}
