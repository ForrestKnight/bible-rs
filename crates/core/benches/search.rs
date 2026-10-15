//! Full-text search latency on the bundled KJV (31,102 verses, 4.1 MB of text).

use std::hint::black_box;

use bible_core::search::{self, Scope};
use criterion::{Criterion, criterion_group, criterion_main};

fn bench(c: &mut Criterion) {
    let kjv = bible_core::bundled::load("kjv").unwrap();
    c.bench_function("search/build index (KJV)", |b| {
        b.iter(|| black_box(bible_core::search::SearchIndex::build(&kjv)))
    });
    kjv.search_index();
    let mut g = c.benchmark_group("search");
    for (name, q, scope) in [
        ("common word \"the\"", "the", Scope::All),
        ("prefix \"lov\"", "lov", Scope::All),
        ("rare word \"melchizedek\"", "melchizedek", Scope::All),
        ("three words \"love one another\"", "love one another", Scope::All),
        ("phrase \"in the beginning\"", "\"in the beginning\"", Scope::All),
        ("phrase \"jesus wept\"", "\"jesus wept\"", Scope::All),
        ("exclusion \"light -darkness\"", "light -darkness", Scope::All),
        ("NT only \"faith\"", "faith", Scope::NewTestament),
        ("single book \"shepherd\" in Psalms", "shepherd", Scope::Book(bible_core::BookId::PSALMS)),
    ] {
        g.bench_function(name, |b| {
            b.iter(|| black_box(search::run(&kjv, black_box(q), scope, false).verses.len()))
        });
    }
    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
