//! Startup-path costs: decoding a bundled translation, reading metadata, parsing references.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

fn bench(c: &mut Criterion) {
    let bytes = bible_core::bundled::bytes("kjv").unwrap();
    c.bench_function("load/decode KJV (.bbl → model)", |b| {
        b.iter(|| black_box(bible_core::format::decode(black_box(bytes)).unwrap()))
    });
    c.bench_function("load/read metadata only", |b| {
        b.iter(|| black_box(bible_core::format::read_meta(black_box(bytes)).unwrap()))
    });
    c.bench_function("load/all 7 bundled metadata", |b| b.iter(|| black_box(bible_core::bundled::metas())));
    c.bench_function("reference/parse \"1co13:4-7\"", |b| {
        b.iter(|| black_box(bible_core::reference::parse(black_box("1co13:4-7"))))
    });
    c.bench_function("reference/suggest typo \"revalation 21\"", |b| {
        b.iter(|| black_box(bible_core::reference::suggest(black_box("revalation 21"), 6)))
    });
    let dir = std::env::temp_dir().join(format!("bible-rs-bench-{}", std::process::id()));
    c.bench_function("study/open SQLite store", |b| {
        b.iter(|| black_box(bible_core::study::StudyStore::open(&dir.join("study.sqlite")).unwrap()))
    });
    let _ = std::fs::remove_dir_all(&dir);
}

criterion_group!(benches, bench);
criterion_main!(benches);
