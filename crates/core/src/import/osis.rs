//! OSIS XML reader — whole documents (`<verse osisID=… sID=…/>` milestones or containers) and
//! the per-verse OSIS fragments stored in SWORD modules.

use std::collections::HashMap;

use crate::books::BookId;
use crate::model::{MarkKind, Style, TranslationBuilder};
use crate::xml::{Tok, attr, tokens};

/// Metadata gathered from `<header><work>`.
#[derive(Debug, Default, Clone)]
pub struct OsisWork {
    pub title: String,
    pub abbrev: String,
    pub rights: String,
    pub language: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Collect {
    Title(MarkKind, u8),
    BookTitle,
    WorkTitle,
    WorkAbbrev,
    Rights,
}

/// What an open container element did, so its end tag can undo it.
#[derive(Debug, Clone)]
enum Effect {
    None,
    Style(Style),
    Skip,
    Verse,
    Collect,
}

pub struct OsisHandler<'b> {
    pub b: &'b mut TranslationBuilder,
    /// Fragment mode (SWORD): book/chapter/verse come from the caller; structure tags are ignored.
    fragment: bool,
    stack: Vec<(String, Effect)>,
    skip: usize,
    collect: Option<(Collect, String)>,
    milestones: HashMap<String, Style>,
    skip_book: bool,
    in_lg: bool,
    pub work: OsisWork,
}

impl<'b> OsisHandler<'b> {
    pub fn new(b: &'b mut TranslationBuilder, fragment: bool) -> Self {
        Self {
            b,
            fragment,
            stack: Vec::new(),
            skip: 0,
            collect: None,
            milestones: HashMap::new(),
            skip_book: false,
            in_lg: false,
            work: OsisWork::default(),
        }
    }

    pub fn feed(&mut self, src: &str) {
        for tok in tokens(src) {
            match tok {
                Tok::Text(t) => self.text(&t),
                Tok::Start { name, attrs } => {
                    let effect = self.open(name, attrs, false);
                    self.stack.push((name.to_string(), effect));
                }
                Tok::Empty { name, attrs } => {
                    let effect = self.open(name, attrs, true);
                    self.close_effect(effect);
                }
                Tok::End { name } => {
                    if let Some(i) = self.stack.iter().rposition(|(n, _)| n == name) {
                        // Close everything opened inside it as well (tolerates sloppy nesting).
                        while self.stack.len() > i {
                            let (_, e) = self.stack.pop().unwrap();
                            self.close_effect(e);
                        }
                    }
                    if name == "lg" {
                        self.in_lg = false;
                    }
                }
            }
        }
    }

    /// Ends a SWORD verse fragment: anything still open stays open only for styles.
    pub fn end_fragment(&mut self) {
        self.collect = None;
        self.skip = 0;
        self.stack.retain(|(_, e)| matches!(e, Effect::Style(_)));
    }

    fn text(&mut self, t: &str) {
        if self.skip > 0 || self.skip_book {
            return;
        }
        if let Some((_, buf)) = &mut self.collect {
            buf.push_str(t);
            return;
        }
        self.b.text(t);
    }

