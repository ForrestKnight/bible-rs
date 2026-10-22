//! Core of bible-rs: the verse-indexed translation model, the `.bbl` binary format, importers,
//! full-text search, reference parsing and local study storage. No GUI code lives here.

pub mod books;
pub mod bundled;
pub mod content;
pub mod copy;
pub mod format;
pub mod highlight;
#[cfg(feature = "import")]
pub mod import;
#[cfg(feature = "native")]
pub mod library;
pub mod model;
pub mod nlt_html;
#[cfg(feature = "native")]
pub mod online;
pub mod plans;
pub mod reference;
pub mod search;
#[cfg(feature = "native")]
pub mod secrets;
pub mod study;
pub mod usfm;
pub mod xml;

pub use books::{BookId, ChapterIdx, Testament, VerseRef};
pub use model::{Translation, TranslationMeta};
