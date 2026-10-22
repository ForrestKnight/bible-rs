//! NLT API (api.nlt.to): the New Living Translation, © Tyndale House Foundation.
//!
//! Tyndale serves the NLT to non-commercial applications without a key (no more than 50 verses a
//! request and 500 requests a day, counted per IP address) or with a free key (500 verses and
//! 5,000 requests). Chapters longer than a request allows are fetched in consecutive ranges.
//!
//! The HTML that comes back is read by `crate::nlt_html`, which the web build shares.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::Result;

use super::{BookShare, CachePolicy, DailyBudget, FetchedChapter, Source, http_agent, read_body};
use crate::books::{BookId, ChapterIdx};
use crate::model::{TranslationBuilder, TranslationMeta};
pub use crate::nlt_html::{COPYRIGHT, parse_html, parse_into};

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

#[cfg(test)]
mod tests {
    use super::*;

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
