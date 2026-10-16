//! NET Bible web service (labs.bible.org): the NET Bible®, © Biblical Studies Press.
//!
//! bible.org serves the NET without a key. Its copyright page lets the text (without the
//! translators' notes) be quoted "in any form" for non-commercial use, with the abbreviation
//! (NET), a copyright acknowledgment, and in apps with internet access "NET" linked to
//! netbible.org. A chapter comes back as JSON, one object per verse whose `text` is an HTML
//! fragment: `<p class="bodytext">` paragraphs, `<p class="poetry">` lines, a Psalm's
//! superscription in `<p class="psasuper">`, the divine name in `<span class="smcaps">`, Strong's
//! numbers in `<st>` (kept as plain text) and note markers `<n/>` (left out).

use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;

use super::{BookShare, CachePolicy, FetchedChapter, Source, http_agent, read_body};
use crate::books::ChapterIdx;
use crate::model::{MarkKind, Style, Translation, TranslationBuilder, TranslationMeta};
use crate::xml::{self, Tok};

/// The acknowledgment the NET Bible copyright page asks for with quoted text.
pub const COPYRIGHT: &str = "Scripture quoted by permission. Quotations designated (NET) are from the NET Bible® \
copyright ©1996, 2019 by Biblical Studies Press, L.L.C. http://netbible.com All rights reserved.";

pub const DEFAULT_BASE: &str = "https://labs.bible.org/api/";

pub struct Net {
    base: String,
    agent: ureq::Agent,
}

impl Default for Net {
    fn default() -> Self {
        Self::with_base(DEFAULT_BASE)
    }
}

impl Net {
    pub fn with_base(base: &str) -> Self {
        Self { base: base.to_string(), agent: http_agent() }
    }
}

#[derive(Deserialize)]
struct Verse {
    chapter: String,
    verse: String,
    text: String,
}

impl Source for Net {
    fn policy(&self) -> CachePolicy {
        CachePolicy {
            // The NET's terms set no limit on how much may be kept; refresh monthly for corrections.
            max_verses: 31_102,
            book_share: BookShare::Any,
            max_age_secs: 30 * 86_400,
            // No published rate limit: one request a second is polite.
            min_request_interval: Duration::from_secs(1),
        }
    }

    fn needs_key(&self) -> bool {
        false
    }

    fn fetch(&self, _key: &str, c: ChapterIdx) -> Result<FetchedChapter> {
        let passage = format!("{} {}", c.book().name(), c.chapter()).replace(' ', "+");
        let url = format!("{}?passage={passage}&type=json&formatting=full", self.base);
        let body = read_body(self.agent.get(&url).call(), "NET Bible API")?;
        let verses: Vec<Verse> = serde_json::from_str(&body).context("NET Bible API: unexpected response")?;
        let chapter = parse_verses(c, &verses);
        anyhow::ensure!(chapter.verse_count() > 0, "NET Bible API: no text came back for {}", c.display());
        Ok(FetchedChapter { chapter, copyright: Some(COPYRIGHT.into()), fums_token: None })
    }
}

/// Parses a chapter's verse objects; any for another chapter are ignored.
fn parse_verses(c: ChapterIdx, verses: &[Verse]) -> Translation {
    let mut b = TranslationBuilder::new(TranslationMeta { id: "net".into(), ..Default::default() });
    b.begin_book(c.book());
    b.begin_chapter(c.chapter() as u32);
    for v in verses.iter().filter(|v| v.chapter.trim().parse::<u8>().ok() == Some(c.chapter())) {
        let (start, end) = crate::usfm::verse_range(&v.verse);
        if start > 0 {
            b.begin_verse(start, end);
            parse_fragment(&mut b, &v.text);
        }
    }
    b.finish()
}

/// Parses a chapter from the API's JSON (tests and tools).
pub fn parse_json(c: ChapterIdx, json: &str) -> Result<Translation> {
    let verses: Vec<Verse> = serde_json::from_str(json)?;
    Ok(parse_verses(c, &verses))
}

