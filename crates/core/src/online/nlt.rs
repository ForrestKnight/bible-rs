//! NLT API (api.nlt.to): the New Living Translation, © Tyndale House Foundation.
//!
//! Tyndale serves the NLT to non-commercial applications without a key (no more than 50 verses a
//! request and 500 requests a day, counted per IP address) or with a free key (500 verses and
//! 5,000 requests). Chapters longer than a request allows are fetched in consecutive ranges.
//!
//! Passages come back as an HTML excerpt: each verse in a `<verse_export vn="…">` element,
//! paragraphs and poetry lines as `<p class="body | poet1-vn | poet2 | …">` (a `-sp` suffix adds
//! space before), headings as `<h3 class="subhead">`, the words of Christ in `<span class="red">`,
//! the divine name in `<span class="sc">`, and translator notes in `<span class="tn">`, which are
//! left out.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::Result;

use super::{BookShare, CachePolicy, DailyBudget, FetchedChapter, Source, http_agent, read_body};
use crate::books::{BookId, ChapterIdx};
use crate::model::{MarkKind, Style, Translation, TranslationBuilder, TranslationMeta};
use crate::xml::{self, Tok};

/// The credit line Tyndale's permission terms require with NLT text (shown with every chapter).
pub const COPYRIGHT: &str = "Scripture quotations are taken from the Holy Bible, New Living Translation, \
copyright ©1996, 2004, 2015 by Tyndale House Foundation. Used by permission of Tyndale House Publishers, \
Carol Stream, Illinois 60188. All rights reserved.";

pub const DEFAULT_BASE: &str = "https://api.nlt.to";

/// The API's terms: (verses per request, requests per day) without and with a key.
pub const ANONYMOUS_LIMITS: (u32, u32) = (50, 500);
pub const KEY_LIMITS: (u32, u32) = (500, 5_000);

pub struct Nlt {
    base: String,
    agent: ureq::Agent,
    budget: DailyBudget,
    /// Every request, including each range of a long chapter, waits this long after the last.
    interval: Duration,
    last_request: Mutex<Option<Instant>>,
}

impl Nlt {
    /// `budget_path` keeps today's request count across sessions.
    pub fn new(budget_path: Option<PathBuf>) -> Self {
        Self::with_base(DEFAULT_BASE, budget_path)
    }

    pub fn with_base(base: &str, budget_path: Option<PathBuf>) -> Self {
        Self {
            base: base.trim_end_matches('/').to_string(),
            agent: http_agent(),
            budget: DailyBudget::new(budget_path),
            interval: Duration::from_secs(1),
            last_request: Mutex::new(None),
        }
    }

    /// Spaces requests differently (tests use zero).
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    pub fn requests_today(&self) -> u32 {
        self.budget.used()
    }

    fn passage(&self, key: &str, reference: &str, per_day: u32) -> Result<String> {
        if let Err(e) = self.budget.spend(per_day, "NLT API") {
            anyhow::ensure!(!key.is_empty(), "{e}. A free API key from api.nlt.to raises it to 5,000");
            return Err(e);
        }
        {
            let mut last = self.last_request.lock().unwrap();
            if let Some(t) = *last {
                std::thread::sleep(self.interval.saturating_sub(t.elapsed()));
            }
            *last = Some(Instant::now());
        }
        // `key=TEST` is the API's documented anonymous access.
        let key = if key.is_empty() { "TEST".to_string() } else { url_encode(key) };
        let url = format!("{}/api/passages?ref={reference}&version=NLT&key={key}", self.base);
        read_body(self.agent.get(&url).call(), "NLT API")
    }
}

impl Source for Nlt {
    fn policy(&self) -> CachePolicy {
        CachePolicy {
            // Tyndale's permission statement: up to 500 verses, and never a complete book.
            max_verses: 500,
            book_share: BookShare::LessThanWhole,
            max_age_secs: 30 * 86_400,
            // `passage` spaces every request itself (a long chapter takes several).
            min_request_interval: Duration::ZERO,
        }
    }

