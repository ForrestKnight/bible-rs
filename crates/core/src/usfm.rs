//! USFM (Unified Standard Format Markers) reader.
//!
//! Handles the subset that matters for reading: books, chapters, verses (including bridges),
//! paragraph and poetry structure, section headings, Psalm titles, and the character styles
//! words-of-Christ / translator additions / divine name / Selah. Footnotes, cross-references,
//! introductions, Strong's attributes and other study apparatus are dropped.

use anyhow::Result;

use crate::books::BookId;
use crate::model::{MarkKind, Style, TranslationBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ctx {
    /// Verse text.
    Body,
    /// Heading text collected until the next paragraph marker.
    Heading(MarkKind, u8),
    /// Book title fields.
    Title(TitleField),
    /// Material we drop (introductions, remarks, parallel-passage references, …).
    Ignore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TitleField {
    Short,
    Long,
    Main(u8),
}

/// Parses one USFM document (typically one book) into the builder.
pub struct UsfmParser<'b> {
    b: &'b mut TranslationBuilder,
    ctx: Ctx,
    skip_book: bool,
    buf: String,
    /// Depth of note-like containers (`\f`, `\x`, `\fe`, `\fig`) whose content is dropped.
    note: Option<String>,
    /// Inside `word|attributes` — drop until the closing marker.
    in_attrs: bool,
    /// Open character markers (to route `|` attributes and pop styles).
    char_stack: Vec<(String, Option<Style>)>,
    short: String,
    long: String,
    main_titles: Vec<(u8, String)>,
}

impl<'b> UsfmParser<'b> {
    pub fn new(b: &'b mut TranslationBuilder) -> Self {
        Self {
            b,
            ctx: Ctx::Ignore,
            skip_book: true,
            buf: String::new(),
            note: None,
            in_attrs: false,
            char_stack: Vec::new(),
            short: String::new(),
            long: String::new(),
            main_titles: Vec::new(),
        }
    }

    pub fn parse(mut self, src: &str) -> Result<()> {
        let src = src.strip_prefix('\u{feff}').unwrap_or(src);
        let bytes = src.as_bytes();
        let mut pos = 0usize;
        while pos < bytes.len() {
            let Some(rel) = memchr::memchr(b'\\', &bytes[pos..]) else {
                self.on_text(&src[pos..]);
                break;
            };
            let at = pos + rel;
            if at > pos {
                self.on_text(&src[pos..at]);
            }
            // Marker name: letters, digits and a leading '+' for nested character styles.
            let mut end = at + 1;
            while end < bytes.len()
                && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'+' || bytes[end] == b'-')
            {
                end += 1;
            }
            let name = &src[at + 1..end];
            if name.is_empty() {
                // A lone backslash: treat as text.
                self.on_text("\\");
                pos = at + 1;
                continue;
            }
            let closing = end < bytes.len() && bytes[end] == b'*';
            if closing {
                end += 1;
            } else if end < bytes.len() && (bytes[end] == b' ' || bytes[end] == b'\t') {
                end += 1;
            } else if end < bytes.len() && bytes[end] == b'\r' {
                end += 1;
                if end < bytes.len() && bytes[end] == b'\n' {
                    end += 1;
                }
            } else if end < bytes.len() && bytes[end] == b'\n' {
                end += 1;
            }
            pos = end;
            let name = name.trim_start_matches('+');
            if closing {
                self.on_close(name);
                continue;
            }
            // Markers that take a single word argument.
            if matches!(name, "id" | "c" | "v" | "ca" | "va" | "vp" | "cp") && self.note.is_none() {
                while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
                    pos += 1;
                }
                let s = pos;
                while pos < bytes.len() && !bytes[pos].is_ascii_whitespace() && bytes[pos] != b'\\' {
                    pos += 1;
                }
                let arg = &src[s..pos];
                if pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
                    pos += 1;
                }
                self.on_arg_marker(name, arg);
                continue;
            }
            self.on_open(name);
        }
        self.flush_ctx();
        self.flush_titles();
        Ok(())
    }

    fn on_text(&mut self, t: &str) {
        if self.note.is_some() || self.skip_book {
            return;
        }
        let t = if self.in_attrs {
            return;
        } else if !self.char_stack.is_empty() {
            match t.find('|') {
                Some(i) => {
                    self.in_attrs = true;
                    &t[..i]
                }
                None => t,
            }
        } else {
            t
        };
        match self.ctx {
            Ctx::Body => self.b.text(t),
            Ctx::Heading(..) => self.buf.push_str(t),
            Ctx::Title(_) => self.buf.push_str(t),
            Ctx::Ignore => {}
        }
    }

    fn on_arg_marker(&mut self, name: &str, arg: &str) {
        match name {
            "id" => {
                self.flush_ctx();
                self.flush_titles();
                match BookId::from_code(arg) {
                    Some(book) => {
                        self.skip_book = false;
                        self.b.begin_book(book);
                    }
                    None => self.skip_book = true,
                }
                self.ctx = Ctx::Ignore;
            }
            _ if self.skip_book => {}
            "c" => {
                self.flush_ctx();
                self.flush_titles();
                self.b.clear_styles();
                self.char_stack.clear();
                self.in_attrs = false;
                let n = leading_number(arg).unwrap_or(0);
                self.b.begin_chapter(n);
                self.ctx = Ctx::Body;
            }
            "v" => {
                if !matches!(self.ctx, Ctx::Body) {
                    self.flush_ctx();
                    self.ctx = Ctx::Body;
                }
                let (start, end) = verse_range(arg);
                if start > 0 {
                    self.b.begin_verse(start, end);
                }
            }
            // `\vp …\vp*` (published verse number) has content to drop; the others are bare.
            "vp" => self.note = Some("vp".into()),
            _ => {}
        }
    }

    fn on_open(&mut self, name: &str) {
        if let Some(_note) = &self.note {
            return;
        }
        if matches!(name, "f" | "fe" | "x" | "fig" | "ef" | "ex") {
            self.note = Some(name.to_string());
            return;
        }
        if self.skip_book {
            return;
        }
        if let Some(style) = char_style(name) {
            self.char_stack.push((name.to_string(), style));
            if let (Some(style), Ctx::Body) = (style, self.ctx) {
                self.b.push_style(style);
            }
            return;
        }
        if is_char_marker(name) {
            self.char_stack.push((name.to_string(), None));
            return;
        }
        // Paragraph-level marker: whatever we were collecting is complete.
        self.flush_ctx();
        self.ctx = match para_kind(name) {
            Para::Break(kind, level) => {
                self.b.mark(kind, level);
                Ctx::Body
            }
            Para::Body => Ctx::Body,
            Para::Heading(kind, level) => Ctx::Heading(kind, level),
            Para::Title(f) => Ctx::Title(f),
            Para::Ignore => Ctx::Ignore,
        };
        self.buf.clear();
    }

    fn on_close(&mut self, name: &str) {
        if let Some(note) = &self.note {
            if note == name {
                self.note = None;
            }
            return;
        }
        if self.skip_book {
            return;
        }
        self.in_attrs = false;
        if let Some(i) = self.char_stack.iter().rposition(|(n, _)| n == name) {
            let (_, style) = self.char_stack.remove(i);
            if let (Some(style), Ctx::Body) = (style, self.ctx) {
                self.b.pop_style(style);
            }
        }
    }

    fn flush_ctx(&mut self) {
        match self.ctx {
            Ctx::Heading(kind, level) => {
                let text = std::mem::take(&mut self.buf);
                self.b.heading(kind, level, &text);
            }
            Ctx::Title(field) => {
                let text = std::mem::take(&mut self.buf).trim().to_string();
                if !text.is_empty() {
                    match field {
                        TitleField::Short if self.short.is_empty() => self.short = text,
                        TitleField::Long => self.long = text,
                        TitleField::Main(l) => self.main_titles.push((l, text)),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        self.buf.clear();
    }

    fn flush_titles(&mut self) {
        if let Some(book) = self.b.current_book().filter(|_| !self.skip_book) {
            let mut long = std::mem::take(&mut self.long);
            if long.is_empty() && !self.main_titles.is_empty() {
                // `\mt2 The Good News According to \mt1 John` reads top to bottom.
                long = self.main_titles.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>().join(" ");
            }
            let long = smart_title_case(&long);
            let short = std::mem::take(&mut self.short);
            self.b.set_book_title(book, Some(&short), Some(&long));
        }
        self.main_titles.clear();
        self.short.clear();
        self.long.clear();
    }
}

/// What a paragraph-level USFM marker (or API.Bible `para` style) means for the reader.
pub(crate) enum Para {
    /// A line break with this mark, followed by verse text.
    Break(MarkKind, u8),
    /// Verse text continuing without a break (`\nb`).
    Body,
    /// Heading text until the next paragraph marker.
    Heading(MarkKind, u8),
    Title(TitleField),
    Ignore,
}

pub(crate) fn para_kind(name: &str) -> Para {
    let (base, level) = split_level(name);
    match base {
        "p" | "pi" | "ph" | "lh" | "lf" | "tr" => Para::Break(MarkKind::Paragraph, 0),
        "m" | "mi" | "pmo" | "pm" | "pmc" | "po" | "lim" => Para::Break(MarkKind::ParagraphFlush, 0),
        "nb" => Para::Body,
        "pc" | "qc" => Para::Break(MarkKind::Centered, 0),
        "pr" | "qr" | "pmr" | "cls" => Para::Break(MarkKind::PoetryRight, 0),
        "q" | "qm" | "qd" => Para::Break(MarkKind::Poetry, level.unwrap_or(1).clamp(1, 4)),
        "li" => Para::Break(MarkKind::List, level.unwrap_or(1).clamp(1, 4)),
        "b" => Para::Break(MarkKind::Blank, 0),
        "s" => Para::Heading(MarkKind::Heading, level.unwrap_or(1).clamp(1, 2)),
        "ms" => Para::Heading(MarkKind::Heading, 0),
        "qa" => Para::Heading(MarkKind::Heading, 3),
        "sp" => Para::Heading(MarkKind::Speaker, 0),
        "d" => Para::Heading(MarkKind::PsalmTitle, 0),
        "toc" if level == Some(1) => Para::Title(TitleField::Long),
        "toc" if level == Some(2) => Para::Title(TitleField::Short),
        "h" => Para::Title(TitleField::Short),
        "mt" => Para::Title(TitleField::Main(level.unwrap_or(1))),
        _ => Para::Ignore,
    }
}

/// Character style markers we render.
pub(crate) fn char_style(name: &str) -> Option<Option<Style>> {
    Some(match name {
        "wj" => Some(Style::WordsOfJesus),
        "add" => Some(Style::Added),
        "nd" => Some(Style::DivineName),
        "qs" => Some(Style::Selah),
        "it" | "em" | "tl" | "bk" | "k" | "qt" | "sls" | "dc" | "rq" => Some(Style::Italic),
        "bd" | "bdit" => Some(Style::Bold),
        "sc" => Some(Style::SmallCaps),
        _ => return None,
    })
}

fn is_char_marker(name: &str) -> bool {
    matches!(
        name,
        "w" | "wh"
            | "wg"
            | "wa"
            | "rb"
            | "jmp"
            | "pro"
            | "ord"
            | "no"
            | "sup"
            | "pn"
            | "png"
            | "addpn"
            | "ref"
            | "lik"
            | "liv"
            | "litl"
            | "xt"
            | "fv"
            | "qac"
    )
}

fn split_level(name: &str) -> (&str, Option<u8>) {
    let digits = name.bytes().rev().take_while(u8::is_ascii_digit).count();
    if digits == 0 || digits == name.len() {
        return (name, None);
    }
    let (base, num) = name.split_at(name.len() - digits);
    (base, num.parse().ok())
}

fn leading_number(s: &str) -> Option<u32> {
    let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// "16" → (16,16); "3-4" → (3,4); "12a" → (12,12).
pub(crate) fn verse_range(s: &str) -> (u32, u32) {
    let start = leading_number(s).unwrap_or(0);
    let end =
        s.split(['-', '–', ',']).nth(1).and_then(leading_number).filter(|&e| e >= start).unwrap_or(start);
    (start, end)
}

/// "THE GOSPEL ACCORDING TO ST. JOHN" → "The Gospel According to St. John". Mixed-case input is
/// returned unchanged.
pub fn smart_title_case(s: &str) -> String {
    let letters: Vec<char> = s.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() || letters.iter().any(|c| c.is_lowercase()) {
        return s.to_string();
    }
    const SMALL: &[&str] = &["of", "the", "to", "and", "a", "an", "in", "unto", "by", "for", "on"];
    let mut out = String::with_capacity(s.len());
    for (i, word) in s.split_whitespace().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let lower = word.to_lowercase();
        let bare = lower.trim_matches(|c: char| !c.is_alphanumeric());
        if i > 0 && SMALL.contains(&bare) {
            out.push_str(&lower);
        } else {
            let mut cs = lower.chars();
            if let Some(f) = cs.next() {
                out.extend(f.to_uppercase());
                out.push_str(cs.as_str());
            }
        }
    }
    out
}

/// Convenience: parse a set of USFM documents into one builder.
pub fn parse_documents<'a>(
    b: &mut TranslationBuilder,
    docs: impl IntoIterator<Item = &'a str>,
) -> Result<()> {
    for doc in docs {
        UsfmParser::new(b).parse(doc)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::{ChapterIdx, VerseRef};
    use crate::model::{Span, TranslationMeta};

    const SAMPLE: &str = r#"\id JHN Test
\h John
\toc1 THE GOSPEL ACCORDING TO ST. JOHN
\toc2 John
\mt1 THE GOSPEL ACCORDING TO ST. JOHN
\c 3
\s1 Jesus and Nicodemus
\r (Some ref)
\p
\v 16 \w For|strong="G1063"\w* God so loved\f + \fr 3:16 \ft note\f* the world. \wj “\+w Truly|strong="x"\+w* I say.”\wj*
\v 17-18 \add bridged\add* verse.
\c 4
\q1
\v 1 The \nd LORD\nd* is my shepherd;
\q2 I shall not want. \qs Selah.\qs*
\b
\q1
\v 2 He makes me lie down.
"#;

    #[test]
    fn parses_structure_styles_and_drops_apparatus() {
        let mut b = TranslationBuilder::new(TranslationMeta::default());
        UsfmParser::new(&mut b).parse(SAMPLE).unwrap();
        let t = b.finish();
        let john = BookId::from_code("JHN").unwrap();
        assert_eq!(t.meta.books[john.index()].long, "The Gospel According to St. John");
        assert_eq!(t.meta.books[john.index()].short, "John");

        let v16 = t.find(VerseRef::new(john, 3, 16)).unwrap();
        assert_eq!(v16.text, "For God so loved the world. “Truly I say.”");
        assert_eq!(v16.spans, &[Span { start: 28, end: 46, style: Style::WordsOfJesus }]);
        assert_eq!(
            v16.marks.iter().map(|m| m.kind).collect::<Vec<_>>(),
            [MarkKind::Heading, MarkKind::Paragraph]
        );
        assert_eq!(t.mark_text(&v16.marks[0]), "Jesus and Nicodemus");

        let v17 = t.find(VerseRef::new(john, 3, 18)).unwrap();
        assert_eq!((v17.number, v17.number_end), (17, 18));
        assert_eq!(v17.text, "bridged verse.");

        let c4 = ChapterIdx::new(john, 4).unwrap();
        let vs: Vec<_> = t.verses_in(c4).collect();
        assert_eq!(vs[0].text, "The LORD is my shepherd;I shall not want. Selah.");
        assert_eq!(vs[0].plain_text(), "The LORD is my shepherd; I shall not want. Selah.");
        let kinds: Vec<_> = vs[0].marks.iter().map(|m| (m.kind, m.level, m.offset)).collect();
        assert_eq!(kinds, [(MarkKind::Poetry, 1, 0), (MarkKind::Poetry, 2, 24)]);
        let kinds: Vec<_> = vs[1].marks.iter().map(|m| (m.kind, m.level)).collect();
        assert_eq!(kinds, [(MarkKind::Blank, 0), (MarkKind::Poetry, 1)]);
        assert!(vs[0].spans.iter().any(|s| s.style == Style::Selah));
    }

    #[test]
    fn skips_non_canonical_books() {
        let mut b = TranslationBuilder::new(TranslationMeta::default());
        UsfmParser::new(&mut b).parse("\\id TOB\n\\c 1\n\\p\n\\v 1 Tobit text").unwrap();
        UsfmParser::new(&mut b).parse("\\id FRT\n\\p Front matter").unwrap();
        assert_eq!(b.finish().verse_count(), 0);
    }

    #[test]
    fn verse_ranges() {
        assert_eq!(verse_range("16"), (16, 16));
        assert_eq!(verse_range("3-4"), (3, 4));
        assert_eq!(verse_range("12a"), (12, 12));
        assert_eq!(verse_range("x"), (0, 0));
    }

    #[test]
    fn title_case() {
        assert_eq!(
            smart_title_case("THE FIRST BOOK OF MOSES, CALLED GENESIS"),
            "The First Book of Moses, Called Genesis"
        );
        assert_eq!(smart_title_case("The Good News"), "The Good News");
    }
}
