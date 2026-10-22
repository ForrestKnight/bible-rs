//! A chapter as the phone and web apps draw it: blocks (paragraphs, poetry lines, headings) of
//! styled runs, each with the space above it. The rules are the Mac reader's
//! (`crates/app/src/reader/layout.rs`), so every app breaks and spaces a chapter the same way; the
//! apps only choose fonts and draw.
//!
//! Spacing is in points at the design's 20 pt reading size; apps scale it with their text size.
//!
//! bible-ffi hands these types to Swift with `#[uniffi::remote]` copies of their definitions, so a
//! change here needs the same change there.

use serde::{Deserialize, Serialize};

use crate::model::{MarkKind, Style, Translation, TranslationMeta, VerseView, caps_words};
use crate::{BookId, ChapterIdx};

/// The reading size the design's spacing is drawn at.
const EM: f32 = 20.0;
/// Between paragraphs.
const PARAGRAPH_GAP: f32 = 16.0;
/// Between verses in verse-per-line mode.
const VERSE_GAP: f32 = 8.0;
/// What the chapter head (book title, chapter numeral or PSALM n label) leaves before the text.
const HEAD_SPACE: f32 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ContentOptions {
    /// Every verse starts a line, its number in the margin.
    pub verse_per_line: bool,
    /// Section headings. Psalm titles show either way.
    pub headings: bool,
    pub verse_numbers: bool,
}

