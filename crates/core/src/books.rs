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

    /// Name used in a chapter or verse reference: "Psalm 23", otherwise the book name.
    pub fn ref_name(self) -> &'static str {
        if self == BookId::PSALMS { "Psalm" } else { self.name() }
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
        (chapter >= 1 && chapter <= book.chapters())
            .then(|| ChapterIdx(book.first_chapter().0 + chapter as u16 - 1))
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
        format!("{} {}", self.book().ref_name(), self.chapter())
    }
}

/// Verses per chapter in KJV versification (31,102 verses), indexed by [`ChapterIdx`].
/// Used for SWORD's KJV verse index and for validating references without loading a text.
pub static KJV_VERSES: [u8; CHAPTER_COUNT] = [
    31, 25, 24, 26, 32, 22, 24, 22, 29, 32, 32, 20, 18, 24, 21, 16, 27, 33, 38, 18, 34, 24, 20, 67, 34, 35,
    46, 22, 35, 43, 55, 32, 20, 31, 29, 43, 36, 30, 23, 23, 57, 38, 34, 34, 28, 34, 31, 22, 33, 26, 22, 25,
    22, 31, 23, 30, 25, 32, 35, 29, 10, 51, 22, 31, 27, 36, 16, 27, 25, 26, 36, 31, 33, 18, 40, 37, 21, 43,
    46, 38, 18, 35, 23, 35, 35, 38, 29, 31, 43, 38, 17, 16, 17, 35, 19, 30, 38, 36, 24, 20, 47, 8, 59, 57,
    33, 34, 16, 30, 37, 27, 24, 33, 44, 23, 55, 46, 34, 54, 34, 51, 49, 31, 27, 89, 26, 23, 36, 35, 16, 33,
    45, 41, 50, 13, 32, 22, 29, 35, 41, 30, 25, 18, 65, 23, 31, 40, 16, 54, 42, 56, 29, 34, 13, 46, 37, 29,
    49, 33, 25, 26, 20, 29, 22, 32, 32, 18, 29, 23, 22, 20, 22, 21, 20, 23, 30, 25, 22, 19, 19, 26, 68, 29,
    20, 30, 52, 29, 12, 18, 24, 17, 24, 15, 27, 26, 35, 27, 43, 23, 24, 33, 15, 63, 10, 18, 28, 51, 9, 45,
    34, 16, 33, 36, 23, 31, 24, 31, 40, 25, 35, 57, 18, 40, 15, 25, 20, 20, 31, 13, 31, 30, 48, 25, 22, 23,
    18, 22, 28, 36, 21, 22, 12, 21, 17, 22, 27, 27, 15, 25, 23, 52, 35, 23, 58, 30, 24, 42, 15, 23, 29, 22,
    44, 25, 12, 25, 11, 31, 13, 27, 32, 39, 12, 25, 23, 29, 18, 13, 19, 27, 31, 39, 33, 37, 23, 29, 33, 43,
    26, 22, 51, 39, 25, 53, 46, 28, 34, 18, 38, 51, 66, 28, 29, 43, 33, 34, 31, 34, 34, 24, 46, 21, 43, 29,
    53, 18, 25, 27, 44, 27, 33, 20, 29, 37, 36, 21, 21, 25, 29, 38, 20, 41, 37, 37, 21, 26, 20, 37, 20, 30,
    54, 55, 24, 43, 26, 81, 40, 40, 44, 14, 47, 40, 14, 17, 29, 43, 27, 17, 19, 8, 30, 19, 32, 31, 31, 32,
    34, 21, 30, 17, 18, 17, 22, 14, 42, 22, 18, 31, 19, 23, 16, 22, 15, 19, 14, 19, 34, 11, 37, 20, 12, 21,
    27, 28, 23, 9, 27, 36, 27, 21, 33, 25, 33, 27, 23, 11, 70, 13, 24, 17, 22, 28, 36, 15, 44, 11, 20, 32,
    23, 19, 19, 73, 18, 38, 39, 36, 47, 31, 22, 23, 15, 17, 14, 14, 10, 17, 32, 3, 22, 13, 26, 21, 27, 30,
    21, 22, 35, 22, 20, 25, 28, 22, 35, 22, 16, 21, 29, 29, 34, 30, 17, 25, 6, 14, 23, 28, 25, 31, 40, 22,
    33, 37, 16, 33, 24, 41, 30, 24, 34, 17, 6, 12, 8, 8, 12, 10, 17, 9, 20, 18, 7, 8, 6, 7, 5, 11, 15, 50,
    14, 9, 13, 31, 6, 10, 22, 12, 14, 9, 11, 12, 24, 11, 22, 22, 28, 12, 40, 22, 13, 17, 13, 11, 5, 26, 17,
    11, 9, 14, 20, 23, 19, 9, 6, 7, 23, 13, 11, 11, 17, 12, 8, 12, 11, 10, 13, 20, 7, 35, 36, 5, 24, 20, 28,
    23, 10, 12, 20, 72, 13, 19, 16, 8, 18, 12, 13, 17, 7, 18, 52, 17, 16, 15, 5, 23, 11, 13, 12, 9, 9, 5, 8,
    28, 22, 35, 45, 48, 43, 13, 31, 7, 10, 10, 9, 8, 18, 19, 2, 29, 176, 7, 8, 9, 4, 8, 5, 6, 5, 6, 8, 8, 3,
    18, 3, 3, 21, 26, 9, 8, 24, 13, 10, 7, 12, 15, 21, 10, 20, 14, 9, 6, 33, 22, 35, 27, 23, 35, 27, 36, 18,
    32, 31, 28, 25, 35, 33, 33, 28, 24, 29, 30, 31, 29, 35, 34, 28, 28, 27, 28, 27, 33, 31, 18, 26, 22, 16,
    20, 12, 29, 17, 18, 20, 10, 14, 17, 17, 11, 16, 16, 13, 13, 14, 31, 22, 26, 6, 30, 13, 25, 22, 21, 34,
    16, 6, 22, 32, 9, 14, 14, 7, 25, 6, 17, 25, 18, 23, 12, 21, 13, 29, 24, 33, 9, 20, 24, 17, 10, 22, 38,
    22, 8, 31, 29, 25, 28, 28, 25, 13, 15, 22, 26, 11, 23, 15, 12, 17, 13, 12, 21, 14, 21, 22, 11, 12, 19,
    12, 25, 24, 19, 37, 25, 31, 31, 30, 34, 22, 26, 25, 23, 17, 27, 22, 21, 21, 27, 23, 15, 18, 14, 30, 40,
    10, 38, 24, 22, 17, 32, 24, 40, 44, 26, 22, 19, 32, 21, 28, 18, 16, 18, 22, 13, 30, 5, 28, 7, 47, 39, 46,
    64, 34, 22, 22, 66, 22, 22, 28, 10, 27, 17, 17, 14, 27, 18, 11, 22, 25, 28, 23, 23, 8, 63, 24, 32, 14,
    49, 32, 31, 49, 27, 17, 21, 36, 26, 21, 26, 18, 32, 33, 31, 15, 38, 28, 23, 29, 49, 26, 20, 27, 31, 25,
    24, 23, 35, 21, 49, 30, 37, 31, 28, 28, 27, 27, 21, 45, 13, 11, 23, 5, 19, 15, 11, 16, 14, 17, 15, 12,
    14, 16, 9, 20, 32, 21, 15, 16, 15, 13, 27, 14, 17, 14, 15, 21, 17, 10, 10, 11, 16, 13, 12, 13, 15, 16,
    20, 15, 13, 19, 17, 20, 19, 18, 15, 20, 15, 23, 21, 13, 10, 14, 11, 15, 14, 23, 17, 12, 17, 14, 9, 21,
    14, 17, 18, 6, 25, 23, 17, 25, 48, 34, 29, 34, 38, 42, 30, 50, 58, 36, 39, 28, 27, 35, 30, 34, 46, 46,
    39, 51, 46, 75, 66, 20, 45, 28, 35, 41, 43, 56, 37, 38, 50, 52, 33, 44, 37, 72, 47, 20, 80, 52, 38, 44,
    39, 49, 50, 56, 62, 42, 54, 59, 35, 35, 32, 31, 37, 43, 48, 47, 38, 71, 56, 53, 51, 25, 36, 54, 47, 71,
    53, 59, 41, 42, 57, 50, 38, 31, 27, 33, 26, 40, 42, 31, 25, 26, 47, 26, 37, 42, 15, 60, 40, 43, 48, 30,
    25, 52, 28, 41, 40, 34, 28, 41, 38, 40, 30, 35, 27, 27, 32, 44, 31, 32, 29, 31, 25, 21, 23, 25, 39, 33,
    21, 36, 21, 14, 23, 33, 27, 31, 16, 23, 21, 13, 20, 40, 13, 27, 33, 34, 31, 13, 40, 58, 24, 24, 17, 18,
    18, 21, 18, 16, 24, 15, 18, 33, 21, 14, 24, 21, 29, 31, 26, 18, 23, 22, 21, 32, 33, 24, 30, 30, 21, 23,
    29, 23, 25, 18, 10, 20, 13, 18, 28, 12, 17, 18, 20, 15, 16, 16, 25, 21, 18, 26, 17, 22, 16, 15, 15, 25,
    14, 18, 19, 16, 14, 20, 28, 13, 28, 39, 40, 29, 25, 27, 26, 18, 17, 20, 25, 25, 22, 19, 14, 21, 22, 18,
    10, 29, 24, 21, 21, 13, 14, 25, 20, 29, 22, 11, 14, 17, 17, 13, 21, 11, 19, 17, 18, 20, 8, 21, 18, 24,
    21, 15, 27, 21,
];

impl ChapterIdx {
    /// Number of verses in this chapter in KJV versification.
    pub fn kjv_verses(self) -> u8 {
        KJV_VERSES[self.index()]
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
        self.to_string()
    }
}

impl std::fmt::Display for VerseRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}:{}", self.book.ref_name(), self.chapter, self.verse)
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
    fn kjv_verse_table() {
        assert_eq!(KJV_VERSES.iter().map(|&v| v as u32).sum::<u32>(), 31_102);
        assert_eq!(ChapterIdx::new(BookId::PSALMS, 119).unwrap().kjv_verses(), 176);
        assert_eq!(ChapterIdx::new(BookId(42), 3).unwrap().kjv_verses(), 36);
    }

    #[test]
    fn verse_keys_round_trip() {
        let v = VerseRef::new(BookId(42), 3, 16);
        assert_eq!(VerseRef::from_key(v.key()), Some(v));
        assert_eq!(v.to_string(), "John 3:16");
    }
}
