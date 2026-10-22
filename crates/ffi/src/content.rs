//! A chapter as the phone apps draw it, laid out by [`bible_core::content`]: blocks (paragraphs,
//! poetry lines, headings) of styled runs, each with the space above it.
//!
//! The definitions below describe the core's types to UniFFI and must match them field for field;
//! the doc comments are what Swift sees.
//!
//! Spacing is in points at the design's 20 pt reading size; apps scale it with their text size.

pub use bible_core::content::{
    Block, BlockKind, ChapterContent, ChapterStatus, ContentOptions, Run, chapter,
};

#[uniffi::remote(Record)]
pub struct ContentOptions {
    /// Every verse starts a line, its number in the margin.
    pub verse_per_line: bool,
    /// Section headings. Psalm titles show either way.
    pub headings: bool,
    pub verse_numbers: bool,
}

#[uniffi::remote(Enum)]
pub enum ChapterStatus {
    Ready,
    /// The translation doesn't have this chapter (a New Testament only import, say).
    Missing,
    /// An online chapter still on its way.
    Pending,
    /// An online translation whose key isn't set.
    NoKey,
    Failed {
        message: String,
    },
}

#[uniffi::remote(Record)]
pub struct ChapterContent {
    /// The chapter's index in the canonical grid (0 = Genesis 1, 1188 = Revelation 22).
    pub index: u16,
    pub book: u8,
    pub chapter: u8,
    pub status: ChapterStatus,
    /// The book's name as the translation spells it ("John", "Psalms").
    pub book_name: String,
    /// The book's full title, where the translation has one that differs from its name ("The
    /// Gospel According to John"); empty otherwise.
    pub book_title: String,
    pub blocks: Vec<Block>,
}

#[uniffi::remote(Enum)]
pub enum BlockKind {
    /// A paragraph of prose; `indent` asks for a first-line indent.
    Prose {
        indent: bool,
    },
    /// A line of poetry (or a list item), `level` 1–4. Wrapped lines hang.
    Poetry {
        level: u8,
    },
    /// A right-aligned line, such as "Selah".
    Right,
    Center,
    /// A section heading: `level` 0 is a major heading, 1–2 the usual ones, 3 an acrostic letter.
    Heading {
        level: u8,
    },
    /// A heading that opens a psalm, set large right under its PSALM n label.
    PsalmHeading,
    /// A psalm's superscription ("A Psalm of David.").
    PsalmTitle,
    /// A speaker's name (Song of Solomon).
    Speaker,
}

#[uniffi::remote(Record)]
pub struct Block {
    pub kind: BlockKind,
    /// Space above the block, in points at a 20 pt reading size.
    pub space_before: f32,
    /// A verse number set in the margin before the first line, for poetry lines that begin a verse
    /// and for every verse in verse-per-line mode. Other verse numbers are runs.
    pub gutter: Option<String>,
    /// The verse the gutter number belongs to.
    pub gutter_verse: u8,
    pub runs: Vec<Run>,
}

#[uniffi::remote(Record)]
pub struct Run {
    pub text: String,
    /// The verse the run belongs to; 0 for headings.
    pub verse: u8,
    /// An inline verse number, ending in a no-break space so it stays with its verse.
    pub number: bool,
    /// Words of Christ, red when red letters are on.
    pub red: bool,
    /// Translator additions (KJV), "Selah" and the source's italics.
    pub italic: bool,
    pub bold: bool,
    /// Letters set in small capitals: the rest of LORD after its L, and of other words printed in
    /// capitals. The text is already upper case.
    pub small_caps: bool,
    /// Where the run's text starts in its verse's text, in UTF-16 code units; 0 for numbers and
    /// headings.
    pub offset: u32,
    /// A space the layout adds, not in the verse's text.
    pub added: bool,
}