impl Default for ContentOptions {
    fn default() -> Self {
        Self { verse_per_line: false, headings: true, verse_numbers: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
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

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
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

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
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
    /// Where the run's text starts in its verse's text, in UTF-16 code units (as JavaScript and
    /// Swift count), to place highlights and selections in it; 0 for numbers and headings.
    pub offset: u32,
    /// A space the layout adds (between verses, or for a line break in a verse set as one
    /// paragraph). It isn't in the verse's text: `offset` is where it falls between two chars.
    pub added: bool,
}

impl ChapterContent {
    /// A chapter that can't be shown, with the reason.
    pub fn unavailable(meta: Option<&TranslationMeta>, c: ChapterIdx, status: ChapterStatus) -> Self {
        let (book_name, book_title) = names(meta, c.book());
        ChapterContent {
            index: c.0,
            book: c.book().0,
            chapter: c.chapter(),
            status,
            book_name,
            book_title,
            blocks: Vec::new(),
        }
    }
}

fn names(meta: Option<&TranslationMeta>, book: BookId) -> (String, String) {
    let Some(meta) = meta else { return (book.name().to_string(), String::new()) };
    let short = meta.book_short(book).to_string();
    let long = meta.books.get(book.index()).map(|t| t.long.clone()).unwrap_or_default();
    let long = if long == short { String::new() } else { long };
    (short, long)
}

/// Lays out the blocks of chapter `c` of `t`.
pub fn chapter(t: &Translation, c: ChapterIdx, opts: ContentOptions) -> ChapterContent {
    let mut b = Builder::new(c, opts);
    let per_line = opts.verse_per_line;
    let mut kind = BlockKind::Prose { indent: false };
    for v in t.verses_in(c) {
        let styles = |off: usize| style_mask(&v, off);
        let mut cursor = 0usize;
        let mut first = true;
        let marks = v.marks;
        let mut mi = 0usize;
        loop {
            let mut broke = false;
            while mi < marks.len() && marks[mi].offset as usize <= cursor {
                let m = &marks[mi];
                mi += 1;
                match m.kind {
                    MarkKind::Heading | MarkKind::PsalmTitle | MarkKind::Speaker => {
                        b.heading(m.kind, m.level, t.mark_text(m));
                        kind = BlockKind::Prose { indent: false };
                    }
                    MarkKind::Blank => {
                        b.flush();
                        b.space = b.space.max(EM * 0.75);
                    }
                    MarkKind::Paragraph => {
                        b.flush();
                        kind = BlockKind::Prose { indent: !b.after_heading && !per_line };
                        b.start(kind);
                        broke = true;
                    }
                    MarkKind::ParagraphFlush => {
                        kind = BlockKind::Prose { indent: false };
                        b.start(kind);
                        broke = true;
                    }
                    MarkKind::Poetry | MarkKind::List => {
                        if matches!(kind, BlockKind::Prose { .. }) && b.text_blocks > 0 {
                            b.space = b.space.max(EM * 0.3);
                        }
                        kind = BlockKind::Poetry { level: m.level.max(1) };
                        b.start(kind);
                        broke = true;
                    }
                    MarkKind::PoetryRight => {
                        b.start(BlockKind::Right);
                        broke = true;
                    }
                    MarkKind::Centered => {
                        b.start(BlockKind::Center);
                        broke = true;
                    }
                }
            }
            if first {
                first = false;
                if per_line && !broke {
                    // A new line in the current kind: poetry stays indented.
                    b.start(match kind {
                        BlockKind::Prose { .. } => BlockKind::Prose { indent: false },
                        k => k,
                    });
                }
                let p = b.ensure(kind);
                // A space from the previous verse in the same block.
                if p.runs.last().is_some_and(|r| !r.text.ends_with([' ', '\u{a0}', '\n'])) {
                    let verse = p.runs.last().map_or(0, |r| r.verse);
                    push_space(&mut p.runs, verse);
                }
                if opts.verse_numbers {
                    let label = if v.number_end > v.number {
                        format!("{}–{}", v.number, v.number_end)
                    } else {
                        v.number.to_string()
                    };
                    // Verse-per-line, and poetry lines that begin a verse, hang their number in
                    // the margin so every line's text aligns.
                    if (per_line || matches!(p.kind, BlockKind::Poetry { .. })) && p.runs.is_empty() {
                        p.gutter = Some(label);
                        p.gutter_verse = v.number;
                    } else {
                        p.runs.push(Run { number: true, ..plain(&format!("{label}\u{a0}"), v.number) });
                    }
                }
            }
            let next = marks.get(mi).map_or(v.text.len(), |m| (m.offset as usize).min(v.text.len()));
            if next > cursor {
                let p = b.ensure(kind);
                styled_runs(
                    &mut p.runs,
                    &v.text[cursor..next],
                    v.number,
                    cursor,
                    utf16(&v.text[..cursor]),
                    &styles,
                );
                cursor = next;
            }
            if mi >= marks.len() && cursor >= v.text.len() {
                break;
            }
            if mi < marks.len() && marks[mi].offset as usize > v.text.len() {
                break;
            }
        }
    }
    b.flush();
    let (book_name, book_title) = names(Some(&t.meta), c.book());
    ChapterContent {
        index: c.0,
        book: c.book().0,
        chapter: c.chapter(),
        status: ChapterStatus::Ready,
        book_name,
        book_title,
        blocks: b.blocks,
    }
}

/// One verse as a single paragraph, as parallel columns, comparisons and share cards set it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerseContent {
    pub number: u8,
    /// The last verse number when the translation joins verses ("7–8"), else `number`.
    pub number_end: u8,
    /// A psalm's superscription, when the verse opens with one.
    pub title: Option<String>,
    pub runs: Vec<Run>,
}

/// Lays out one verse as a paragraph: poetry's line breaks become spaces.
pub fn verse(t: &Translation, v: &VerseView<'_>) -> VerseContent {
    let styles = |off: usize| style_mask(v, off);
    let mut runs = Vec::new();
    let mut cursor = 0usize;
    for m in v.marks.iter().filter(|m| m.kind.breaks_line() && m.offset > 0) {
        let at = (m.offset as usize).min(v.text.len());
        if at > cursor {
            styled_runs(&mut runs, &v.text[cursor..at], v.number, cursor, utf16(&v.text[..cursor]), &styles);
            cursor = at;
            if !runs.last().is_some_and(|r| r.text.ends_with(' ')) && !v.text[at..].starts_with(' ') {
                push_space(&mut runs, v.number);
            }
        }
    }
    if cursor < v.text.len() {
        styled_runs(&mut runs, &v.text[cursor..], v.number, cursor, utf16(&v.text[..cursor]), &styles);
    }
    let titles: Vec<&str> = v
        .marks
        .iter()
        .filter(|m| m.offset == 0 && m.kind == MarkKind::PsalmTitle)
        .map(|m| t.mark_text(m))
        .collect();
    VerseContent {
        number: v.number,
        number_end: v.number_end.max(v.number),
        title: (!titles.is_empty()).then(|| titles.join(" ")),
        runs,
    }
}

struct Builder {
    opts: ContentOptions,
    blocks: Vec<Block>,
    pending: Option<Block>,
    /// Space asked for before the next block; the largest request wins.
    space: f32,
    /// Space a heading leaves below itself, added to whatever comes next.
    after: f32,
    /// Blocks of verse text so far (headings don't count).
    text_blocks: usize,
    /// Nothing but headings since the last paragraph: the next paragraph isn't indented.
    after_heading: bool,
    /// A psalm, whose opening heading and title sit under its PSALM n label.
    psalm: bool,
}

impl Builder {
    fn new(c: ChapterIdx, opts: ContentOptions) -> Self {
        Builder {
            opts,
            blocks: Vec::new(),
            pending: None,
            space: HEAD_SPACE,
            after: 0.0,
            text_blocks: 0,
            after_heading: true,
            psalm: c.book() == BookId::PSALMS,
        }
    }

