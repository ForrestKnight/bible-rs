//! The objects built from the text (`objects/*.json` at the repository's root, documented in
//! `objects/README.md`): where the reader shows an object's glyph, and the verses its page quotes.
//! Only the Ark of the Covenant so far.

use std::sync::OnceLock;

use bible_core::books::{BookId, VerseRef};
use serde_json::Value;

const ARK: &str = include_str!("../../../objects/ark.json");
/// The cubits every object can be measured in; the royal cubit cites its verse.
const CUBITS: &str = include_str!("../../../objects/cubits.json");

pub struct Object {
    /// Its page on the website, /<slug>, which the app shows from its own copy (`webview.rs`).
    pub slug: String,
    pub title: String,
    /// The first verse of each passage that describes it: the reader marks these with its glyph.
    pub glyphs: Vec<VerseRef>,
    /// Every verse the page quotes, with its OSIS reference (`Exod.25.10`), in the files' order.
    pub verses: Vec<(String, VerseRef)>,
}

/// The Ark of the Covenant (objects/ark.json).
pub fn ark() -> &'static Object {
    static ARK_OBJECT: OnceLock<Object> = OnceLock::new();
    ARK_OBJECT.get_or_init(|| parse(ARK, CUBITS).expect("objects/ark.json"))
}

fn parse(json: &str, cubits: &str) -> Option<Object> {
    let v: Value = serde_json::from_str(json).ok()?;
    let cubits: Value = serde_json::from_str(cubits).ok()?;
    let mut glyphs = Vec::new();
    let mut verses = Vec::new();
    for source in v["sources"].as_array()? {
        let range = osis_range(source.as_str()?)?;
        glyphs.push(*range.first()?);
        for r in range {
            push(&mut verses, r);
        }
    }
    let mut refs = Vec::new();
    collect_refs(&v, &mut refs);
    collect_refs(&cubits, &mut refs);
    for r in refs {
        push(&mut verses, *osis_range(r)?.first()?);
    }
    Some(Object { slug: v["slug"].as_str()?.into(), title: v["title"].as_str()?.into(), glyphs, verses })
}

fn push(verses: &mut Vec<(String, VerseRef)>, r: VerseRef) {
    if !verses.iter().any(|(_, v)| *v == r) {
        verses.push((osis(r), r));
    }
}

/// Every string under a `ref` key, anywhere in the file.
fn collect_refs<'a>(v: &'a Value, out: &mut Vec<&'a str>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                match (k.as_str(), x) {
                    ("ref", Value::String(s)) => out.push(s),
                    _ => collect_refs(x, out),
                }
            }
        }
        Value::Array(a) => a.iter().for_each(|x| collect_refs(x, out)),
        _ => {}
    }
}

/// `Exod.25.10` or `Exod.25.10-22` as its verses.
pub fn osis_range(s: &str) -> Option<Vec<VerseRef>> {
    let mut parts = s.split('.');
    let book = BookId::from_osis(parts.next()?)?;
    let chapter: u8 = parts.next()?.parse().ok()?;
    let verses = parts.next()?;
    if parts.next().is_some() || chapter == 0 || chapter > book.chapters() {
        return None;
    }
    let (first, last) = match verses.split_once('-') {
        Some((a, b)) => (a.parse::<u8>().ok()?, b.parse::<u8>().ok()?),
        None => {
            let v = verses.parse::<u8>().ok()?;
            (v, v)
        }
    };
    if first == 0 || last < first {
        return None;
    }
    Some((first..=last).map(|v| VerseRef::new(book, chapter, v)).collect())
}

pub fn osis(r: VerseRef) -> String {
    format!("{}.{}.{}", r.book.info().osis, r.chapter, r.verse)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXODUS: BookId = BookId(1);

    #[test]
    fn the_ark_marks_the_first_verse_of_each_passage() {
        let ark = ark();
        assert_eq!(ark.slug, "ark-of-the-covenant");
        assert_eq!(ark.glyphs, [VerseRef::new(EXODUS, 25, 10), VerseRef::new(EXODUS, 37, 1)]);
    }

    #[test]
    fn the_ark_quotes_its_passages_and_every_ref() {
        let refs: Vec<&str> = ark().verses.iter().map(|(o, _)| o.as_str()).collect();
        // Exodus 25:10–22 and 37:1–9 in full, then the verses cited outside them.
        assert_eq!(refs.len(), 13 + 9 + 3, "{refs:?}");
        assert_eq!(refs[0], "Exod.25.10");
        assert!(refs.contains(&"Exod.37.9"));
        assert!(refs.contains(&"Heb.9.4"));
        assert!(refs.contains(&"Exod.31.18"));
        assert!(refs.contains(&"Ezek.40.5"));
    }

    #[test]
    fn osis_references() {
        assert_eq!(osis_range("Heb.9.4"), Some(vec![VerseRef::new(BookId(57), 9, 4)]));
        assert_eq!(osis_range("1Kgs.8.9"), Some(vec![VerseRef::new(BookId(10), 8, 9)]));
        assert_eq!(osis_range("Exod.25.10-12").map(|v| v.len()), Some(3));
        assert_eq!(osis_range("Exod.41.1"), None);
        assert_eq!(osis_range("Exod.25.0"), None);
        assert_eq!(osis_range("Nope.1.1"), None);
        assert_eq!(osis(VerseRef::new(BookId(10), 8, 9)), "1Kgs.8.9");
    }
}
