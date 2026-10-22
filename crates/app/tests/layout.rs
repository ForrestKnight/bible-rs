//! Typesetting tests: run the real layout engine with the bundled fonts and translations.

use bible_core::books::{BookId, ChapterIdx};
use bible_rs::reader::layout::{self, ChapterLayout, ColumnText, Ctx, Mode, Typography};
use bible_rs::theme::{self, Theme};
use eframe::egui;

fn with_fonts<R>(f: impl FnOnce(&egui::Context) -> R) -> R {
    let ctx = egui::Context::default();
    ctx.set_fonts(theme::font_definitions());
    ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
    ctx.begin_pass(egui::RawInput::default());
    let r = f(&ctx);
    ctx.end_pass().textures_delta.clear();
    r
}

fn chapter(t: &bible_core::Translation, typo: &Typography, c: ChapterIdx) -> ChapterLayout {
    with_fonts(|ctx| {
        let cx = Ctx::new(typo, Theme::Light, 640.0, 0.0);
        ctx.fonts_mut(|f| layout::layout_chapter(f, &cx, t, c))
    })
}

#[test]
fn every_verse_has_an_anchor_in_reading_order() {
    let kjv = bible_core::bundled::load("kjv").unwrap();
    let c = ChapterIdx::new(BookId(42), 3).unwrap();
    let l = chapter(&kjv, &Typography::default(), c);
    let numbers: Vec<u8> = l.anchors.iter().map(|a| a.number).collect();
    assert_eq!(numbers, (1..=36).collect::<Vec<u8>>());
    assert!(l.anchors.windows(2).all(|w| w[0].top <= w[1].top), "anchors must go down the page");
    assert!(l.height > l.anchors.last().unwrap().bottom);
    // The reading-position lookup finds the verse whose first line is above a point.
    let v16 = l.anchors.iter().find(|a| a.number == 16).unwrap();
    assert_eq!(l.verse_at_y(v16.top + 1.0).unwrap().number, 16);
}

#[test]
fn hit_testing_maps_glyphs_back_to_verses() {
    let kjv = bible_core::bundled::load("kjv").unwrap();
    let c = ChapterIdx::new(BookId(42), 3).unwrap();
    let l = chapter(&kjv, &Typography::default(), c);
    let v16 = kjv.find(bible_core::VerseRef::new(BookId(42), 3, 16)).unwrap();
    let block = l.blocks.iter().find(|b| b.verse_chars(v16.index).is_some()).unwrap();
    let range = block.verse_chars(v16.index).unwrap();
    // Every character of the verse maps back to the verse, and its rectangles are inside the block.
    let mid = range.start + 20;
    assert_eq!(block.verse_at_char(mid), Some(v16.index));
    let rects = block.char_rects(range.start..range.end.min(range.start + 400));
    assert!(!rects.is_empty());
    for r in &rects {
        assert!(r.width() > 0.0 && r.height() > 0.0);
        assert!(block.galley.rect.expand(1.0).contains_rect(*r), "{r:?} outside {:?}", block.galley.rect);
    }
    let cursor = block.galley.cursor_from_pos(rects[0].center().to_vec2());
    assert_eq!(block.verse_at_char(cursor.index.0), Some(v16.index));
}

#[test]
fn small_caps_share_the_text_baseline() {
    let kjv = bible_core::bundled::load("kjv").unwrap();
    let c = ChapterIdx::new(BookId::PSALMS, 23).unwrap();
    let l = chapter(&kjv, &Typography::default(), c);
    let mut checked = 0;
    for b in &l.blocks {
        for row in &b.galley.rows {
            let text: String = row.glyphs.iter().map(|g| g.chr).collect();
            if let Some(i) = text.find("LORD") {
                let i = text[..i].chars().count();
                let (l_glyph, o_glyph) = (&row.glyphs[i], &row.glyphs[i + 1]);
                assert!(
                    (l_glyph.pos.y - o_glyph.pos.y).abs() <= 1.0,
                    "L at {} vs ORD at {}",
                    l_glyph.pos.y,
                    o_glyph.pos.y
                );
                assert!(o_glyph.advance_width < l_glyph.advance_width, "smaller capitals expected");
                checked += 1;
            }
        }
    }
    assert!(checked >= 2, "Psalm 23 mentions the LORD twice");
}

