//! API.Bible (rest.api.bible) — licensed translations such as NASB, NKJV or CSB where the user's
//! plan and the publisher allow it.
//!
//! Chapters are requested with `content-type=json`, a USX-derived tree whose paragraph and
//! character styles are USFM marker names, so they map through the same tables as the USFM
//! importer. Every displayed chapter is reported via FUMS, as API.Bible requires.

use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;

use super::{BookShare, CachePolicy, FetchedChapter, Source, http_agent, read_body};
use crate::books::ChapterIdx;
use crate::model::{Translation, TranslationBuilder, TranslationMeta};
use crate::usfm::{Para, char_style, para_kind, verse_range};

pub const DEFAULT_BASE: &str = "https://rest.api.bible/v1";
pub const FUMS_BASE: &str = "https://fums.api.bible/f3";

pub struct ApiBible {
    base: String,
    fums_base: String,
    bible_id: String,
    agent: ureq::Agent,
    device_id: String,
    session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteBible {
    pub id: String,
    pub name: String,
    pub abbreviation: String,
    pub language: String,
    pub description: String,
}

impl ApiBible {
    pub fn new(bible_id: &str, device_id: &str) -> Self {
        Self::with_base(DEFAULT_BASE, FUMS_BASE, bible_id, device_id)
    }

    pub fn with_base(base: &str, fums_base: &str, bible_id: &str, device_id: &str) -> Self {
        Self {
            base: base.trim_end_matches('/').to_string(),
            fums_base: fums_base.to_string(),
            bible_id: bible_id.to_string(),
            agent: http_agent(),
            device_id: device_id.to_string(),
            session_id: super::random_id(),
        }
    }
}

/// Lists the Bibles an API key can access (optionally filtered by ISO 639-3 language, e.g. "eng").
pub fn list_bibles(base: &str, key: &str, language: Option<&str>) -> Result<Vec<RemoteBible>> {
    let mut url = format!("{}/bibles", base.trim_end_matches('/'));
    if let Some(l) = language {
        url.push_str(&format!("?language={l}"));
    }
    let body = read_body(http_agent().get(&url).header("api-key", key).call(), "API.Bible")?;
    #[derive(Deserialize)]
    struct Lang {
        #[serde(default)]
        name: String,
    }
    #[derive(Deserialize)]
    struct Item {
        id: String,
        #[serde(default)]
        name: String,
        #[serde(default)]
        abbreviation: String,
        #[serde(default, rename = "abbreviationLocal")]
        abbreviation_local: String,
        #[serde(default)]
        description: Option<String>,
        language: Option<Lang>,
    }
    #[derive(Deserialize)]
    struct List {
        data: Vec<Item>,
    }
    let list: List = serde_json::from_str(&body).context("unexpected API.Bible response")?;
    let mut out: Vec<RemoteBible> = list
        .data
        .into_iter()
        .map(|i| RemoteBible {
            id: i.id,
            name: i.name.trim().to_string(),
            abbreviation: if i.abbreviation_local.is_empty() { i.abbreviation } else { i.abbreviation_local },
            language: i.language.map(|l| l.name).unwrap_or_default(),
            description: i.description.unwrap_or_default(),
        })
        .collect();
    out.sort_by_key(|a| a.abbreviation.to_lowercase());
    Ok(out)
}

impl Source for ApiBible {
    fn policy(&self) -> CachePolicy {
        CachePolicy {
            // "fewer than 500 consecutive verses", refreshed at least every 14 days.
            max_verses: 499,
            book_share: BookShare::Any,
            max_age_secs: 14 * 86_400,
            min_request_interval: Duration::from_millis(500),
        }
    }

    fn fetch(&self, key: &str, c: ChapterIdx) -> Result<FetchedChapter> {
        let url = format!(
            "{}/bibles/{}/chapters/{}.{}?content-type=json&include-notes=false&include-titles=true\
             &include-chapter-numbers=false&include-verse-numbers=true&include-verse-spans=false",
            self.base,
            self.bible_id,
            c.book().info().code,
            c.chapter()
        );
        let body = read_body(self.agent.get(&url).header("api-key", key).call(), "API.Bible")?;
        let v: Value = serde_json::from_str(&body).context("unexpected API.Bible response")?;
        let data = v.get("data").context("API.Bible response has no data")?;
        let chapter = match data.get("content") {
            Some(Value::Array(items)) => parse_json_content(c, items),
            Some(Value::String(s)) => parse_text_content(c, s),
            _ => anyhow::bail!("API.Bible response has no content"),
        };
        let copyright = data.get("copyright").and_then(Value::as_str).map(|s| s.trim().to_string());
        let meta = v.get("meta");
        let fums_token = meta
            .and_then(|m| m.get("fumsToken").or_else(|| m.get("fumsId")))
            .and_then(Value::as_str)
            .map(str::to_string);
        Ok(FetchedChapter { chapter, copyright, fums_token })
    }

