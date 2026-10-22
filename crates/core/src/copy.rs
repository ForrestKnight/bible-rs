//! Formatting verses for the clipboard: text with its reference and translation.

use crate::books::VerseRef;
use crate::highlight::TextSpan;
use crate::model::Translation;
use crate::reference::format_range;

/// The words a text selection covers, its verses' parts joined by spaces.
pub fn span_text(t: &Translation, span: &TextSpan) -> Option<String> {
    let parts: Vec<String> = span
        .refs()
        .into_iter()
        .filter_map(|r| t.find(r).map(|v| v.plain_slice(span.range_of(r))))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    (!parts.is_empty()).then(|| parts.join(" "))
}

/// A text selection with its reference: `“so loved the world” — John 3:16 (KJV)`.
pub fn format_span(t: &Translation, span: &TextSpan) -> Option<String> {
    let text = span_text(t, span)?;
    Some(format!("“{text}” — {} ({})", format_range(span.first, span.last), t.meta.abbrev))
}

/// One verse: `“text” — John 3:16 (KJV)`. Several: a reference line, then numbered verses.
pub fn format_verses(t: &Translation, refs: &[VerseRef]) -> Option<String> {
    let mut verses: Vec<(VerseRef, String)> =
        refs.iter().filter_map(|r| t.find(*r).map(|v| (v.reference(), v.plain_text()))).collect();
    verses.dedup_by_key(|(r, _)| *r);
    let first = verses.first()?.0;
    let last = verses.last()?.0;
    let abbrev = &t.meta.abbrev;
    if verses.len() == 1 {
        return Some(format!("“{}” — {} ({abbrev})", verses[0].1, first));
    }
    let body: Vec<String> = verses.iter().map(|(r, text)| format!("{} {text}", r.verse)).collect();
    Some(format!("{} ({abbrev})\n{}", format_range(first, last), body.join(" ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::BookId;
    use crate::model::{TranslationBuilder, TranslationMeta};

    #[test]
    fn single_and_range() {
        let mut b = TranslationBuilder::new(TranslationMeta { abbrev: "KJV".into(), ..Default::default() });
        b.begin_book(BookId(42));
        b.begin_chapter(3);
        b.begin_verse(16, 16);
        b.text("For God so loved the world.");
        b.begin_verse(17, 17);
        b.text("For God sent not his Son.");
        let t = b.finish();
        let v16 = VerseRef::new(BookId(42), 3, 16);
        let v17 = VerseRef::new(BookId(42), 3, 17);
        assert_eq!(format_verses(&t, &[v16]).unwrap(), "“For God so loved the world.” — John 3:16 (KJV)");
        assert_eq!(
            format_verses(&t, &[v16, v17]).unwrap(),
            "John 3:16–17 (KJV)\n16 For God so loved the world. 17 For God sent not his Son."
        );
        assert!(format_verses(&t, &[VerseRef::new(BookId(0), 1, 1)]).is_none());
        let span = TextSpan { first: v16, start: 4, last: v17, end: 12 };
        assert_eq!(span_text(&t, &span).unwrap(), "God so loved the world. For God sent");
        assert_eq!(
            format_span(&t, &span).unwrap(),
            "“God so loved the world. For God sent” — John 3:16–17 (KJV)"
        );
        let one = TextSpan { first: v16, start: 7, last: v16, end: 16 };
        assert_eq!(format_span(&t, &one).unwrap(), "“so loved” — John 3:16 (KJV)");
    }

    #[test]
    fn a_line_break_in_the_words_is_a_space() {
        let mut b = TranslationBuilder::new(TranslationMeta { abbrev: "KJV".into(), ..Default::default() });
        b.begin_book(BookId::PSALMS);
        b.begin_chapter(23);
        b.begin_verse(1, 1);
        b.text("The LORD is my shepherd;");
        b.mark(crate::model::MarkKind::Poetry, 2);
        b.text("I shall not want.");
        let t = b.finish();
        let v = VerseRef::new(BookId::PSALMS, 23, 1);
        let span = TextSpan { first: v, start: 15, last: v, end: 31 };
        assert_eq!(span_text(&t, &span).unwrap(), "shepherd; I shall");
    }
}