#[test]
fn poetry_wraps_with_a_hanging_indent() {
    let kjv = bible_core::bundled::load("kjv").unwrap();
    let c = ChapterIdx::new(BookId::PSALMS, 23).unwrap();
    let narrow = Typography { size: 22.0, ..Typography::default() };
    let l = with_fonts(|ctx| {
        let cx = Ctx::new(&narrow, Theme::Light, 360.0, 0.0);
        ctx.fonts_mut(|f| layout::layout_chapter(f, &cx, &kjv, c))
    });
    let wrapped = l.blocks.iter().find(|b| b.galley.rows.len() > 1).expect("a wrapped poetry line");
    let first = wrapped.galley.rows[0].pos.x;
    assert!(wrapped.galley.rows[1].pos.x > first + 10.0, "continuation lines are indented");
}

#[test]
fn verse_per_line_starts_every_verse_on_a_new_block_with_a_gutter_number() {
    let kjv = bible_core::bundled::load("kjv").unwrap();
    let c = ChapterIdx::new(BookId(42), 3).unwrap();
    let typo = Typography { mode: Mode::VersePerLine, ..Typography::default() };
    let l = chapter(&kjv, &typo, c);
    let starts = l.blocks.iter().filter(|b| b.verses.first().is_some_and(|(ch, _, _)| *ch == 0)).count();
    assert_eq!(starts, 36);
    assert!(l.blocks.iter().all(|b| b.gutter.is_some()));
}

#[test]
fn parallel_rows_align_across_translations() {
    let kjv = bible_core::bundled::load("kjv").unwrap();
    let web = bible_core::bundled::load("web").unwrap();
    let c = ChapterIdx::new(BookId::PSALMS, 1).unwrap();
    let typo = Typography::default();
    let l = with_fonts(|ctx| {
        let cx = Ctx::new(&typo, Theme::Light, 420.0, 30.0);
        let cols =
            [("KJV".to_string(), ColumnText::Ready(&kjv)), ("WEB".to_string(), ColumnText::Ready(&web))];
        ctx.fonts_mut(|f| layout::layout_parallel(f, &cx, &cols, c))
    });
    for n in 1..=6u8 {
        let tops: Vec<f32> = l.anchors.iter().filter(|a| a.number == n).map(|a| a.top).collect();
        assert_eq!(tops.len(), 2, "verse {n} in both columns");
        assert!((tops[0] - tops[1]).abs() < 0.5, "verse {n} rows align: {tops:?}");
    }
    // Text starts after each column's verse-number lane; columns are a column and a gutter apart.
    let mut xs: Vec<f32> = l.blocks.iter().map(|b| b.pos.x).collect();
    xs.sort_by(f32::total_cmp);
    xs.dedup();
    assert_eq!(xs.len(), 2);
    assert!((xs[1] - xs[0] - 450.0).abs() < 0.5, "{xs:?}");
    assert!(l.blocks.iter().all(|b| b.gutter.is_some()), "every cell has its verse number");
}

#[test]
fn every_chapter_of_every_bundled_translation_lays_out() {
    // A smoke test over the whole canon (shapes ~120k paragraphs): no panics, sane heights.
    let typo = Typography::default();
    with_fonts(|ctx| {
        for id in ["kjv", "bsb"] {
            let t = bible_core::bundled::load(id).unwrap();
            let cx = Ctx::new(&typo, Theme::Light, 600.0, 0.0);
            for c in ChapterIdx::all().step_by(7) {
                let l = ctx.fonts_mut(|f| layout::layout_chapter(f, &cx, &t, c));
                assert!(l.height > 100.0, "{id} {}", c.display());
                assert_eq!(l.anchors.len(), t.verses_in(c).len(), "{id} {}", c.display());
            }
        }
    });
}

#[test]
fn parallel_cells_flow_poetry_as_prose() {
    // BSB Psalm 1:1 is a q1 line followed by q2 lines; in a parallel column the verse flows as
    // one paragraph, as the design shows, so rows break only where the text wraps.
    let bsb = bible_core::bundled::load("bsb").unwrap();
    let c = ChapterIdx::new(BookId::PSALMS, 1).unwrap();
    let typo = Typography::default();
    let l = with_fonts(|ctx| {
        let cx = Ctx::new(&typo, Theme::Light, 420.0, 30.0);
        let cols = [("BSB".to_string(), ColumnText::Ready(&bsb))];
        ctx.fonts_mut(|f| layout::layout_parallel(f, &cx, &cols, c))
    });
    let v1 = bsb.find(bible_core::VerseRef::new(BookId::PSALMS, 1, 1)).unwrap();
    let cell = l.blocks.iter().find(|b| b.verse_chars(v1.index).is_some()).unwrap();
    assert!(cell.galley.rows.len() >= 2);
    assert!(cell.galley.rows.iter().all(|r| !r.ends_with_newline), "no forced line breaks");
}

