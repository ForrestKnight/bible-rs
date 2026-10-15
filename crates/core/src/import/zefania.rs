//! Zefania XML reader (`<XMLBIBLE><BIBLEBOOK bnumber><CHAPTER cnumber><VERS vnumber>`).

use crate::books::BookId;
use crate::model::{MarkKind, Style, TranslationBuilder};
use crate::xml::{Tok, attr, tokens};

#[derive(Debug, Default, Clone)]
pub struct ZefaniaInfo {
    pub title: String,
    pub abbrev: String,
    pub rights: String,
    pub language: String,
}

pub fn read(b: &mut TranslationBuilder, src: &str) -> ZefaniaInfo {
    let mut info = ZefaniaInfo::default();
    let mut skip = 0usize;
    let mut skip_book = false;
    let mut collect: Option<(&'static str, String)> = None;
    let mut styles: Vec<(String, Option<Style>)> = Vec::new();
    let mut in_info = false;
    for tok in tokens(src) {
        match tok {
            Tok::Text(t) => {
                if skip > 0 || skip_book {
                    continue;
                }
                if let Some((_, buf)) = &mut collect {
                    buf.push_str(&t);
                } else {
                    b.text(&t);
                }
            }
            Tok::Start { name, attrs } | Tok::Empty { name, attrs } if skip > 0 => {
                let _ = attrs;
                if name.eq_ignore_ascii_case("NOTE") || name.eq_ignore_ascii_case("XREF") {
                    skip += 1;
                }
            }
            Tok::End { name } if skip > 0 => {
                if name.eq_ignore_ascii_case("NOTE") || name.eq_ignore_ascii_case("XREF") {
                    skip -= 1;
                }
            }
            Tok::Start { name, attrs } => {
                let upper = name.to_ascii_uppercase();
                match upper.as_str() {
                    "INFORMATION" => in_info = true,
                    "TITLE" if in_info => collect = Some(("title", String::new())),
                    "IDENTIFIER" if in_info => collect = Some(("abbrev", String::new())),
                    "RIGHTS" if in_info => collect = Some(("rights", String::new())),
                    "LANGUAGE" if in_info => collect = Some(("language", String::new())),
                    "BIBLEBOOK" => {
                        let n: usize =
                            attr(attrs, "bnumber").and_then(|n| n.trim().parse().ok()).unwrap_or(0);
                        match n.checked_sub(1).and_then(BookId::new) {
                            Some(book) => {
                                skip_book = false;
                                b.begin_book(book);
                                let short = attr(attrs, "bname");
                                b.set_book_title(book, short.as_deref(), None);
                            }
                            None => {
                                skip_book = true;
                                let name = attr(attrs, "bname").map(|s| s.to_string());
                                b.skip_book(&name.unwrap_or_else(|| format!("book {n}")));
                            }
                        }
                    }
                    "CHAPTER" if !skip_book => {
                        let n = attr(attrs, "cnumber").and_then(|n| n.trim().parse().ok()).unwrap_or(0);
                        b.begin_chapter(n);
                    }
                    "VERS" if !skip_book => {
                        let n: u32 = attr(attrs, "vnumber").and_then(|n| n.trim().parse().ok()).unwrap_or(0);
                        let end = attr(attrs, "vend")
                            .or_else(|| attr(attrs, "aix"))
                            .and_then(|n| n.trim().parse().ok())
                            .unwrap_or(n);
                        b.begin_verse(n, end);
                    }
                    "CAPTION" if !skip_book => collect = Some(("caption", String::new())),
                    "NOTE" | "XREF" | "MEDIA" => skip = 1,
                    "STYLE" => {
                        let css = attr(attrs, "css").unwrap_or_default().to_ascii_lowercase();
                        let fs = attr(attrs, "fs").unwrap_or_default().to_ascii_lowercase();
                        let style = if css.contains("#ff0000") || css.contains("color:red") || fs == "red" {
                            Some(Style::WordsOfJesus)
                        } else if fs == "italic" || css.contains("italic") {
                            Some(Style::Italic)
                        } else if fs == "bold" || css.contains("bold") {
                            Some(Style::Bold)
                        } else if fs == "divine-name" || css.contains("small-caps") {
                            Some(Style::DivineName)
                        } else {
                            None
                        };
                        if let Some(s) = style {
                            b.push_style(s);
                        }
                        styles.push((upper, style));
                    }
                    "DIV" => {}
                    _ => {}
                }
            }
            Tok::Empty { name, attrs } => {
                if name.eq_ignore_ascii_case("BR") {
                    let art = attr(attrs, "art").unwrap_or_default();
                    if art.contains("x-p") {
                        b.mark(MarkKind::Paragraph, 0);
                    } else {
                        b.mark(MarkKind::Poetry, 1);
                    }
                }
            }
            Tok::End { name } => {
                let upper = name.to_ascii_uppercase();
                match upper.as_str() {
                    "INFORMATION" => in_info = false,
                    "TITLE" | "IDENTIFIER" | "RIGHTS" | "LANGUAGE" | "CAPTION" => {
                        if let Some((field, text)) = collect.take() {
                            let text = text.trim().to_string();
                            match field {
                                "title" => info.title = text,
                                "abbrev" => info.abbrev = text,
                                "rights" => info.rights = text,
                                "language" => info.language = text,
                                "caption" => b.heading(MarkKind::Heading, 1, &text),
                                _ => {}
                            }
                        }
                    }
                    "VERS" => b.end_current_verse(),
                    "STYLE" => {
                        if let Some(i) = styles.iter().rposition(|(n, _)| *n == upper)
                            && let (_, Some(s)) = styles.remove(i)
                        {
                            b.pop_style(s);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    info
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::VerseRef;
    use crate::model::TranslationMeta;

    #[test]
    fn reads_zefania() {
        let src = r#"<?xml version="1.0"?>
<XMLBIBLE biblename="Test"><INFORMATION><title>Zef Test</title><identifier>ZT</identifier><rights>PD</rights></INFORMATION>
<BIBLEBOOK bnumber="43" bname="John"><CHAPTER cnumber="3">
<CAPTION>For God So Loved</CAPTION>
<VERS vnumber="16">For <gr str="2316">God</gr> so loved<NOTE>n</NOTE> the world, <STYLE css="color:#FF0000">that he gave</STYLE></VERS>
<VERS vnumber="17">Second<BR art="x-nl"/>line</VERS>
</CHAPTER></BIBLEBOOK>
<BIBLEBOOK bnumber="70"><CHAPTER cnumber="1"><VERS vnumber="1">Extra</VERS></CHAPTER></BIBLEBOOK>
</XMLBIBLE>"#;
        let mut b = TranslationBuilder::new(TranslationMeta::default());
        let info = read(&mut b, src);
        let t = b.finish();
        assert_eq!(
            (info.title.as_str(), info.abbrev.as_str(), info.rights.as_str()),
            ("Zef Test", "ZT", "PD")
        );
        let v = t.find(VerseRef::new(BookId(42), 3, 16)).unwrap();
        assert_eq!(v.text, "For God so loved the world, that he gave");
        assert_eq!(v.spans[0].style, Style::WordsOfJesus);
        assert_eq!(v.marks[0].kind, MarkKind::Heading);
        let v = t.find(VerseRef::new(BookId(42), 3, 17)).unwrap();
        assert_eq!(v.plain_text(), "Second line");
        assert_eq!(t.verse_count(), 2);
    }
}