    fn needs_key(&self) -> bool {
        false
    }

    fn fetch(&self, key: &str, c: ChapterIdx) -> Result<FetchedChapter> {
        let (per_request, per_day) = if key.is_empty() { ANONYMOUS_LIMITS } else { KEY_LIMITS };
        let mut b = TranslationBuilder::new(TranslationMeta { id: "nlt".into(), ..Default::default() });
        b.begin_book(c.book());
        b.begin_chapter(c.chapter() as u32);
        let kjv = u32::from(c.kjv_verses());
        let book = api_book(c.book());
        let mut start = 1u32;
        let mut full = false;
        // Ranges cover the chapter's KJV verse count. The NLT sometimes numbers one more verse
        // (3 John has 15, the KJV 14), which the last range picks up. If that range came back full,
        // one more is asked for. Ranges past the end are clamped by the API.
        while start <= kjv || (full && start <= kjv + per_request) {
            let end = start + per_request - 1;
            let html = self.passage(key, &format!("{book}.{}.{start}-{end}", c.chapter()), per_day)?;
            let got = parse_into(&mut b, &html);
            full = got >= per_request as usize;
            start = end + 1;
        }
        let chapter = b.finish();
        anyhow::ensure!(chapter.verse_count() > 0, "NLT API: no text came back for {}", c.display());
        Ok(FetchedChapter { chapter, copyright: Some(COPYRIGHT.into()), fums_token: None })
    }
}

/// The API's name for a book: OSIS, except five it spells its own way (checked against its
/// `/api/parse` endpoint for all 66 books).
pub fn api_book(b: BookId) -> &'static str {
    match b.info().osis {
        "1Thess" => "1Thes",
        "2Thess" => "2Thes",
        "1John" => "1Jn",
        "2John" => "2Jn",
        "3John" => "3Jn",
        osis => osis,
    }
}

fn url_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Parses one HTML excerpt into a chapter (tests and tools).
pub fn parse_html(c: ChapterIdx, html: &str) -> Translation {
    let mut b = TranslationBuilder::new(TranslationMeta { id: "nlt".into(), ..Default::default() });
    b.begin_book(c.book());
    b.begin_chapter(c.chapter() as u32);
    parse_into(&mut b, html);
    b.finish()
}

/// An inline element left open: what it changed, so its end tag can undo it.
struct Inline<'a> {
    name: &'a str,
    skip: bool,
    caps: bool,
    styles: &'static [Style],
    /// The styles were pushed on the builder (not inside left-out text or a heading).
    applied: bool,
}

/// A heading or psalm title being collected up to its end tag. Without a mark (the chapter
/// number, the passage header) it is dropped.
struct Capture<'a> {
    mark: Option<(MarkKind, u8)>,
    end: &'a str,
    text: String,
}

