//! ESV API (api.esv.org) — the English Standard Version, © Crossway.
//!
//! Uses the plain-text passage endpoint with fixed indentation so paragraphs (2 spaces), poetry
//! levels (4 spaces each), headings (no indent) and `[n]` verse numbers can be recovered.

use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;

use super::{BookShare, CachePolicy, FetchedChapter, Source, http_agent, read_body};
use crate::books::{BookId, ChapterIdx};
use crate::model::{MarkKind, Style, Translation, TranslationBuilder, TranslationMeta};

/// One of the copyright notices the ESV API terms require (shown with the text and in Settings).
pub const COPYRIGHT: &str = "Scripture quotations are from the ESV® Bible (The Holy Bible, English Standard Version®), \
© 2001 by Crossway, a publishing ministry of Good News Publishers. Used by permission. All rights reserved. The ESV text \
may not be quoted in any publication made available to the public by a Creative Commons license. The ESV may not be \
translated into any other language. Users may not copy or download more than 500 verses of the ESV Bible or more than \
one half of any book of the ESV Bible. www.esv.org";

/// Where a free key is made: an "API application" on the user's esv.org account.
pub const SIGN_UP_URL: &str = "https://api.esv.org/account/create-application/";

pub struct Esv {
    base: String,
    agent: ureq::Agent,
}

impl Default for Esv {
    fn default() -> Self {
        Self::with_base("https://api.esv.org")
    }
}

impl Esv {
    pub fn with_base(base: &str) -> Self {
        Self { base: base.trim_end_matches('/').to_string(), agent: http_agent() }
    }
}

#[derive(Deserialize)]
struct Response {
    passages: Vec<String>,
}

impl Source for Esv {
    fn policy(&self) -> CachePolicy {
        CachePolicy {
            max_verses: 500,
            book_share: BookShare::Half,
            max_age_secs: 30 * 86_400,
            // Terms: no more than 60 requests a minute.
            min_request_interval: Duration::from_millis(1_050),
        }
    }

    fn fetch(&self, key: &str, c: ChapterIdx) -> Result<FetchedChapter> {
        let q = format!("{} {}", c.book().name(), c.chapter());
        let url = format!(
            "{}/v3/passage/text/?q={}&include-passage-references=false&include-verse-numbers=true\
             &include-first-verse-numbers=true&include-footnotes=false&include-footnote-body=false\
             &include-headings=true&include-short-copyright=false&include-copyright=false&include-selahs=true\
             &indent-using=space&indent-paragraphs=2&indent-poetry=true&indent-poetry-lines=4\
             &indent-declares=40&indent-psalm-doxology=30&line-length=0",
            self.base,
            q.replace(' ', "+")
        );
        let body = read_body(
            self.agent.get(&url).header("Authorization", &format!("Token {key}")).call(),
            "ESV API",
        )?;
        let resp: Response = serde_json::from_str(&body).context("unexpected ESV API response")?;
        let text = resp.passages.first().context("ESV API returned no passage")?;
        Ok(FetchedChapter {
            chapter: parse_text(c, text),
            copyright: Some(COPYRIGHT.into()),
            fums_token: None,
        })
    }
}

/// Parses an ESV plain-text passage (one chapter) into the common model.
pub fn parse_text(c: ChapterIdx, text: &str) -> Translation {
    let mut b = TranslationBuilder::new(TranslationMeta { id: "esv".into(), ..Default::default() });
    b.begin_book(c.book());
    b.begin_chapter(c.chapter() as u32);
    let mut seen_verse = false;
    let mut headings_before_first = 0;
    let mut in_poetry = false;
    let mut pending_blank = false;
    for line in text.lines() {
        if line.trim().is_empty() {
            pending_blank = in_poetry;
            continue;
        }
        let indent = line.len() - line.trim_start_matches(' ').len();
        let content = line.trim();
        let has_verse = find_marker(content, 0).is_some();
        if indent == 0 && !has_verse {
            // Headings (and, before the first verse of a psalm, its superscription).
            let psalm_title = c.book() == BookId::PSALMS
                && !seen_verse
                && (headings_before_first > 0 || looks_like_superscription(content));
            let (kind, level) = if psalm_title { (MarkKind::PsalmTitle, 0) } else { (MarkKind::Heading, 1) };
            b.heading(kind, level, content);
            if !seen_verse {
                headings_before_first += 1;
            }
            in_poetry = false;
            pending_blank = false;
            continue;
        }
        match indent {
            0..=3 => {
                b.mark(MarkKind::Paragraph, 0);
                in_poetry = false;
            }
            30.. => b.mark(MarkKind::PoetryRight, 0),
            _ => {
                if pending_blank {
                    b.mark(MarkKind::Blank, 0);
                }
                b.mark(MarkKind::Poetry, (indent / 4).clamp(1, 3) as u8);
                in_poetry = true;
            }
        }
        pending_blank = false;
        let mut pos = 0;
        while pos < content.len() {
            match find_marker(content, pos) {
                Some((start, end, verse)) => {
                    emit(&mut b, &content[pos..start]);
                    b.begin_verse(verse, verse);
                    seen_verse = true;
                    pos = end;
                }
                None => {
                    emit(&mut b, &content[pos..]);
                    break;
                }
            }
        }
    }
    b.finish()
}

