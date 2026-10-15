//! Core of bible-rs: the verse-indexed translation model, the `.bbl` binary format, importers,
//! full-text search, reference parsing and local study storage. No GUI code lives here.

pub mod books;
#[cfg(feature = "bundled")]
pub mod bundled;
pub mod format;
pub mod model;
pub mod reference;
pub mod search;
pub mod usfm;

pub use books::{BookId, ChapterIdx, Testament, VerseRef};
pub use model::{Translation, TranslationMeta};