/// Feeds one verse's HTML fragment into `b`.
fn parse_fragment(b: &mut TranslationBuilder, html: &str) {
    let mut caps = 0usize;
    let mut skip = 0usize;
    // A Psalm superscription being collected; it runs to the next paragraph (the API leaves its
    // `<p>` open).
    let mut title: Option<String> = None;
    let mut italic = 0usize;
    for tok in xml::tokens(html) {
        match tok {
            Tok::Start { name: "p", attrs } => {
                if let Some(t) = title.take() {
                    b.heading(MarkKind::PsalmTitle, 0, &t);
                }
                let class = xml::attr(attrs, "class").unwrap_or_default();
                if class == "psasuper" {
                    title = Some(String::new());
                } else if class.contains("poetry") {
                    b.mark(MarkKind::Poetry, 1);
                } else {
                    b.mark(MarkKind::Paragraph, 0);
                }
            }
            Tok::End { name: "p" } => {
                if let Some(t) = title.take() {
                    b.heading(MarkKind::PsalmTitle, 0, &t);
                }
            }
            Tok::Start { name: "span", attrs } if xml::attr(attrs, "class").as_deref() == Some("smcaps") => {
                caps += 1;
                if title.is_none() {
                    b.push_style(Style::DivineName);
                }
            }
            Tok::End { name: "span" } if caps > 0 => {
                caps -= 1;
                if title.is_none() {
                    b.pop_style(Style::DivineName);
                }
            }
            // Translators' notes stay out (the notes aren't covered by the free-use terms).
            Tok::Start { name: "n", .. } => skip += 1,
            Tok::End { name: "n" } => skip = skip.saturating_sub(1),
            Tok::Start { name: "i", .. } => {
                italic += 1;
                b.push_style(Style::Italic);
            }
            Tok::End { name: "i" } if italic > 0 => {
                italic -= 1;
                b.pop_style(Style::Italic);
            }
            Tok::Text(t) if skip == 0 => {
                let t = if caps > 0 { t.to_uppercase() } else { t.into_owned() };
                match &mut title {
                    Some(s) => s.push_str(&t),
                    None => b.text(&t),
                }
            }
            _ => {}
        }
    }
    if let Some(t) = title {
        b.heading(MarkKind::PsalmTitle, 0, &t);
    }
    while italic > 0 {
        italic -= 1;
        b.pop_style(Style::Italic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::{BookId, VerseRef};

    // The *format* of NET Bible API responses (`formatting=full`), with public-domain (KJV) wording.
    const PSALM: &str = r#"[{"bookname":"Psalms","chapter":"23","verse":"1","text":"<p class=\"psasuper\">A <st data-num=\"04210\" class=\"\">Psalm<\/st> of David.<p class=\"poetry\">The <span class=\"smcaps\"><st data-num=\"03068\">Lord<\/st><\/span> is my shepherd;<n id=\"1\" \/> <p class=\"poetry\">I shall not want.<n id=\"2\" \/> <\/p>"},
{"bookname":"Psalms","chapter":"23","verse":"2","text":"<p class=\"poetry\">He maketh me to lie down in <i>green<\/i> pastures: (Selah)<\/p>"}]"#;

    const JOHN: &str = r#"[{"bookname":"John","chapter":"3","verse":"16","text":"<p class=\"bodytext\"><st data-num=\"1063\" class=\"\">For<\/st> God so loved the world,<n id=\"1\" \/> that he gave his only begotten Son. "},
{"bookname":"John","chapter":"3","verse":"17","text":"For God sent not his Son into the world to condemn the world.<\/p>"},
{"bookname":"John","chapter":"4","verse":"1","text":"<p class=\"bodytext\">Another chapter.<\/p>"}]"#;

    #[test]
    fn parses_psalm() {
        let c = ChapterIdx::new(BookId::PSALMS, 23).unwrap();
        let t = parse_json(c, PSALM).unwrap();
        let vs: Vec<_> = t.verses_in(c).collect();
        assert_eq!(vs.len(), 2);
        let kinds: Vec<_> = vs[0].marks.iter().map(|m| (m.kind, m.level)).collect();
        assert_eq!(kinds, [(MarkKind::PsalmTitle, 0), (MarkKind::Poetry, 1), (MarkKind::Poetry, 1)]);
        assert_eq!(t.mark_text(&vs[0].marks[0]), "A Psalm of David.");
        assert_eq!(vs[0].plain_text(), "The LORD is my shepherd; I shall not want.");
        assert_eq!(vs[0].spans.iter().map(|s| s.style).collect::<Vec<_>>(), [Style::DivineName]);
        assert_eq!(vs[1].plain_text(), "He maketh me to lie down in green pastures: (Selah)");
        assert_eq!(vs[1].spans[0].style, Style::Italic);
    }

    #[test]
    fn parses_prose_and_ignores_other_chapters() {
        let c = ChapterIdx::new(BookId(42), 3).unwrap();
        let t = parse_json(c, JOHN).unwrap();
        assert_eq!(t.verse_count(), 2);
        let v16 = t.find(VerseRef::new(BookId(42), 3, 16)).unwrap();
        assert_eq!(v16.text, "For God so loved the world, that he gave his only begotten Son.");
        assert_eq!(v16.marks.iter().map(|m| m.kind).collect::<Vec<_>>(), [MarkKind::Paragraph]);
        let v17 = t.find(VerseRef::new(BookId(42), 3, 17)).unwrap();
        assert!(v17.marks.is_empty(), "verse 17 continues the paragraph");
        assert!(!Net::default().needs_key());
    }
}