/// Emits text, styling "Selah" and the divine name the way the printed ESV does.
fn emit(b: &mut TranslationBuilder, s: &str) {
    let mut rest = s;
    while let Some(i) = rest.find("LORD") {
        // Only whole-word upper-case LORD is the divine name.
        let before_ok = rest[..i].chars().last().is_none_or(|c| !c.is_alphabetic());
        let after_ok = rest[i + 4..].chars().next().is_none_or(|c| !c.is_alphabetic());
        b.text(&rest[..i]);
        if before_ok && after_ok {
            b.push_style(Style::DivineName);
            b.text("LORD");
            b.pop_style(Style::DivineName);
        } else {
            b.text("LORD");
        }
        rest = &rest[i + 4..];
    }
    b.text(rest);
}

fn looks_like_superscription(s: &str) -> bool {
    ["A Psalm", "A Song", "A Maskil", "A Miktam", "A Prayer", "Of David", "To the choirmaster", "A Shiggaion"]
        .iter()
        .any(|p| s.starts_with(p))
}

/// Finds the next `[n]` or `[c:n]` verse marker at or after `from`: (start, end, verse).
fn find_marker(s: &str, from: usize) -> Option<(usize, usize, u32)> {
    let bytes = s.as_bytes();
    let mut i = from;
    while let Some(rel) = s[i..].find('[') {
        let start = i + rel;
        let mut j = start + 1;
        while j < bytes.len() && (bytes[j].is_ascii_digit() || bytes[j] == b':') {
            j += 1;
        }
        if j < bytes.len() && bytes[j] == b']' && j > start + 1 {
            let inner = &s[start + 1..j];
            let verse = inner.rsplit(':').next()?.parse().ok()?;
            let mut end = j + 1;
            if bytes.get(end) == Some(&b' ') {
                end += 1;
            }
            return Some((start, end, verse));
        }
        i = start + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::VerseRef;

    // The *format* of an ESV API text response, filled with public-domain (KJV) wording.
    const PSALM: &str = "The LORD Is My Shepherd\n\nA Psalm of David.\n\n    [1] The LORD is my shepherd;\n        I shall not want.\n    [2] He maketh me to lie down in green pastures:\n        he leadeth me beside the still waters.\n\n    [3] He restoreth my soul.\n";
    const JOHN: &str = "For God So Loved the World\n\n  [16] For God so loved the world, that he gave his only begotten Son, that whosoever believeth in him should not perish, but have everlasting life. [17] For God sent not his Son into the world to condemn the world;\n  but that the world through him might be saved.\n";

    #[test]
    fn parses_psalm_structure() {
        let c = ChapterIdx::new(BookId::PSALMS, 23).unwrap();
        let t = parse_text(c, PSALM);
        let vs: Vec<_> = t.verses_in(c).collect();
        assert_eq!(vs.len(), 3);
        let kinds: Vec<_> = vs[0].marks.iter().map(|m| (m.kind, m.level)).collect();
        assert_eq!(
            kinds,
            [(MarkKind::Heading, 1), (MarkKind::PsalmTitle, 0), (MarkKind::Poetry, 1), (MarkKind::Poetry, 2)]
        );
        assert_eq!(t.mark_text(&vs[0].marks[1]), "A Psalm of David.");
        assert_eq!(vs[0].plain_text(), "The LORD is my shepherd; I shall not want.");
        assert_eq!(vs[0].spans[0].style, Style::DivineName);
        assert_eq!(vs[2].marks[0].kind, MarkKind::Blank);
    }

    #[test]
    fn parses_prose_with_inline_verse_numbers() {
        let c = ChapterIdx::new(BookId(42), 3).unwrap();
        let t = parse_text(c, JOHN);
        let v16 = t.find(VerseRef::new(BookId(42), 3, 16)).unwrap();
        assert_eq!(
            v16.marks.iter().map(|m| m.kind).collect::<Vec<_>>(),
            [MarkKind::Heading, MarkKind::Paragraph]
        );
        assert!(v16.text.ends_with("everlasting life."));
        let v17 = t.find(VerseRef::new(BookId(42), 3, 17)).unwrap();
        assert_eq!(
            v17.plain_text(),
            "For God sent not his Son into the world to condemn the world; but that the world through him might be saved."
        );
        assert_eq!(v17.marks[0].kind, MarkKind::Paragraph);
        assert_eq!(find_marker("x [3:16] y", 0), Some((2, 9, 16)));
        assert_eq!(find_marker("[a] [] x", 0), None);
    }
}