    fn open(&mut self, name: &str, attrs: &str, empty: bool) -> Effect {
        if self.skip > 0 {
            if !empty {
                self.skip += 1;
                return Effect::Skip;
            }
            return Effect::None;
        }
        let a = |k: &str| attr(attrs, k);
        let ty = a("type").map(|t| t.to_ascii_lowercase()).unwrap_or_default();
        let milestone_start = a("sID").is_some();
        let milestone_end = a("eID");
        match name {
            "note" | "rdg" | "figure" | "index" | "catchWord" => {
                if empty {
                    Effect::None
                } else {
                    self.skip = 1;
                    Effect::Skip
                }
            }
            "header" | "work" => Effect::None,
            "title" if self.in_header() => self.start_collect(Collect::WorkTitle, empty),
            "abbreviation" if self.in_header() => self.start_collect(Collect::WorkAbbrev, empty),
            "rights" if self.in_header() => self.start_collect(Collect::Rights, empty),
            _ if self.in_header() => Effect::None,
            "div" if !self.fragment => {
                if ty == "book" {
                    let id = a("osisID").unwrap_or_default();
                    match BookId::from_osis(id.split('.').next().unwrap_or("")) {
                        Some(book) => {
                            self.skip_book = false;
                            self.b.begin_book(book);
                        }
                        None => self.skip_book = true,
                    }
                }
                Effect::None
            }
            "chapter" if !self.fragment => {
                if milestone_end.is_none() {
                    if let Some((book, ch, _)) = a("osisID").as_deref().and_then(parse_osis_ref) {
                        if self.b.current_book() != Some(book) {
                            self.skip_book = false;
                            self.b.begin_book(book);
                        }
                        self.b.begin_chapter(ch);
                    } else if let Some(n) = a("n").and_then(|n| n.parse().ok()) {
                        self.b.begin_chapter(n);
                    }
                }
                Effect::None
            }
            "verse" if !self.fragment => {
                if milestone_end.is_some() {
                    self.b.end_current_verse();
                    return Effect::None;
                }
                let id = a("osisID").unwrap_or_default();
                let mut ids = id.split_whitespace();
                let first = ids.next().and_then(parse_osis_ref);
                let last = ids.last().and_then(parse_osis_ref);
                if let Some((book, ch, v)) = first {
                    if self.b.current_book() != Some(book) {
                        self.skip_book = false;
                        self.b.begin_book(book);
                    }
                    if self.b.current_chapter().map(|c| c.chapter() as u32) != Some(ch)
                        || self.b.current_chapter().map(|c| c.book()) != Some(book)
                    {
                        self.b.begin_chapter(ch);
                    }
                    let end = last.map_or(v, |(_, _, e)| e);
                    self.b.begin_verse(v, end);
                }
                if empty || milestone_start { Effect::None } else { Effect::Verse }
            }
            "title" => {
                let kind = match ty.as_str() {
                    "psalm" => Collect::Title(MarkKind::PsalmTitle, 0),
                    "main" => Collect::BookTitle,
                    "chapter" | "x-chapter-label" | "parallel" | "scope" => {
                        if empty {
                            return Effect::None;
                        }
                        self.skip = 1;
                        return Effect::Skip;
                    }
                    "acrostic" => Collect::Title(MarkKind::Heading, 3),
                    _ if a("canonical").as_deref() == Some("true") => Collect::Title(MarkKind::PsalmTitle, 0),
                    _ => Collect::Title(MarkKind::Heading, if ty == "major" { 0 } else { 1 }),
                };
                self.start_collect(kind, empty)
            }
            "speaker" => self.start_collect(Collect::Title(MarkKind::Speaker, 0), empty),
            "p" => {
                self.b.mark(MarkKind::Paragraph, 0);
                Effect::None
            }
            "milestone" => {
                if matches!(ty.as_str(), "x-p" | "pilcrow" | "x-paragraph" | "x-pilcrow") {
                    self.b.mark(MarkKind::Paragraph, 0);
                }
                Effect::None
            }
            "lg" => {
                if milestone_end.is_none() {
                    self.in_lg = true;
                }
                Effect::None
            }
            "l" => {
                if milestone_end.is_none() {
                    let level = a("level").and_then(|l| l.parse().ok()).unwrap_or(1u8).clamp(1, 4);
                    match ty.as_str() {
                        "selah" | "x-selah" => self.b.mark(MarkKind::PoetryRight, 0),
                        _ => self.b.mark(MarkKind::Poetry, level),
                    }
                }
                Effect::None
            }
            "lb" => {
                if self.in_lg {
                    self.b.mark(MarkKind::Poetry, 1);
                } else if !ty.contains("end") {
                    self.b.mark(MarkKind::ParagraphFlush, 0);
                }
                Effect::None
            }
            "item" => {
                self.b
                    .mark(MarkKind::List, a("level").and_then(|l| l.parse().ok()).unwrap_or(1u8).clamp(1, 4));
                Effect::None
            }
            "q" => {
                let jesus = a("who").is_some_and(|w| w.eq_ignore_ascii_case("jesus"));
                if let Some(id) = milestone_end {
                    if let Some(style) = self.milestones.remove(id.as_ref()) {
                        self.b.pop_style(style);
                    }
                    return Effect::None;
                }
                if jesus {
                    self.b.push_style(Style::WordsOfJesus);
                    if let Some(sid) = a("sID") {
                        self.milestones.insert(sid.into_owned(), Style::WordsOfJesus);
                        return Effect::None;
                    }
                    if empty {
                        self.b.pop_style(Style::WordsOfJesus);
                        return Effect::None;
                    }
                    return Effect::Style(Style::WordsOfJesus);
                }
                Effect::None
            }
            "transChange" if ty == "added" => self.style(Style::Added, empty),
            "divineName" => self.style(Style::DivineName, empty),
            "hi" => match ty.as_str() {
                "italic" | "emphasis" => self.style(Style::Italic, empty),
                "bold" => self.style(Style::Bold, empty),
                "small-caps" | "x-small-caps" => self.style(Style::SmallCaps, empty),
                _ => Effect::None,
            },
            "foreign" | "w" | "seg" | "name" | "abbr" | "transChange" | "inscription" | "a" => Effect::None,
            "head" if !self.fragment => self.start_collect(Collect::BookTitle, empty),
            _ => Effect::None,
        }
    }

    fn in_header(&self) -> bool {
        self.stack.iter().any(|(n, _)| n == "header")
    }

    fn style(&mut self, s: Style, empty: bool) -> Effect {
        if empty {
            return Effect::None;
        }
        self.b.push_style(s);
        Effect::Style(s)
    }