    fn report_view(&self, token: &str) {
        let url = format!("{}?t={token}&dId={}&sId={}", self.fums_base, self.device_id, self.session_id);
        let _ = self.agent.get(&url).call();
    }
}

/// Parses API.Bible's JSON chapter content (`[{name: "para", attrs: {style}, items: […]}, …]`).
pub fn parse_json_content(c: ChapterIdx, items: &[Value]) -> Translation {
    let mut b = TranslationBuilder::new(TranslationMeta::default());
    b.begin_book(c.book());
    b.begin_chapter(c.chapter() as u32);
    for item in items {
        walk(&mut b, item);
    }
    b.finish()
}

fn style_of(v: &Value) -> &str {
    v.get("attrs").and_then(|a| a.get("style")).and_then(Value::as_str).unwrap_or("")
}

fn children(v: &Value) -> &[Value] {
    v.get("items").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn collect_text(v: &Value, out: &mut String) {
    if let Some(t) = v.get("text").and_then(Value::as_str) {
        out.push_str(t);
    }
    if v.get("name").and_then(Value::as_str) == Some("note") {
        return;
    }
    for c in children(v) {
        collect_text(c, out);
    }
}

fn walk(b: &mut TranslationBuilder, v: &Value) {
    if v.get("type").and_then(Value::as_str) == Some("text") {
        if let Some(t) = v.get("text").and_then(Value::as_str) {
            b.text(t);
        }
        return;
    }
    match v.get("name").and_then(Value::as_str).unwrap_or("") {
        "para" => match para_kind(style_of(v)) {
            Para::Break(kind, level) => {
                b.mark(kind, level);
                children(v).iter().for_each(|c| walk(b, c));
            }
            Para::Body => children(v).iter().for_each(|c| walk(b, c)),
            Para::Heading(kind, level) => {
                let mut s = String::new();
                collect_text(v, &mut s);
                b.heading(kind, level, &s);
            }
            Para::Title(_) | Para::Ignore => {}
        },
        "verse" => {
            let n = v.get("attrs").and_then(|a| a.get("number")).and_then(Value::as_str).unwrap_or("");
            let (start, end) = verse_range(n);
            if start > 0 {
                b.begin_verse(start, end);
            }
        }
        "char" => {
            let style = char_style(style_of(v)).flatten();
            if let Some(s) = style {
                b.push_style(s);
            }
            children(v).iter().for_each(|c| walk(b, c));
            if let Some(s) = style {
                b.pop_style(s);
            }
        }
        "note" => {}
        _ => children(v).iter().for_each(|c| walk(b, c)),
    }
}

/// Fallback for `content-type=text` ("[1] In the beginning…").
pub fn parse_text_content(c: ChapterIdx, s: &str) -> Translation {
    super::esv::parse_text(c, s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::{BookId, VerseRef};
    use crate::model::{MarkKind, Style};

    /// The *shape* of an API.Bible JSON chapter, filled with public-domain (KJV) text.
    pub const SAMPLE: &str = r#"{"data":{"id":"JHN.3","copyright":"Test copyright notice","content":[
      {"name":"para","type":"tag","attrs":{"style":"s1"},"items":[{"text":"Jesus and Nicodemus","type":"text"}]},
      {"name":"para","type":"tag","attrs":{"style":"p"},"items":[
        {"name":"verse","type":"tag","attrs":{"number":"16","style":"v","sid":"JHN 3:16"},"items":[{"text":"16","type":"text"}]},
        {"text":"For God so loved the world, ","type":"text","attrs":{"verseId":"JHN.3.16"}},
        {"name":"char","type":"tag","attrs":{"style":"wj"},"items":[{"text":"that he gave","type":"text"}]},
        {"name":"note","type":"tag","attrs":{"style":"f"},"items":[{"text":"a footnote","type":"text"}]},
        {"name":"verse","type":"tag","attrs":{"number":"17-18","style":"v"},"items":[{"text":"17-18","type":"text"}]},
        {"text":"For God sent not his Son.","type":"text"}]},
      {"name":"para","type":"tag","attrs":{"style":"q1"},"items":[
        {"name":"verse","type":"tag","attrs":{"number":"19","style":"v"},"items":[{"text":"19","type":"text"}]},
        {"text":"And this is the condemnation,","type":"text"}]},
      {"name":"para","type":"tag","attrs":{"style":"q2"},"items":[{"text":"that light is come.","type":"text"}]}
    ]},"meta":{"fumsToken":"tok123"}}"#;

    #[test]
    fn parses_json_tree() {
        let v: Value = serde_json::from_str(SAMPLE).unwrap();
        let c = ChapterIdx::new(BookId(42), 3).unwrap();
        let t = parse_json_content(c, v["data"]["content"].as_array().unwrap());
        let v16 = t.find(VerseRef::new(BookId(42), 3, 16)).unwrap();
        assert_eq!(v16.text, "For God so loved the world, that he gave");
        assert_eq!(
            v16.marks.iter().map(|m| m.kind).collect::<Vec<_>>(),
            [MarkKind::Heading, MarkKind::Paragraph]
        );
        assert_eq!(v16.spans[0].style, Style::WordsOfJesus);
        let v17 = t.find(VerseRef::new(BookId(42), 3, 18)).unwrap();
        assert_eq!((v17.number, v17.number_end), (17, 18));
        let v19 = t.find(VerseRef::new(BookId(42), 3, 19)).unwrap();
        assert_eq!(v19.plain_text(), "And this is the condemnation, that light is come.");
        assert_eq!(
            v19.marks.iter().map(|m| (m.kind, m.level)).collect::<Vec<_>>(),
            [(MarkKind::Poetry, 1), (MarkKind::Poetry, 2)]
        );
    }
}
