use std::path::PathBuf;
use std::sync::Arc;

use bible_ffi::*;

/// A fresh data folder per test.
fn open(name: &str) -> (Arc<Bible>, PathBuf) {
    let dir = std::env::temp_dir().join(format!("bible-ffi-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    (Bible::new(dir.to_string_lossy().into()).unwrap(), dir)
}

fn index(reference: &str) -> u16 {
    parse_reference(reference.into()).unwrap().chapter_index
}

fn text(b: &Block) -> String {
    b.runs.iter().map(|r| r.text.as_str()).collect()
}

const JOHN_3_16: VerseRef = VerseRef { book: 42, chapter: 3, verse: 16 };

#[test]
fn books_and_references() {
    let books = books();
    assert_eq!(books.len(), 66);
    assert_eq!((books[42].name.as_str(), books[42].chapters), ("John", 21));
    assert!(!books[38].new_testament && books[39].new_testament);
    assert_eq!(books[1].first_chapter, 50);

    let r = parse_reference("jn 3:16".into()).unwrap();
    assert_eq!((r.book, r.chapter, r.verse, r.label.as_str()), (42, 3, Some(16), "John 3:16"));
    assert_eq!(chapter_label(r.chapter_index), "John 3");
    assert_eq!(parse_reference("1co13:4-7".into()).unwrap().label, "1 Corinthians 13:4–7");
    assert_eq!(parse_reference("revalation 21".into()).unwrap().label, "Revelation 21");
    assert!(parse_reference("xyzzy".into()).is_none());
    assert_eq!(suggest_references("jo 3".into(), 3)[0].label, "John 3");
    assert_eq!(range_label(VerseRef { verse: 18, ..JOHN_3_16 }, JOHN_3_16), "John 3:16–18");
}

#[test]
fn translations_list_the_bundled_texts() {
    let (bible, dir) = open("translations");
    let list = bible.translations();
    let ids: Vec<&str> = list.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, ["kjv", "web", "bsb", "asv", "ylt", "dby", "wbs"]);
    assert!(
        list.iter().all(|t| t.origin == TranslationOrigin::Bundled && t.license.starts_with("Public Domain"))
    );
    assert_eq!((list[0].abbrev.as_str(), list[0].year), ("KJV", Some(1769)));
    assert!(list[0].red_letters && list[2].headings);
    bible.prepare("kjv".into()).unwrap();
    assert!(bible.prepare("nlt".into()).is_err());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn prose_has_inline_numbers_and_red_letters() {
    let (bible, dir) = open("prose");
    let c = bible.chapter("kjv".into(), index("jn 3"), ContentOptions::default());
    assert_eq!(c.status, ChapterStatus::Ready);
    assert_eq!((c.book, c.chapter, c.book_name.as_str()), (42, 3, "John"));
    assert_eq!(c.book_title, "The Gospel According to St. John");
    let first = &c.blocks[0];
    assert_eq!(first.kind, BlockKind::Prose { indent: false });
    assert_eq!(first.space_before, 16.0);
    assert_eq!(first.runs[0], Run { text: "1\u{a0}".into(), verse: 1, number: true, ..plain() });
    // Later paragraphs are indented and 16 pt apart.
    assert!(
        c.blocks[1..].iter().all(|b| b.kind == BlockKind::Prose { indent: true } && b.space_before == 16.0)
    );
    let v16: Vec<&Run> = c.blocks.iter().flat_map(|b| &b.runs).filter(|r| r.verse == 16).collect();
    assert_eq!(v16[0].text, "16\u{a0}");
    assert!(v16[1].red && v16[1].text.starts_with("For God so loved the world"));
    // Translator additions are italic.
    let added: Vec<&str> =
        c.blocks.iter().flat_map(|b| &b.runs).filter(|r| r.italic).map(|r| r.text.as_str()).collect();
    assert!(added.contains(&"of") && added.contains(&"must"), "{added:?}");
    let _ = std::fs::remove_dir_all(dir);
}

fn plain() -> Run {
    Run {
        text: String::new(),
        verse: 0,
        number: false,
        red: false,
        italic: false,
        bold: false,
        small_caps: false,
        offset: 0,
        added: false,
    }
}

#[test]
fn psalms_hang_numbers_and_set_lord_in_small_caps() {
    let (bible, dir) = open("psalm");
    let c = bible.chapter("kjv".into(), index("ps 23"), ContentOptions::default());
    assert_eq!(c.blocks[0].kind, BlockKind::PsalmTitle);
    assert_eq!((text(&c.blocks[0]).as_str(), c.blocks[0].space_before), ("A Psalm of David.", 0.0));
    let line = &c.blocks[1];
    assert_eq!((line.kind, line.space_before), (BlockKind::Poetry { level: 1 }, 16.0));
    assert_eq!((line.gutter.as_deref(), line.gutter_verse), (Some("1"), 1));
    assert!(line.runs.iter().all(|r| !r.number));
    assert_eq!(text(line), "The LORD is my shepherd; I shall not want.");
    let lord: Vec<(&str, bool)> = line.runs.iter().take(2).map(|r| (r.text.as_str(), r.small_caps)).collect();
    assert_eq!(lord, [("The L", false), ("ORD", true)]);

    // The BSB opens a psalm with a heading over its title; second lines are indented a level.
    let c = bible.chapter("bsb".into(), index("ps 3"), ContentOptions::default());
    let kinds: Vec<BlockKind> = c.blocks.iter().take(5).map(|b| b.kind).collect();
    assert_eq!(
        kinds,
        [
            BlockKind::PsalmHeading,
            BlockKind::PsalmTitle,
            BlockKind::Poetry { level: 1 },
            BlockKind::Poetry { level: 2 },
            BlockKind::Poetry { level: 1 },
        ]
    );
    assert_eq!(c.blocks[3].gutter, None);
    assert!(c.blocks.iter().any(|b| b.kind == BlockKind::Right && text(b) == "Selah"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn headings_numbers_and_verse_per_line_follow_the_options() {
    let (bible, dir) = open("options");
    let gen1 = index("gen 1");
    let c = bible.chapter("bsb".into(), gen1, ContentOptions::default());
    assert_eq!(c.blocks[0].kind, BlockKind::Heading { level: 1 });
    assert_eq!((text(&c.blocks[0]).as_str(), c.blocks[0].space_before), ("The Creation", 24.0));
    // The paragraph after a heading: the heading's 8 pt plus the paragraph gap, and no indent.
    assert_eq!((c.blocks[1].kind, c.blocks[1].space_before), (BlockKind::Prose { indent: false }, 24.0));

    let bare = bible.chapter("bsb".into(), gen1, ContentOptions { headings: false, ..Default::default() });
    assert!(bare.blocks.iter().all(|b| !matches!(b.kind, BlockKind::Heading { .. })));
    assert_eq!(bare.blocks[0].space_before, 16.0);

    let per_line = ContentOptions { verse_per_line: true, ..Default::default() };
    let c = bible.chapter("web".into(), index("jn 3"), per_line);
    assert_eq!(c.blocks.len(), 36);
    for (i, b) in c.blocks.iter().enumerate() {
        assert_eq!(b.gutter.as_deref(), Some((i + 1).to_string().as_str()));
        assert_eq!(b.space_before, if i == 0 { 16.0 } else { 8.0 });
    }

    let no_numbers = ContentOptions { verse_numbers: false, ..per_line };
    let c = bible.chapter("web".into(), index("jn 3"), no_numbers);
    assert!(c.blocks.iter().all(|b| b.gutter.is_none() && b.runs.iter().all(|r| !r.number)));
    let _ = std::fs::remove_dir_all(dir);
}

/// Every verse's words arrive, in order, in runs marked with its number.
#[test]
fn every_chapter_keeps_all_its_text() {
    let (bible, dir) = open("complete");
    let squash =
        |s: &str| s.chars().filter(|c| !c.is_whitespace()).flat_map(char::to_lowercase).collect::<String>();
    for id in ["kjv", "bsb"] {
        let t = bible_core::bundled::load(id).unwrap();
        for c in bible_core::ChapterIdx::all() {
            for options in
                [ContentOptions::default(), ContentOptions { verse_per_line: true, ..Default::default() }]
            {
                let content = bible.chapter(id.into(), c.0, options);
                for v in t.verses_in(c) {
                    let got: String = content
                        .blocks
                        .iter()
                        .flat_map(|b| &b.runs)
                        .filter(|r| r.verse == v.number && !r.number)
                        .map(|r| r.text.as_str())
                        .collect();
                    assert_eq!(squash(&got), squash(&v.plain_text()), "{id} {}:{}", c.display(), v.number);
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn search_pages_and_utf16_match_ranges() {
    let (bible, dir) = open("search");
    let page = bible.search("kjv".into(), "love one another".into(), SearchScope::All, false, 0, 5).unwrap();
    assert!(page.total > 5 && page.hits.len() == 5, "{}", page.total);
    assert!(page.elapsed_ms < 1_000.0);
    let rest = bible
        .search("kjv".into(), "love one another".into(), SearchScope::All, false, page.total - 2, 5)
        .unwrap();
    assert_eq!(rest.hits.len(), 2);

    // The WEB's curly quotes are three bytes in UTF-8 and one unit in UTF-16.
    let john = SearchScope::Book { book: 42 };
    let page = bible.search("web".into(), "kingdom".into(), john, false, 0, 100).unwrap();
    let hit = page.hits.iter().find(|h| h.reference == "John 3:3").unwrap();
    assert!(hit.text.contains('“'));
    let units: Vec<u16> = hit.text.encode_utf16().collect();
    let m = hit.matches[0];
    let matched = String::from_utf16(&units[m.start as usize..(m.start + m.length) as usize]).unwrap();
    assert_eq!(matched, "Kingdom");
    assert_eq!((hit.verse, hit.chapter_index), (VerseRef { book: 42, chapter: 3, verse: 3 }, index("jn 3")));

    let nt = bible.search("kjv".into(), "faith".into(), SearchScope::NewTestament, false, 0, 1).unwrap();
    let ot = bible.search("kjv".into(), "faith".into(), SearchScope::OldTestament, false, 0, 1).unwrap();
    assert!(nt.total > ot.total && nt.hits[0].verse.book >= 39);
    assert!(bible.search("nlt".into(), "love".into(), SearchScope::All, false, 0, 1).is_err());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_search_that_finds_nothing_suggests_a_spelling() {
    let (bible, dir) = open("suggest");
    let page = bible.search("kjv".into(), "bethsaida pool".into(), SearchScope::All, true, 0, 10).unwrap();
    assert_eq!(page.total, 0);
    let s = page.suggestion.unwrap();
    assert_eq!((s.query.as_str(), s.total), ("bethesda pool", 1));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn study_marks_follow_the_verse_and_persist() {
    let (bible, dir) = open("study");
    let jn3 = index("jn 3");
    assert!(bible.marks("kjv".into(), jn3).is_empty());
    bible.set_highlight(JOHN_3_16, Some(HighlightColor::Green)).unwrap();
    bible.set_note(JOHN_3_16, "God's love".into()).unwrap();
    bible.set_bookmark(VerseRef { verse: 1, ..JOHN_3_16 }, true).unwrap();
    bible.set_setting("ios".into(), "{\"theme\":\"sepia\"}".into()).unwrap();
    assert!(bible.set_highlight(VerseRef { book: 66, ..JOHN_3_16 }, None).is_err());
    drop(bible);

    let bible = Bible::new(dir.to_string_lossy().into()).unwrap();
    let marks = bible.marks("web".into(), jn3);
    assert_eq!(
        marks,
        [
            VerseMarks { verse: 1, highlight: None, note: false, bookmark: true },
            VerseMarks { verse: 16, highlight: Some(HighlightColor::Green), note: true, bookmark: false },
        ]
    );
    assert_eq!(bible.note(JOHN_3_16).unwrap().as_deref(), Some("God's love"));
    assert_eq!(bible.setting("ios".into()).as_deref(), Some("{\"theme\":\"sepia\"}"));

    let notes = bible.study_items(StudyKind::Notes, "web".into());
    assert_eq!((notes[0].reference.as_str(), notes[0].body.as_str()), ("John 3:16", "God's love"));
    assert!(notes[0].text.starts_with("For God so loved the world"));
    assert_eq!(notes[0].highlight, Some(HighlightColor::Green));
    assert_eq!(bible.study_items(StudyKind::Bookmarks, "kjv".into())[0].verse.verse, 1);
    assert_eq!(bible.study_items(StudyKind::Highlights, "kjv".into())[0].chapter_index, jn3);

    let md = bible.export(ExportFormat::Markdown, "kjv".into()).unwrap();
    assert!(md.contains("### John 3:16") && md.contains("God's love"), "{md}");
    assert!(bible.export(ExportFormat::Json, "kjv".into()).unwrap().contains("\"color\": \"green\""));

    bible.set_note(JOHN_3_16, "  ".into()).unwrap();
    bible.set_highlight(JOHN_3_16, None).unwrap();
    bible.set_bookmark(VerseRef { verse: 1, ..JOHN_3_16 }, false).unwrap();
    assert!(bible.marks("kjv".into(), jn3).is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn copy_text_carries_the_reference() {
    let (bible, dir) = open("copy");
    let one = bible.copy_text("kjv".into(), vec![JOHN_3_16]).unwrap();
    assert!(one.starts_with("“For God so loved the world") && one.ends_with("” — John 3:16 (KJV)"), "{one}");
    let two = bible.copy_text("web".into(), vec![JOHN_3_16, VerseRef { verse: 17, ..JOHN_3_16 }]).unwrap();
    assert!(two.starts_with("John 3:16–17 (WEB)\n16 For God so loved"), "{two}");
    assert!(bible.copy_text("kjv".into(), vec![VerseRef { verse: 99, ..JOHN_3_16 }]).is_err());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn unknown_translations_and_chapters_fail_softly() {
    let (bible, dir) = open("missing");
    let c = bible.chapter("nlt".into(), index("jn 3"), ContentOptions::default());
    assert!(matches!(c.status, ChapterStatus::Failed { .. }) && c.blocks.is_empty());
    assert_eq!(c.book_name, "John");
    let last = bible.chapter("kjv".into(), u16::MAX, ContentOptions::default());
    assert_eq!((last.book, last.chapter), (65, 22));
    let _ = std::fs::remove_dir_all(dir);
}

/// Highlights of parts of a verse (made on the Mac or the web, or imported) list one row each on
/// the phone, told apart by where they start, and one can be removed alone.
#[test]
fn a_verse_with_several_highlights_lists_each() {
    let dir = std::env::temp_dir().join(format!("bible-ffi-parts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    {
        use bible_core::study::{HighlightColor as Colour, StudyStore, TextSpan};
        let kjv = bible_core::bundled::load("kjv").unwrap();
        let v = bible_core::VerseRef::new(bible_core::BookId(42), 3, 16);
        let text = |r: bible_core::VerseRef| kjv.find(r).map(|v| v.text.to_string());
        let mut s = StudyStore::open(&dir.join("study.sqlite")).unwrap();
        s.highlight_span(&TextSpan { first: v, start: 4, last: v, end: 26 }, Some(Colour::Yellow), text)
            .unwrap();
        s.highlight_span(&TextSpan { first: v, start: 96, last: v, end: 113 }, Some(Colour::Green), text)
            .unwrap();
    }
    let bible = Bible::new(dir.to_string_lossy().into()).unwrap();
    let items = bible.study_items(StudyKind::Highlights, "kjv".into());
    let rows: Vec<(u32, &str)> = items.iter().map(|i| (i.start, i.text.as_str())).collect();
    assert_eq!(rows.len(), 2);
    assert!(rows.contains(&(4, "…God so loved the world…")), "{rows:?}");
    assert!(rows.contains(&(96, "…should not perish…")), "{rows:?}");
    bible.remove_highlight(JOHN_3_16, 96).unwrap();
    let left = bible.study_items(StudyKind::Highlights, "kjv".into());
    assert_eq!((left.len(), left[0].start, left[0].highlight), (1, 4, Some(HighlightColor::Yellow)));
    let _ = std::fs::remove_dir_all(dir);
}