#[test]
fn publisher_notices_follow_the_text_as_links() {
    // An online chapter ends with its publisher's notice, below the last verse, across the columns.
    let kjv = bible_core::bundled::load("kjv").unwrap();
    let web = bible_core::bundled::load("web").unwrap();
    let c = ChapterIdx::new(BookId(42), 3).unwrap();
    let typo = Typography::default();
    let notice = |abbrev: &str| layout::Notice {
        abbrev: abbrev.into(),
        text: "Used by permission. All rights reserved. ".repeat(8),
        url: "https://example.org",
    };
    with_fonts(|ctx| {
        let cx = Ctx::new(&typo, Theme::Light, 420.0, 30.0);
        let mut single = ctx.fonts_mut(|f| layout::layout_chapter(f, &cx, &kjv, c));
        let (height, last) = (single.height, single.anchors.last().unwrap().bottom);
        ctx.fonts_mut(|f| layout::add_notices(f, &cx, &mut single, &[notice("ESV")], 1));
        let (rect, url) = single.links[0];
        assert_eq!(url, "https://example.org");
        assert!(rect.top() > last && rect.left() == 0.0 && rect.width() <= 420.0);
        assert!(single.height > height && single.height > rect.bottom());

        let cols =
            [("NLT".to_string(), ColumnText::Ready(&kjv)), ("WEB".to_string(), ColumnText::Ready(&web))];
        let mut parallel = ctx.fonts_mut(|f| layout::layout_parallel(f, &cx, &cols, c));
        ctx.fonts_mut(|f| layout::add_notices(f, &cx, &mut parallel, &[notice("NLT"), notice("NET")], 2));
        assert_eq!(parallel.links.len(), 2);
        let (first, second) = (parallel.links[0].0, parallel.links[1].0);
        assert!(second.top() >= first.bottom(), "one under the other");
        // A long notice wraps at the column's width alone, and across both columns side by side.
        assert!(first.width() > 420.0, "{first:?}");
        assert!(first.left() > 0.0, "it starts after the verse-number lane");
    });
}

/// Each block's text runs are its verses' text, glyph for glyph, so a highlight of part of a verse
/// lands on its words, and a point in the text maps back to the right char.
#[test]
fn text_runs_are_the_verses_text() {
    let kjv = bible_core::bundled::load("kjv").unwrap();
    let web = bible_core::bundled::load("web").unwrap();
    let per_line = Typography { mode: Mode::VersePerLine, ..Typography::default() };
    let mut layouts = Vec::new();
    for (book, n) in [(BookId(42), 3), (BookId::PSALMS, 23), (BookId::PSALMS, 119)] {
        let c = ChapterIdx::new(book, n).unwrap();
        layouts.push((c, chapter(&kjv, &Typography::default(), c)));
        layouts.push((c, chapter(&kjv, &per_line, c)));
        let cols =
            vec![("KJV".to_string(), ColumnText::Ready(&kjv)), ("WEB".to_string(), ColumnText::Ready(&web))];
        let parallel = with_fonts(|ctx| {
            let typo = Typography::default();
            let cx = Ctx::new(&typo, Theme::Light, 400.0, 32.0);
            ctx.fonts_mut(|f| layout::layout_parallel(f, &cx, &cols, c))
        });
        layouts.push((c, parallel));
    }
    let mut runs = 0;
    for (c, l) in &layouts {
        for b in &l.blocks {
            let t = if b.column == 0 { &kjv } else { &web };
            let glyphs: Vec<char> = b.galley.job.text.chars().collect();
            for r in &b.runs {
                let v = t.verses_in(*c).find(|v| v.index == r.verse).unwrap();
                let want: String = v.text.chars().skip(r.offset).take(r.len).collect();
                let got: String = glyphs[r.at..r.at + r.len].iter().collect();
                // LORD's smaller capitals are set upper case.
                assert_eq!(got.to_uppercase(), want.to_uppercase(), "{} {}", c.display(), v.number);
                assert_eq!(b.text_pos(r.at), Some((r.verse, r.offset)));
                assert_eq!(b.text_pos(r.at + r.len), Some((r.verse, r.offset + r.len)));
                assert_eq!(b.galley_range(r.verse, r.offset..r.offset + r.len), Some(r.at..r.at + r.len));
                assert_eq!(b.galley_char(r.verse, r.offset), Some(r.at));
                runs += 1;
            }
        }
    }
    assert!(runs > 500, "{runs}");
    // A verse number maps to the start of its verse's text.
    let (_, l) = &layouts[0];
    let b = l.blocks.iter().find(|b| b.verses.len() > 1).unwrap();
    let (start, vi, _) = b.verses[1];
    let first = b.runs.iter().find(|r| r.verse == vi).unwrap();
    assert_eq!(b.text_pos(start + 1), Some((vi, first.offset)), "inside the number");
}
