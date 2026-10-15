//! Chapter typesetting cost: what scrolling into a chapter that isn't laid out yet costs the UI
//! thread (shaping with HarfRust, line breaking, glyph placement, our block building).

use std::hint::black_box;

use bible_core::books::{BookId, ChapterIdx};
use bible_rs::reader::layout::{self, ColumnText, Ctx, Mode, Typography};
use bible_rs::theme::{self, Theme};
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use eframe::egui;

fn bench(c: &mut Criterion) {
    let ctx = egui::Context::default();
    ctx.set_fonts(theme::font_definitions());
    ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
    ctx.begin_pass(egui::RawInput::default());
    let kjv = bible_core::bundled::load("kjv").unwrap();
    let web = bible_core::bundled::load("web").unwrap();
    let typo = Typography::default();
    let per_line = Typography { mode: Mode::VersePerLine, ..Typography::default() };
    // egui caches galleys that were used in the previous pass. Ending the pass and alternating
    // between two widths (outside the timed region) makes every timed layout a cache miss.
    let mut flip = false;
    let mut fresh = || {
        ctx.end_pass().textures_delta.clear();
        ctx.begin_pass(egui::RawInput::default());
        flip = !flip;
        if flip { 640.0 } else { 641.0 }
    };
    let mut g = c.benchmark_group("layout");
    for (name, book, ch) in [
        ("John 3 (36 verses)", BookId(42), 3),
        ("Genesis 1 (31 verses)", BookId::GENESIS, 1),
        ("Psalm 23 (poetry)", BookId::PSALMS, 23),
        ("Psalm 119 (176 verses, longest)", BookId::PSALMS, 119),
    ] {
        let c_idx = ChapterIdx::new(book, ch).unwrap();
        g.bench_function(format!("paragraphs/{name}"), |b| {
            b.iter_batched(
                &mut fresh,
                |w| {
                    let cx = Ctx::new(&typo, Theme::Light, w, 0.0);
                    ctx.fonts_mut(|f| black_box(layout::layout_chapter(f, &cx, &kjv, c_idx)))
                },
                BatchSize::PerIteration,
            )
        });
    }
    let john3 = ChapterIdx::new(BookId(42), 3).unwrap();
    g.bench_function("verse-per-line/John 3", |b| {
        b.iter_batched(
            &mut fresh,
            |w| {
                let cx = Ctx::new(&per_line, Theme::Light, w, 0.0);
                ctx.fonts_mut(|f| black_box(layout::layout_chapter(f, &cx, &kjv, john3)))
            },
            BatchSize::PerIteration,
        )
    });
    g.bench_function("parallel KJV+WEB/John 3", |b| {
        b.iter_batched(
            &mut fresh,
            |w| {
                let cx = Ctx::new(&typo, Theme::Light, w / 2.0, 34.0);
                let cols = [
                    ("KJV".to_string(), ColumnText::Ready(&kjv)),
                    ("WEB".to_string(), ColumnText::Ready(&web)),
                ];
                ctx.fonts_mut(|f| black_box(layout::layout_parallel(f, &cx, &cols, john3)))
            },
            BatchSize::PerIteration,
        )
    });
    g.finish();
    ctx.end_pass().textures_delta.clear();
}

criterion_group!(benches, bench);
criterion_main!(benches);
