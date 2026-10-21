//! Importing a translation the reader brings (built with the `import` feature, into a module of
//! its own that only the import worker loads, so readers don't download the importers).

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use bible_core::{BookId, format, import};
use js_sys::{Function, Reflect, Uint8Array};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::{TranslationInfo, err, to_js};

/// What a picked file (or folder, or zip) holds, read before converting it: the import sheet's
/// summary. `paths` are relative to what was picked; `contents` match them.
#[wasm_bindgen(js_name = peekImport)]
pub fn peek_import(paths: Vec<String>, contents: Vec<Uint8Array>, label: &str) -> Result<JsValue, JsError> {
    let p = import::peek_files(picked(paths, contents), label).map_err(err)?;
    to_js(&PeekInfo {
        format: p.format.name(),
        sword: p.format == import::Format::Sword,
        label: p.label,
        bytes: p.bytes,
        abbrev: p.abbrev,
        name: p.name,
        locked: p.locked,
        details: p.details,
    })
}

/// Converts a picked source into a `.bbl` file for the page to keep, given an id not in `taken`.
/// `onBook(books, book)` hears of each book as it begins. The result carries the file's bytes,
/// its metadata, and what needs a look: books missing or left out, and verses adjusted.
#[wasm_bindgen(js_name = importTranslation)]
pub fn import_translation(
    paths: Vec<String>,
    contents: Vec<Uint8Array>,
    label: &str,
    options: JsValue,
    taken: Vec<String>,
    on_book: Option<Function>,
) -> Result<JsValue, JsError> {
    let o: ImportArgs = serde_wasm_bindgen::from_value(options)?;
    let opts = import::ImportOptions {
        abbrev: o.abbrev.filter(|s| !s.trim().is_empty()),
        name: o.name.filter(|s| !s.trim().is_empty()),
        unlock_key: o.unlock_key.filter(|s| !s.trim().is_empty()),
        ..Default::default()
    };
    ON_BOOK.with(|f| *f.borrow_mut() = on_book);
    let progress = Arc::new(import::Progress { on_book: Some(report_book), ..Default::default() });
    let result = import::import_files(picked(paths, contents), label, &opts, &progress);
    ON_BOOK.with(|f| *f.borrow_mut() = None);
    let mut imp = result.map_err(err)?;
    let t = &mut imp.translation;
    let base = if t.meta.id.is_empty() { "imported".to_string() } else { t.meta.id.clone() };
    let mut id = base.clone();
    let mut n = 2;
    while taken.contains(&id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    t.meta.id = id;
    let bytes = format::encode(t, 9).map_err(err)?;
    let out = to_js(&ImportResult {
        info: TranslationInfo::new(&t.meta, format::decoded_len(&bytes)),
        format: imp.format.name(),
        verses: t.verse_count(),
        missing: (0..66).filter(|b| !t.has_book(BookId(*b))).collect(),
        skipped_books: imp.skipped_books,
        warnings: imp.warnings,
    })?;
    Reflect::set(&out, &"bytes".into(), &Uint8Array::from(&bytes[..]))
        .map_err(|_| JsError::new("couldn't hand over the file"))?;
    Ok(out)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PeekInfo {
    format: &'static str,
    sword: bool,
    label: String,
    bytes: u64,
    abbrev: String,
    name: String,
    locked: bool,
    details: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportArgs {
    abbrev: Option<String>,
    name: Option<String>,
    unlock_key: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportResult {
    info: TranslationInfo,
    format: &'static str,
    verses: usize,
    missing: Vec<u8>,
    skipped_books: Vec<String>,
    warnings: Vec<String>,
}

thread_local! {
    static ON_BOOK: RefCell<Option<Function>> = const { RefCell::new(None) };
}

fn report_book(p: &import::Progress) {
    ON_BOOK.with(|f| {
        if let Some(f) = &*f.borrow() {
            let book = p.book().map_or(-1, |b| i32::from(b.0));
            let _ = f.call2(&JsValue::NULL, &p.books().into(), &book.into());
        }
    });
}

fn picked(paths: Vec<String>, contents: Vec<Uint8Array>) -> HashMap<String, Vec<u8>> {
    paths.into_iter().zip(contents).map(|(p, c)| (p, c.to_vec())).collect()
}