/// Feeds one excerpt into `b` (its book and chapter already begun); returns how many verses it
/// held. Text outside `<verse_export>` elements (the page's title, the passage header) is
/// ignored, so consecutive ranges of a chapter can be fed one after another.
pub fn parse_into(b: &mut TranslationBuilder, html: &str) -> usize {
    let toks: Vec<Tok<'_>> = xml::tokens(html).collect();
    let mut verses = 0;
    let mut in_verse = false;
    let mut inline: Vec<Inline<'_>> = Vec::new();
    let mut skip = 0usize;
    let mut caps = 0usize;
    let mut capture: Option<Capture<'_>> = None;
    let mut selah = false;
    for (i, tok) in toks.iter().enumerate() {
        match tok {
            Tok::Start { name: "verse_export", attrs } => {
                let (start, end) =
                    verse_number(xml::attr(attrs, "vn").as_deref().unwrap_or(""), &toks[i + 1..]);
                if start > 0 {
                    b.begin_verse(start, end);
                    verses += 1;
                    in_verse = true;
                }
            }
            Tok::End { name: "verse_export" } => {
                b.end_current_verse();
                in_verse = false;
            }
            _ if !in_verse => {}
            Tok::Start { name, attrs } | Tok::Empty { name, attrs } => {
                let class = xml::attr(attrs, "class").unwrap_or_default();
                let empty = matches!(tok, Tok::Empty { .. });
                match *name {
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" if capture.is_none() && !empty => {
                        capture =
                            Some(Capture { mark: heading_kind(&class), end: name, text: String::new() });
                    }
                    "p" if capture.is_none() && !empty => {
                        if class == "psa-title" {
                            let mark = Some((MarkKind::PsalmTitle, 0));
                            capture = Some(Capture { mark, end: "p", text: String::new() });
                        } else if let Some((kind, level, space_before)) = para_kind(&class) {
                            if space_before {
                                b.mark(MarkKind::Blank, 0);
                            }
                            b.mark(kind, level);
                            if kind == MarkKind::PoetryRight {
                                b.push_style(Style::Selah);
                                selah = true;
                            }
                        }
                    }
                    "span" | "a" | "em" | "i" if !empty => {
                        let (skip_it, caps_it, styles): (bool, bool, &'static [Style]) =
                            match (*name, &*class) {
                                ("span", "vn" | "tn" | "tn-ref" | "cw" | "cw_ch") | ("a", "a-tn") => {
                                    (true, false, &[])
                                }
                                ("span", "red") => (false, false, &[Style::WordsOfJesus]),
                                ("span", "sc" | "subhead-sc" | "tn-sc") => {
                                    (false, true, &[Style::DivineName])
                                }
                                ("span", "red-sc") => {
                                    (false, true, &[Style::WordsOfJesus, Style::DivineName])
                                }
                                ("em" | "i", _) => (false, false, &[Style::Italic]),
                                _ => (false, false, &[]),
                            };
                        let applied = skip == 0 && capture.is_none() && !skip_it;
                        if applied {
                            styles.iter().for_each(|s| b.push_style(*s));
                        }
                        skip += usize::from(skip_it);
                        caps += usize::from(caps_it);
                        inline.push(Inline { name, skip: skip_it, caps: caps_it, styles, applied });
                    }
                    _ => {}
                }
            }
            Tok::End { name } => {
                if capture.as_ref().is_some_and(|c| c.end == *name) {
                    if let Some(Capture { mark: Some((kind, level)), text, .. }) = capture.take() {
                        b.heading(kind, level, &text);
                    }
                    continue;
                }
                if *name == "p" && selah {
                    b.pop_style(Style::Selah);
                    selah = false;
                }
                if let Some(at) = inline.iter().rposition(|e| e.name == *name) {
                    // Anything opened inside and never closed ends here too.
                    for e in inline.drain(at..).rev() {
                        skip -= usize::from(e.skip);
                        caps -= usize::from(e.caps);
                        if e.applied {
                            e.styles.iter().rev().for_each(|s| b.pop_style(*s));
                        }
                    }
                }
            }
            Tok::Text(t) => {
                if skip > 0 {
                    continue;
                }
                let t = if caps > 0 { std::borrow::Cow::Owned(t.to_uppercase()) } else { t.clone() };
                match &mut capture {
                    Some(c) => c.text.push_str(&t),
                    None => b.text(&t),
                }
            }
        }
    }
    if selah {
        b.pop_style(Style::Selah);
    }
    for e in inline.iter().rev().filter(|e| e.applied) {
        e.styles.iter().rev().for_each(|s| b.pop_style(*s));
    }
    verses
}