    fn start_collect(&mut self, c: Collect, empty: bool) -> Effect {
        if empty || self.collect.is_some() {
            return Effect::None;
        }
        self.collect = Some((c, String::new()));
        Effect::Collect
    }

    fn close_effect(&mut self, e: Effect) {
        match e {
            Effect::None => {}
            Effect::Style(s) => self.b.pop_style(s),
            Effect::Skip => self.skip = self.skip.saturating_sub(1),
            Effect::Verse => self.b.end_current_verse(),
            Effect::Collect => {
                if let Some((c, text)) = self.collect.take() {
                    let text = text.trim();
                    match c {
                        Collect::Title(kind, level) => self.b.heading(kind, level, text),
                        Collect::BookTitle => {
                            if let Some(book) = self.b.current_book() {
                                self.b.set_book_title(book, None, Some(text));
                            }
                        }
                        Collect::WorkTitle if self.work.title.is_empty() => {
                            self.work.title = text.to_string()
                        }
                        Collect::WorkAbbrev if self.work.abbrev.is_empty() => {
                            self.work.abbrev = text.to_string()
                        }
                        Collect::Rights if self.work.rights.is_empty() => self.work.rights = text.to_string(),
                        _ => {}
                    }
                }
            }
        }
    }
}

/// "Gen.1.1" → (Genesis, 1, 1); "Ps.23" → (Psalms, 23, 0).
pub fn parse_osis_ref(s: &str) -> Option<(BookId, u32, u32)> {
    let s = s.split('!').next()?;
    let mut parts = s.split('.');
    let book = BookId::from_osis(parts.next()?)?;
    let ch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let v = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    Some((book, ch, v))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::{ChapterIdx, VerseRef};
    use crate::model::{Span, TranslationMeta};

    const DOC: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<osis xmlns="http://www.bibletechnologies.net/2003/OSIS/namespace"><osisText osisIDWork="TEST">
<header><work osisWork="TEST"><title>Test Bible</title><rights>Public domain test.</rights></work></header>
<div type="book" osisID="John"><title type="main">The Gospel of John</title>
<chapter osisID="John.3" sID="John.3"/>
<title>Jesus and Nicodemus</title>
<p><verse osisID="John.3.16" sID="v16"/>For God so loved<note>a note</note> the world, <q who="Jesus" marker="" sID="q1"/>that he gave<verse eID="v16"/>
<verse osisID="John.3.17" sID="v17"/>his <transChange type="added">only</transChange> Son.<q eID="q1" marker=""/><verse eID="v17"/></p>
<chapter eID="John.3"/></div>
<div type="book" osisID="Ps"><chapter osisID="Ps.23">
<title type="psalm" canonical="true">A Psalm of David.</title>
<lg><l level="1"><verse osisID="Ps.23.1">The <divineName>Lord</divineName> is my shepherd;</verse></l>
<l level="2"><verse osisID="Ps.23.2 Ps.23.3">I shall not want.</verse></l></lg></chapter></div>
<div type="book" osisID="Tob"><chapter osisID="Tob.1"><verse osisID="Tob.1.1">Apocrypha</verse></chapter></div>
</osisText></osis>"#;

    #[test]
    fn reads_milestones_containers_and_styles() {
        let mut b = TranslationBuilder::new(TranslationMeta::default());
        let mut h = OsisHandler::new(&mut b, false);
        h.feed(DOC);
        let work = h.work.clone();
        let t = b.finish();
        assert_eq!(work.title, "Test Bible");
        assert_eq!(work.rights, "Public domain test.");
        let john = BookId(42);
        assert_eq!(t.meta.books[42].long, "The Gospel of John");
        let v16 = t.find(VerseRef::new(john, 3, 16)).unwrap();
        assert_eq!(v16.text, "For God so loved the world, that he gave");
        assert_eq!(
            v16.marks.iter().map(|m| m.kind).collect::<Vec<_>>(),
            [MarkKind::Heading, MarkKind::Paragraph]
        );
        assert_eq!(v16.spans, &[Span { start: 28, end: 40, style: Style::WordsOfJesus }]);
        let v17 = t.find(VerseRef::new(john, 3, 17)).unwrap();
        assert_eq!(v17.text, "his only Son.");
        assert!(v17.spans.contains(&Span { start: 0, end: 13, style: Style::WordsOfJesus }));
        assert!(v17.spans.contains(&Span { start: 4, end: 8, style: Style::Added }));
        let ps = ChapterIdx::new(BookId::PSALMS, 23).unwrap();
        let vs: Vec<_> = t.verses_in(ps).collect();
        assert_eq!(vs.len(), 2);
        assert_eq!(vs[0].marks[0].kind, MarkKind::PsalmTitle);
        assert_eq!((vs[1].number, vs[1].number_end), (2, 3));
        assert_eq!(vs[1].marks[0].level, 2);
        assert_eq!(vs[0].spans[0].style, Style::DivineName);
        assert_eq!(t.verse_count(), 4, "deuterocanonical books are skipped");
    }
}
