//! Public-domain translations compiled into the binary (see `LICENSES/` for provenance).
//!
//! Only the compressed `.bbl` bytes are embedded; a translation is decoded the first time it is
//! used (≈10 ms), so startup only pays for the one being read.

use anyhow::Result;

use crate::format;
use crate::model::{Translation, TranslationMeta};

macro_rules! bbl {
    ($id:literal) => {
        (
            $id,
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/translations/", $id, ".bbl"))
                as &[u8],
        )
    };
}

/// `(id, compressed bytes)`, in the order they are offered to the reader.
pub static BUNDLED: &[(&str, &[u8])] =
    &[bbl!("kjv"), bbl!("web"), bbl!("bsb"), bbl!("asv"), bbl!("ylt"), bbl!("dby"), bbl!("wbs")];

pub const DEFAULT_ID: &str = "kjv";

pub fn bytes(id: &str) -> Option<&'static [u8]> {
    BUNDLED.iter().find(|(i, _)| *i == id).map(|(_, b)| *b)
}

pub fn load(id: &str) -> Result<Translation> {
    let b = bytes(id).ok_or_else(|| anyhow::anyhow!("no bundled translation {id:?}"))?;
    format::decode(b)
}

pub fn metas() -> Vec<TranslationMeta> {
    BUNDLED.iter().filter_map(|(_, b)| format::read_meta(b).ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::{BookId, ChapterIdx, VerseRef};
    use crate::model::Origin;

    #[test]
    fn every_bundled_translation_is_complete_and_public_domain() {
        for (id, _) in BUNDLED {
            let t = load(id).unwrap();
            assert_eq!(t.meta.id, *id);
            assert_eq!(t.meta.origin, Origin::Bundled);
            assert!(t.meta.license.starts_with("Public Domain"), "{id}: {}", t.meta.license);
            assert!(t.verse_count() > 31_000, "{id} has {} verses", t.verse_count());
            for c in ChapterIdx::all() {
                assert!(t.has_chapter(c), "{id} is missing {}", c.display());
            }
            let first = t.find(VerseRef::new(BookId::GENESIS, 1, 1)).unwrap();
            assert!(first.text.contains("beginning"), "{id}: {}", first.text);
        }
    }

    #[test]
    fn red_letters_where_the_source_has_them() {
        let kjv = load("kjv").unwrap();
        assert!(kjv.meta.red_letters);
        let v = kjv.find(VerseRef::new(BookId(42), 3, 16)).unwrap();
        assert!(v.spans.iter().any(|s| s.style == crate::model::Style::WordsOfJesus));
        assert_eq!(
            v.text,
            "For God so loved the world, that he gave his only begotten Son, that whosoever believeth in him should not perish, but have everlasting life."
        );
    }
}