    fn start(&mut self, kind: BlockKind) {
        self.flush();
        self.pending =
            Some(Block { kind, space_before: 0.0, gutter: None, gutter_verse: 0, runs: Vec::new() });
    }

    fn ensure(&mut self, kind: BlockKind) -> &mut Block {
        if self.pending.is_none() {
            self.start(kind);
        }
        self.pending.as_mut().expect("started")
    }

    fn flush(&mut self) {
        let Some(mut block) = self.pending.take() else { return };
        if block.runs.is_empty() {
            return;
        }
        let gap = match block.kind {
            BlockKind::Prose { .. } if self.opts.verse_per_line => VERSE_GAP,
            BlockKind::Prose { .. } => PARAGRAPH_GAP,
            _ => 0.0,
        };
        // The chapter head is always above, so even the first block gets its gap.
        block.space_before = self.after + gap.max(self.space);
        self.after = 0.0;
        self.space = 0.0;
        self.text_blocks += 1;
        self.after_heading = false;
        self.blocks.push(block);
    }

    fn heading(&mut self, kind: MarkKind, level: u8, text: &str) {
        if !self.opts.headings && kind == MarkKind::Heading {
            return;
        }
        let opening = self.psalm && self.text_blocks == 0;
        let (kind, before, after): (BlockKind, f32, f32) = match kind {
            MarkKind::PsalmTitle => (BlockKind::PsalmTitle, if opening { 0.0 } else { 4.0 }, 8.0),
            MarkKind::Speaker => (BlockKind::Speaker, 12.0, 4.0),
            _ if opening => (BlockKind::PsalmHeading, 4.0, 0.0),
            _ => (BlockKind::Heading { level }, 24.0, 8.0),
        };
        self.flush();
        let mut runs = Vec::new();
        styled_runs(&mut runs, text, 0, 0, 0, &|_| 0);
        if runs.is_empty() {
            return;
        }
        let space_before = self.after + if opening { before } else { before.max(self.space) };
        self.blocks.push(Block { kind, space_before, gutter: None, gutter_verse: 0, runs });
        self.space = 0.0;
        self.after = 0.0;
        if opening {
            // The psalm starts 16 pt below whichever of heading and title comes last.
            self.space = 16.0;
        } else {
            self.after = after;
        }
        self.after_heading = true;
    }
}

fn style_mask(v: &VerseView<'_>, off: usize) -> u8 {
    v.spans
        .iter()
        .filter(|s| (s.start as usize) <= off && off < s.end as usize)
        .fold(0u8, |m, s| m | s.style.bit())
}

fn plain(text: &str, verse: u8) -> Run {
    Run {
        text: text.to_string(),
        verse,
        number: false,
        red: false,
        italic: false,
        bold: false,
        small_caps: false,
        offset: 0,
        added: false,
    }
}

fn utf16(text: &str) -> u32 {
    text.encode_utf16().count() as u32
}

/// Appends `run`, extending the last run when it looks the same and its text follows on.
fn push(runs: &mut Vec<Run>, run: Run) {
    if run.text.is_empty() {
        return;
    }
    if let Some(last) = runs.last_mut()
        && !last.number
        && !run.number
        && !last.added
        && !run.added
        && (last.verse, last.red, last.italic, last.bold, last.small_caps)
            == (run.verse, run.red, run.italic, run.bold, run.small_caps)
        && last.offset + utf16(&last.text) == run.offset
    {
        last.text.push_str(&run.text);
        return;
    }
    runs.push(run);
}

/// Appends a space the layout adds after verse `verse`'s text so far, as a run of its own at the
/// end of that text.
fn push_space(runs: &mut Vec<Run>, verse: u8) {
    let offset = runs
        .last()
        .filter(|r| r.verse == verse && !r.number && !r.added)
        .map_or(0, |r| r.offset + utf16(&r.text));
    push(runs, Run { offset, added: true, ..plain(" ", verse) });
}

/// Appends `text` (which starts `base` bytes, `base16` UTF-16 units, into its verse) as runs of
/// uniform style. The divine name and words in capitals keep each word's first letter and set the
/// rest in small capitals.
fn styled_runs(
    runs: &mut Vec<Run>,
    text: &str,
    verse: u8,
    base: usize,
    base16: u32,
    styles_at: &dyn Fn(usize) -> u8,
) {
    let caps = caps_words(text);
    let small = Style::DivineName.bit() | Style::SmallCaps.bit();
    let at = |i: usize| base16 + utf16(&text[..i]);
    let mut emit = |s: usize, e: usize, st: u8| {
        if s >= e {
            return;
        }
        let run = Run {
            red: st & Style::WordsOfJesus.bit() != 0,
            italic: st & (Style::Added.bit() | Style::Italic.bit() | Style::Selah.bit()) != 0,
            bold: st & Style::Bold.bit() != 0,
            offset: at(s),
            ..plain("", verse)
        };
        if st & small == 0 {
            push(runs, Run { text: text[s..e].to_string(), ..run });
            return;
        }
        let mut word_start = true;
        let mut buf = String::new();
        let mut buf_small = false;
        let mut buf_at = s;
        for (i, c) in text[s..e].char_indices() {
            let is_small = c.is_alphabetic() && !word_start;
            if is_small != buf_small {
                let offset = at(buf_at);
                push(
                    runs,
                    Run { text: std::mem::take(&mut buf), small_caps: buf_small, offset, ..run.clone() },
                );
                buf_small = is_small;
                buf_at = s + i;
            }
            // Upper case only where it is as long, so the run still counts as its verse's text.
            let mut up = c.to_uppercase();
            match (is_small, up.len()) {
                (true, 1) if up.clone().next().is_some_and(|u| u.len_utf16() == c.len_utf16()) => {
                    buf.push(up.next().expect("one char"))
                }
                _ => buf.push(c),
            }
            word_start = !c.is_alphabetic();
        }
        push(runs, Run { text: buf, small_caps: buf_small, offset: at(buf_at), ..run });
    };
    let mut run_start = 0usize;
    let mut run_style = None;
    for (i, _) in text.char_indices() {
        let mut st = styles_at(base + i);
        if caps.iter().any(|r| r.contains(&i)) {
            st |= Style::SmallCaps.bit();
        }
        match run_style {
            None => run_style = Some(st),
            Some(prev) if prev != st => {
                emit(run_start, i, prev);
                run_start = i;
                run_style = Some(st);
            }
            _ => {}
        }
    }
    if let Some(st) = run_style {
        emit(run_start, text.len(), st);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::BookId;

    /// Each run of verse text is its verse's text from the run's offset, as a highlight needs, give
    /// or take a space the layout added at its end and small capitals' case.
    fn check_offsets(t: &Translation, c: ChapterIdx, runs: &[&Run]) -> usize {
        let mut checked = 0;
        for r in runs.iter().filter(|r| r.verse > 0 && !r.number) {
            if r.added {
                assert_eq!(r.text, " ");
                continue;
            }
            let v = t.verses_in(c).find(|v| v.number == r.verse).unwrap();
            let text: Vec<u16> = v.text.encode_utf16().collect();
            let run: Vec<u16> = r.text.encode_utf16().collect();
            let start = r.offset as usize;
            let len = run.len().min(text.len() - start);
            let want = String::from_utf16(&text[start..start + len]).unwrap();
            let got = String::from_utf16(&run[..len]).unwrap();
            assert_eq!(got.to_uppercase(), want.to_uppercase(), "verse {} run {:?}", r.verse, r.text);
            assert_eq!(len, run.len(), "all of a run is its verse's text: {:?}", r.text);
            checked += 1;
        }
        checked
    }

    #[test]
    fn runs_know_where_they_are_in_their_verse() {
        let kjv = crate::bundled::load("kjv").unwrap();
        for (book, n) in [(BookId(42), 3), (BookId::PSALMS, 23), (BookId::PSALMS, 119), (BookId(39), 5)] {
            let c = ChapterIdx::new(book, n).unwrap();
            for opts in
                [ContentOptions::default(), ContentOptions { verse_per_line: true, ..Default::default() }]
            {
                let content = chapter(&kjv, c, opts);
                let runs: Vec<&Run> = content.blocks.iter().flat_map(|b| &b.runs).collect();
                assert!(check_offsets(&kjv, c, &runs) > 10);
            }
            for v in kjv.verses_in(c) {
                let content = verse(&kjv, &v);
                let runs: Vec<&Run> = content.runs.iter().collect();
                check_offsets(&kjv, c, &runs);
                // A whole verse's runs, minus added spaces, are its text.
                let joined: usize = runs.iter().map(|r| r.text.encode_utf16().count()).sum();
                assert!(joined >= v.text.encode_utf16().count());
            }
        }
    }
}