/// A verse's number range: the element's `vn`, widened by a bridged label ("7-8") in its
/// `<span class="vn">`.
fn verse_number(vn: &str, rest: &[Tok<'_>]) -> (u32, u32) {
    let (start, end) = crate::usfm::verse_range(vn);
    let label = rest
        .iter()
        .take_while(|t| !matches!(t, Tok::End { name: "verse_export" }))
        .zip(rest.iter().skip(1))
        .find_map(|(t, next)| match (t, next) {
            (Tok::Start { name: "span", attrs }, Tok::Text(text))
                if xml::attr(attrs, "class").as_deref() == Some("vn") =>
            {
                Some(crate::usfm::verse_range(text))
            }
            _ => None,
        });
    match label {
        Some((s, e)) if s == start && e > end => (start, e),
        _ => (start, end),
    }
}

/// What a heading element is: section headings, Hebrew acrostic letters (Psalm 119) and Song of
/// Songs speakers are kept; the chapter number and the passage header are not.
fn heading_kind(class: &str) -> Option<(MarkKind, u8)> {
    match class {
        "chapter-number" | "bk_ch_vs_header" => None,
        "psa-hebrew" => Some((MarkKind::Heading, 3)),
        "sos-speaker" => Some((MarkKind::Speaker, 0)),
        _ => Some((MarkKind::Heading, 1)),
    }
}

/// A paragraph class as a break: `body*` prose, `poetN*` poetry at level N, `selah` the
/// right-aligned "Interlude", `ext-*` extracts (set flush); `-sp` adds space before. A bare `<p>`
/// (the API emits stray ones at verse ends) is no break.
fn para_kind(class: &str) -> Option<(MarkKind, u8, bool)> {
    if class.is_empty() {
        return None;
    }
    let extract = class.starts_with("ext-");
    let rest = class.trim_start_matches("ext-");
    let space_before = rest.split('-').any(|part| part == "sp");
    let base = rest.split('-').next().unwrap_or("");
    let (kind, level) = if let Some(level) = base.strip_prefix("poet") {
        (MarkKind::Poetry, level.parse::<u8>().unwrap_or(1).clamp(1, 4))
    } else if base == "selah" {
        (MarkKind::PoetryRight, 0)
    } else if extract || base == "hanging" {
        (MarkKind::ParagraphFlush, 0)
    } else {
        (MarkKind::Paragraph, 0)
    };
    Some((kind, level, space_before))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::VerseRef;

    // The *format* of NLT API responses, filled with public-domain (KJV) wording.
    const PSALM: &str = r#" <!DOCTYPE html><html lang="en-US"><head><title>NLT API</title>
<link rel="stylesheet" href="https://api.nlt.to/content/nlt-api-css?vers=1.04"/></head><body>
<div id="bibletext" class=" NLT NLT BibleText section"><section><h2 class="bk_ch_vs_header">Psalm 23:1-3, NLT</h2><verse_export orig="psal_23_1" bk="psal" ch="23" vn="1">
<h3 class="chapter-number"><span class="cw">Psalm</span> <span class="cw_ch">23</span></h3>
<h4 class="subhead">The <span class="subhead-sc">Lord</span> Is My Shepherd</h4>
<p class="psa-title">A Psalm of David.</p>
<p class="poet1-vn"><span class="vn">1</span>The <span class="sc">Lord</span> is my shepherd;</p>
<p class="poet2">I shall not want.<a class="a-tn">*</a><span class="tn"><span class="tn-ref">23:1</span> Or <em>a note.</em></span></p>
</verse_export><verse_export orig="psal_23_2" bk="psal" ch="23" vn="2">
<p class="poet1-vn"><span class="vn">2</span>He maketh me to lie down in green pastures:</p>
<p class="poet2">he leadeth me beside the still waters.</p>
</verse_export><verse_export orig="psal_23_3" bk="psal" ch="23" vn="3">
<p class="poet1-vn-sp"><span class="vn">3</span>He restoreth my soul.</p>
<p class="selah">Selah<a class="a-tn">*</a><span class="tn"><span class="tn-ref">23:3</span> A note.</span></p>
</verse_export></section></div></body></html>"#;

    const JOHN: &str = r#"<div id="bibletext" class=" NLT NLT BibleText section"><section><h2 class="bk_ch_vs_header">John 3:16-18, NLT</h2><verse_export orig="john_3_16" bk="john" ch="3" vn="16">
<h3 class="subhead">For God So Loved the World</h3>
<p class="body-hd"><span class="vn">16</span><span class="red">&ldquo;For God so loved the world,<a class="a-tn">*</a><span class="tn"><span class="tn-ref">3:16</span> Or <em>a note.</em></span> that he gave his only begotten Son.</span> </verse_export><verse_export orig="john_3_17" bk="john" ch="3" vn="17"><span class="vn">17</span><span class="red">For God sent not his Son into the world to condemn the world.&rdquo;</span><p>
</verse_export><verse_export orig="john_3_18" bk="john" ch="3" vn="18">
<p class="body"><span class="vn">18</span>He that believeth on him is not condemned: thus saith the <span class="red-sc">Lord</span>, <em>verily</em>.</p>
</verse_export></section></div>"#;

    #[test]
    fn parses_psalm_structure() {
        let c = ChapterIdx::new(BookId::PSALMS, 23).unwrap();
        let t = parse_html(c, PSALM);
        let vs: Vec<_> = t.verses_in(c).collect();
        assert_eq!(vs.len(), 3);
        let kinds: Vec<_> = vs[0].marks.iter().map(|m| (m.kind, m.level)).collect();
        assert_eq!(
            kinds,
            [(MarkKind::Heading, 1), (MarkKind::PsalmTitle, 0), (MarkKind::Poetry, 1), (MarkKind::Poetry, 2)]
        );
        assert_eq!(t.mark_text(&vs[0].marks[0]), "The LORD Is My Shepherd");
        assert_eq!(t.mark_text(&vs[0].marks[1]), "A Psalm of David.");
        // Notes and their markers are left out; the divine name is kept in capitals.
        assert_eq!(vs[0].plain_text(), "The LORD is my shepherd; I shall not want.");
        assert_eq!(vs[0].spans.iter().map(|s| s.style).collect::<Vec<_>>(), [Style::DivineName]);
        // `-sp`: a stanza break before the line; `selah`: a right-aligned Selah line.
        let v3: Vec<_> = vs[2].marks.iter().map(|m| (m.kind, m.level)).collect();
        assert_eq!(v3, [(MarkKind::Blank, 0), (MarkKind::Poetry, 1), (MarkKind::PoetryRight, 0)]);
        assert_eq!(vs[2].plain_text(), "He restoreth my soul. Selah");
        assert!(
            vs[2].spans.iter().any(|s| s.style == Style::Selah && &vs[2].text[s.start as usize..] == "Selah")
        );
        assert!(!vs[2].text.contains("note") && !t.mark_text(&vs[0].marks[0]).contains("23"));
    }

    #[test]
    fn parses_prose_red_letters_and_notes() {
        let c = ChapterIdx::new(BookId(42), 3).unwrap();
        let t = parse_html(c, JOHN);
        let v16 = t.find(VerseRef::new(BookId(42), 3, 16)).unwrap();
        assert_eq!(v16.text, "“For God so loved the world, that he gave his only begotten Son.");
        assert_eq!(
            v16.marks.iter().map(|m| m.kind).collect::<Vec<_>>(),
            [MarkKind::Heading, MarkKind::Paragraph]
        );
        assert_eq!(t.mark_text(&v16.marks[0]), "For God So Loved the World");
        assert_eq!(v16.spans.len(), 1);
        assert_eq!(
            (v16.spans[0].style, v16.spans[0].start, v16.spans[0].end as usize),
            (Style::WordsOfJesus, 0, v16.text.len())
        );
        // Verse 17 continues the paragraph (no break), still in red; the stray `<p>` is no break.
        let v17 = t.find(VerseRef::new(BookId(42), 3, 17)).unwrap();
        assert!(v17.marks.is_empty());
        assert!(v17.text.ends_with("condemn the world.”"));
        assert_eq!(v17.spans[0].style, Style::WordsOfJesus);
        let v18 = t.find(VerseRef::new(BookId(42), 3, 18)).unwrap();
        assert_eq!(v18.marks.iter().map(|m| m.kind).collect::<Vec<_>>(), [MarkKind::Paragraph]);
        assert_eq!(v18.text, "He that believeth on him is not condemned: thus saith the LORD, verily.");
        let styles: Vec<_> =
            v18.spans.iter().map(|s| (s.style, &v18.text[s.start as usize..s.end as usize])).collect();
        assert!(styles.contains(&(Style::WordsOfJesus, "LORD")));
        assert!(styles.contains(&(Style::DivineName, "LORD")));
        assert!(styles.contains(&(Style::Italic, "verily")));
    }

    #[test]
    fn ranges_of_a_chapter_join_up() {
        // Two consecutive ranges, each a complete page: the second page's title isn't text.
        let c = ChapterIdx::new(BookId(42), 3).unwrap();
        let page = |vn: u8, text: &str| {
            format!(
                r#"<html><head><title>NLT API</title></head><body><div><section><h2 class="bk_ch_vs_header">John 3:{vn}, NLT</h2><verse_export vn="{vn}"><p class="body"><span class="vn">{vn}</span>{text}</p></verse_export></section></div></body></html>"#
            )
        };
        let mut b = TranslationBuilder::new(TranslationMeta::default());
        b.begin_book(c.book());
        b.begin_chapter(3);
        assert_eq!(parse_into(&mut b, &page(1, "There was a man of the Pharisees.")), 1);
        assert_eq!(parse_into(&mut b, &page(2, "The same came to Jesus by night.")), 1);
        let t = b.finish();
        let vs: Vec<_> = t.verses_in(c).map(|v| v.text.to_string()).collect();
        assert_eq!(vs, ["There was a man of the Pharisees.", "The same came to Jesus by night."]);
    }

    #[test]
    fn classes_and_bridged_verses() {
        assert_eq!(para_kind("poet2-vn-sp"), Some((MarkKind::Poetry, 2, true)));
        assert_eq!(para_kind("body-ch-hd"), Some((MarkKind::Paragraph, 0, false)));
        assert_eq!(para_kind("ext-hanging-sp"), Some((MarkKind::ParagraphFlush, 0, true)));
        assert_eq!(para_kind("ext-poet1-vn-sp"), Some((MarkKind::Poetry, 1, true)));
        assert_eq!(para_kind(""), None);
        assert_eq!(heading_kind("psa-hebrew"), Some((MarkKind::Heading, 3)));
        assert_eq!(heading_kind("chapter-number"), None);
        let c = ChapterIdx::new(BookId(1), 20).unwrap();
        let t = parse_html(
            c,
            r#"<verse_export vn="7"><p class="body"><span class="vn">7-8</span>Thou shalt not steal.</p></verse_export>"#,
        );
        let v = t.verses_in(c).next().unwrap();
        assert_eq!((v.number, v.number_end), (7, 8));
    }

    #[test]
    fn book_names_and_policy() {
        assert_eq!(api_book(BookId::GENESIS), "Gen");
        assert_eq!(api_book(BookId::PSALMS), "Ps");
        assert_eq!(api_book(BookId(51)), "1Thes");
        assert_eq!(api_book(BookId(63)), "3Jn");
        assert_eq!(url_encode("a b/c"), "a%20b%2Fc");
        let p = Nlt::new(None).policy();
        assert_eq!((p.max_verses, p.book_share), (500, BookShare::LessThanWhole));
        assert!(!Nlt::new(None).needs_key());
    }
}
