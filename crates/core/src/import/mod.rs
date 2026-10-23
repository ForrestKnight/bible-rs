//! Importing user-supplied translations: USFM (file, folder or zip), OSIS XML, Zefania XML and
//! SWORD modules (folder, `.conf` file or zip). Imported text is converted to the common model
//! and saved as a `.bbl` file in the user's data directory — never anywhere in this repository.

pub mod osis;
pub mod sword;
pub mod zefania;

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU16, Ordering};

use anyhow::{Context, Result, bail, ensure};

use crate::model::{Origin, Translation, TranslationBuilder, TranslationMeta};
use crate::usfm::UsfmParser;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Usfm,
    Osis,
    Zefania,
    Sword,
}

impl Format {
    pub fn name(self) -> &'static str {
        match self {
            Format::Usfm => "USFM",
            Format::Osis => "OSIS XML",
            Format::Zefania => "Zefania XML",
            Format::Sword => "SWORD module",
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ImportOptions {
    pub id: Option<String>,
    pub abbrev: Option<String>,
    pub name: Option<String>,
    /// Unlock key for locked SWORD modules.
    pub unlock_key: Option<String>,
}

pub struct Imported {
    pub translation: Translation,
    pub format: Format,
    pub warnings: Vec<String>,
    /// Books in the source outside the 66-book canon, which were left out.
    pub skipped_books: Vec<String>,
}

/// An import's progress, read by the thread that shows it.
#[derive(Debug, Default)]
pub struct Progress {
    /// The step under way: 0 reading the file, 1 detecting its format, 2 converting books,
    /// 3 checking the result.
    pub step: AtomicU8,
    /// Books begun so far, and the latest (`BookId` index + 1; 0 before the first).
    pub books: AtomicU16,
    pub book: AtomicU8,
    /// Set to give up: the import stops with an error once conversion ends.
    pub cancel: AtomicBool,
    /// Called as each book begins, for a caller with no other thread to watch from (the browser).
    pub on_book: Option<fn(&Progress)>,
}

impl Progress {
    pub fn step(&self) -> u8 {
        self.step.load(Ordering::Relaxed)
    }

    pub fn books(&self) -> u16 {
        self.books.load(Ordering::Relaxed)
    }

    /// The book being converted.
    pub fn book(&self) -> Option<crate::books::BookId> {
        self.book.load(Ordering::Relaxed).checked_sub(1).map(crate::books::BookId)
    }
}

/// What an import source is, read before importing it (the import sheet's summary).
#[derive(Debug, Clone)]
pub struct Peek {
    pub format: Format,
    /// The file or folder name.
    pub label: String,
    /// Size on disk.
    pub bytes: u64,
    pub abbrev: String,
    pub name: String,
    /// A SWORD module that needs an unlock key.
    pub locked: bool,
    /// Further facts for the summary ("zText", "KJV versification").
    pub details: Vec<String>,
}

/// Reads an import source's format and names without converting it.
pub fn peek(path: &Path) -> Result<Peek> {
    let src = load_source(path)?;
    let bytes = if path.is_dir() {
        src.files.values().map(|f| f.len() as u64).sum()
    } else {
        std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    };
    peek_source(src, bytes)
}

/// [`peek`] for files already in memory, keyed by path relative to the source (a single file's
/// name, or a folder's contents); a lone zip is opened. `label` names the source.
pub fn peek_files(files: HashMap<String, Vec<u8>>, label: &str) -> Result<Peek> {
    let bytes = files.values().map(|f| f.len() as u64).sum();
    peek_source(source_of(files, label)?, bytes)
}

fn peek_source(src: Source, bytes: u64) -> Result<Peek> {
    let format = detect(&src)?;
    let stem =
        Path::new(&src.label).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let mut abbrev = String::new();
    let mut name = String::new();
    let mut locked = false;
    let mut details = Vec::new();
    match format {
        Format::Sword => {
            if let Some((_, conf)) =
                src.files.iter().find(|(k, _)| k.starts_with("mods.d/") && k.ends_with(".conf"))
                && let Ok(conf) = sword::SwordConf::parse(&text_of(conf))
            {
                name = conf.description();
                abbrev = conf.get("Abbreviation").unwrap_or(&conf.module).to_string();
                locked = conf.is_locked() && conf.get("CipherKey").is_none_or(str::is_empty);
                if let Some(d) = conf.get("ModDrv") {
                    let d = d.to_ascii_lowercase();
                    details.push(if d.starts_with("ztext") { "zText".into() } else { "rawText".into() });
                }
                details.push(format!("{} versification", conf.get("Versification").unwrap_or("KJV")));
            }
        }
        Format::Osis | Format::Zefania => {
            // The title in the header, if there is one near the top.
            for bytes in src.files.values() {
                let head = String::from_utf8_lossy(&bytes[..bytes.len().min(64 << 10)]);
                let lower = head.to_ascii_lowercase();
                if let Some(a) = lower.find("<title")
                    && let Some(open) = lower[a..].find('>')
                    && let Some(close) = lower[a + open..].find("</title>")
                {
                    name = head[a + open + 1..a + open + close].trim().to_string();
                    break;
                }
            }
        }
        Format::Usfm => {}
    }
    if abbrev.is_empty() {
        abbrev = stem.chars().take(8).collect::<String>().to_uppercase();
    }
    if name.is_empty() {
        name = stem;
    }
    Ok(Peek { format, label: src.label, bytes, abbrev, name, locked, details })
}

const MAX_TOTAL: u64 = 512 << 20;

/// The files of an import source, held in memory, keyed by relative path.
struct Source {
    files: HashMap<String, Vec<u8>>,
    label: String,
}

fn load_source(path: &Path) -> Result<Source> {
    let label = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "import".into());
    let mut files = HashMap::new();
    if path.is_dir() {
        let mut total = 0u64;
        walk(path, path, &mut files, &mut total)?;
    } else if is_zip(path)? {
        files = unzip(std::fs::File::open(path)?)?;
    } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("conf")) {
        // A SWORD .conf: the module root is the parent of `mods.d`.
        let root =
            path.parent().and_then(Path::parent).context("a SWORD .conf must live in a mods.d folder")?;
        let mut total = 0u64;
        walk(root, root, &mut files, &mut total)?;
        files.retain(|k, _| {
            !k.starts_with("mods.d/") || k.ends_with(&*path.file_name().unwrap().to_string_lossy())
        });
    } else {
        files.insert(
            label.clone(),
            std::fs::read(path).with_context(|| format!("reading {}", path.display()))?,
        );
    }
    Ok(Source { files, label })
}

/// The source for files in memory: a lone zip's contents, or the files as given.
fn source_of(mut files: HashMap<String, Vec<u8>>, label: &str) -> Result<Source> {
    if files.len() == 1 && files.values().all(|b| b.starts_with(ZIP_MAGIC)) {
        let (_, bytes) = files.drain().next().unwrap();
        files = unzip(std::io::Cursor::new(bytes))?;
    }
    Ok(Source { files, label: label.to_string() })
}

fn unzip(reader: impl std::io::Read + std::io::Seek) -> Result<HashMap<String, Vec<u8>>> {
    let mut archive = zip::ZipArchive::new(reader).context("reading zip archive")?;
    let mut files = HashMap::new();
    let mut total = 0u64;
    for i in 0..archive.len() {
        let mut f = archive.by_index(i)?;
        if !f.is_file() {
            continue;
        }
        let declared = f.size();
        let left = MAX_TOTAL.saturating_sub(total);
        if declared > left {
            bail!("archive is larger than 512 MB uncompressed");
        }
        total += declared;
        // The declared size is only a claim: never read more than the budget left, whatever
        // the entry really inflates to.
        let mut buf = Vec::with_capacity((declared as usize).min(1 << 20));
        (&mut f).take(left + 1).read_to_end(&mut buf)?;
        if buf.len() as u64 > left {
            bail!("archive is larger than 512 MB uncompressed");
        }
        files.insert(f.name()?.replace('\\', "/"), buf);
    }
    Ok(files)
}

fn walk(root: &Path, dir: &Path, out: &mut HashMap<String, Vec<u8>>, total: &mut u64) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            walk(root, &p, out, total)?;
        } else {
            *total += entry.metadata()?.len();
            if *total > MAX_TOTAL {
                bail!("folder is larger than 512 MB");
            }
            let rel = p.strip_prefix(root).unwrap_or(&p).to_string_lossy().replace('\\', "/");
            out.insert(rel, std::fs::read(&p)?);
        }
    }
    Ok(())
}

const ZIP_MAGIC: &[u8; 4] = b"PK\x03\x04";

fn is_zip(path: &Path) -> Result<bool> {
    let mut magic = [0u8; 4];
    let mut f = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    Ok(f.read(&mut magic)? == 4 && magic == *ZIP_MAGIC)
}

fn text_of(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        // Older files are often Windows-1252/Latin-1.
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

fn detect(src: &Source) -> Result<Format> {
    if src.files.keys().any(|k| k.starts_with("mods.d/") && k.ends_with(".conf")) {
        return Ok(Format::Sword);
    }
    let lower = |k: &str| k.to_ascii_lowercase();
    if src.files.keys().any(|k| [".usfm", ".sfm", ".ptx"].iter().any(|e| lower(k).ends_with(e))) {
        return Ok(Format::Usfm);
    }
    for (name, bytes) in &src.files {
        let head = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]).to_ascii_lowercase();
        if head.contains("<osis") {
            return Ok(Format::Osis);
        }
        if head.contains("<xmlbible") {
            return Ok(Format::Zefania);
        }
        if head.trim_start_matches('\u{feff}').trim_start().starts_with("\\id ") {
            return Ok(Format::Usfm);
        }
        let _ = name;
    }
    bail!("unrecognized format: expected USFM, OSIS XML, Zefania XML or a SWORD module")
}

pub fn slug(s: &str) -> String {
    let s: String = s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase();
    if s.is_empty() { "imported".into() } else { s.chars().take(16).collect() }
}

/// Imports a translation from `path` (file, folder or zip).
pub fn import_path(path: &Path, opts: &ImportOptions) -> Result<Imported> {
    import_with_progress(path, opts, &Arc::new(Progress::default()))
}

/// [`import_path`], reporting its steps and books to `progress`.
pub fn import_with_progress(path: &Path, opts: &ImportOptions, progress: &Arc<Progress>) -> Result<Imported> {
    progress.step.store(0, Ordering::Relaxed);
    let src = load_source(path)?;
    import_source(src, opts, progress)
}

/// [`import_with_progress`] for files already in memory, as [`peek_files`] takes them.
pub fn import_files(
    files: HashMap<String, Vec<u8>>,
    label: &str,
    opts: &ImportOptions,
    progress: &Arc<Progress>,
) -> Result<Imported> {
    progress.step.store(0, Ordering::Relaxed);
    import_source(source_of(files, label)?, opts, progress)
}

fn import_source(src: Source, opts: &ImportOptions, progress: &Arc<Progress>) -> Result<Imported> {
    progress.step.store(1, Ordering::Relaxed);
    let format = detect(&src)?;
    progress.step.store(2, Ordering::Relaxed);
    let stem =
        Path::new(&src.label).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let mut meta = TranslationMeta { language: "en".into(), ..Default::default() };
    let mut b = TranslationBuilder::new(meta.clone());
    let p = progress.clone();
    b.on_book = Some(Box::new(move |book| {
        p.books.fetch_add(1, Ordering::Relaxed);
        p.book.store(book.0 + 1, Ordering::Relaxed);
        if let Some(f) = p.on_book {
            f(&p);
        }
    }));
    let mut detected_name = String::new();
    let mut detected_abbrev = String::new();
    let mut rights = String::new();
    match format {
        Format::Usfm => {
            let mut names: Vec<&String> = src
                .files
                .keys()
                .filter(|k| {
                    let l = k.to_ascii_lowercase();
                    [".usfm", ".sfm", ".ptx", ".txt"].iter().any(|e| l.ends_with(e)) || src.files.len() == 1
                })
                .collect();
            names.sort();
            for n in names {
                let text = text_of(&src.files[n]);
                if text.trim_start().starts_with("\\id") || text.contains("\n\\id ") {
                    UsfmParser::new(&mut b).parse(&text)?;
                }
            }
        }
        Format::Osis | Format::Zefania => {
            let (_, bytes) = src
                .files
                .iter()
                .find(|(_, b)| {
                    let head = String::from_utf8_lossy(&b[..b.len().min(4096)]).to_ascii_lowercase();
                    head.contains("<osis") || head.contains("<xmlbible")
                })
                .context("no XML document found")?;
            let text = text_of(bytes);
            if format == Format::Osis {
                let mut h = osis::OsisHandler::new(&mut b, false);
                h.feed(&text);
                detected_name = h.work.title.clone();
                detected_abbrev = h.work.abbrev.clone();
                rights = h.work.rights.clone();
            } else {
                let info = zefania::read(&mut b, &text);
                detected_name = info.title;
                detected_abbrev = info.abbrev;
                rights = info.rights;
            }
        }
        Format::Sword => {
            let (conf_name, conf_bytes) =
                src.files.iter().find(|(k, _)| k.starts_with("mods.d/") && k.ends_with(".conf")).unwrap();
            let conf = sword::SwordConf::parse(&text_of(conf_bytes)).with_context(|| conf_name.clone())?;
            detected_name = conf.description();
            detected_abbrev = conf.get("Abbreviation").unwrap_or(&conf.module).to_string();
            rights = conf.copyright();
            let files = sword::MemFiles(src.files.clone());
            sword::read_module(&mut b, &conf, &files, opts.unlock_key.as_deref())?;
        }
    }
    ensure!(!progress.cancel.load(Ordering::Relaxed), "import cancelled");
    progress.step.store(3, Ordering::Relaxed);
    let warnings = std::mem::take(&mut b.warnings);
    let skipped_books = std::mem::take(&mut b.skipped_books);
    let mut t = b.finish();
    if t.verse_count() == 0 {
        bail!("no verses of the 66-book canon were found in {}", src.label);
    }
    let abbrev = opts
        .abbrev
        .clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| (!detected_abbrev.is_empty()).then(|| detected_abbrev.clone()))
        .unwrap_or_else(|| stem.chars().take(8).collect::<String>().to_uppercase());
    let name = opts
        .name
        .clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| (!detected_name.is_empty()).then(|| detected_name.clone()))
        .unwrap_or_else(|| stem.clone());
    meta = TranslationMeta {
        id: opts.id.clone().unwrap_or_else(|| slug(&abbrev)),
        abbrev,
        name,
        license: "Imported by you for personal use; not distributed with bible-rs".into(),
        copyright: rights,
        source: format!("{} ({})", src.label, format.name()),
        origin: Origin::Imported { format: format.name().into(), file_name: src.label.clone() },
        ..t.meta.clone()
    };
    t.meta = meta;
    Ok(Imported { translation: t, format, warnings, skipped_books })
}

/// Where imported translations are stored inside a data directory.
pub fn imports_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("translations")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::{BookId, VerseRef};

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("bible-rs-import-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn imports_usfm_folder_and_zip() {
        let d = tmp("usfm");
        std::fs::write(d.join("43JHN.usfm"), "\\id JHN\n\\c 3\n\\p\n\\v 16 For God so loved the world.")
            .unwrap();
        std::fs::write(d.join("01GEN.SFM"), "\\id GEN\n\\c 1\n\\p\n\\v 1 In the beginning.").unwrap();
        let imp =
            import_path(&d, &ImportOptions { abbrev: Some("Test".into()), ..Default::default() }).unwrap();
        assert_eq!(imp.format, Format::Usfm);
        assert_eq!(imp.translation.verse_count(), 2);
        assert_eq!(imp.translation.meta.id, "test");
        assert!(matches!(imp.translation.meta.origin, Origin::Imported { .. }));

        let zpath = d.join("bible.zip");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&zpath).unwrap());
            z.start_file("x/43JHN.usfm", zip::write::SimpleFileOptions::default()).unwrap();
            std::io::Write::write_all(&mut z, b"\\id JHN\n\\c 1\n\\p\n\\v 1 In the beginning was the Word.")
                .unwrap();
            z.finish().unwrap();
        }
        let imp = import_path(&zpath, &ImportOptions::default()).unwrap();
        assert_eq!(imp.translation.meta.abbrev, "BIBLE");
        assert_eq!(
            imp.translation.find(VerseRef::new(BookId(42), 1, 1)).unwrap().text,
            "In the beginning was the Word."
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn reports_progress_skipped_books_and_peeks() {
        let d = tmp("progress");
        std::fs::write(d.join("43JHN.usfm"), "\\id JHN\n\\c 3\n\\p\n\\v 16 For God so loved the world.")
            .unwrap();
        std::fs::write(d.join("68TOB.usfm"), "\\id TOB\n\\c 1\n\\p\n\\v 1 Tobit.").unwrap();
        std::fs::write(d.join("00FRT.usfm"), "\\id FRT\n\\p Front matter").unwrap();
        let peek = peek(&d).unwrap();
        assert_eq!(peek.format, Format::Usfm);
        assert!(!peek.locked);
        let progress = Arc::new(Progress::default());
        let imp = import_with_progress(&d, &ImportOptions::default(), &progress).unwrap();
        assert_eq!(imp.skipped_books, vec!["TOB".to_string()]);
        assert_eq!((progress.step(), progress.books()), (3, 1));
        assert_eq!(progress.book(), Some(BookId(42)));
        let cancelled = Arc::new(Progress::default());
        cancelled.cancel.store(true, Ordering::Relaxed);
        assert!(import_with_progress(&d, &ImportOptions::default(), &cancelled).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn imports_files_in_memory() {
        let usfm = b"\\id JHN\n\\c 3\n\\p\n\\v 16 For God so loved the world.".to_vec();
        let files = HashMap::from([("43JHN.usfm".to_string(), usfm.clone())]);
        let peek = peek_files(files.clone(), "Mine").unwrap();
        assert_eq!(
            (peek.format, peek.abbrev.as_str(), peek.bytes),
            (Format::Usfm, "MINE", usfm.len() as u64)
        );

        static BOOKS: AtomicU16 = AtomicU16::new(0);
        fn count(p: &Progress) {
            BOOKS.store(p.books(), Ordering::Relaxed);
        }
        let progress = Arc::new(Progress { on_book: Some(count), ..Default::default() });
        let imp = import_files(files, "Mine", &ImportOptions::default(), &progress).unwrap();
        assert_eq!((imp.translation.verse_count(), BOOKS.load(Ordering::Relaxed)), (1, 1));

        let mut zipped = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut zipped);
            z.start_file("x/43JHN.usfm", zip::write::SimpleFileOptions::default()).unwrap();
            std::io::Write::write_all(&mut z, &usfm).unwrap();
            z.finish().unwrap();
        }
        let files = HashMap::from([("bible.zip".to_string(), zipped.into_inner())]);
        let imp = import_files(files, "bible.zip", &ImportOptions::default(), &Arc::default()).unwrap();
        assert_eq!((imp.translation.meta.abbrev.as_str(), imp.translation.verse_count()), ("BIBLE", 1));
    }

    #[test]
    fn imports_osis_and_zefania_files() {
        let d = tmp("xml");
        let osis = d.join("t.xml");
        std::fs::write(&osis, r#"<osis><osisText><header><work><title>My OSIS</title></work></header><div type="book" osisID="Gen"><chapter osisID="Gen.1"><verse osisID="Gen.1.1">In the beginning</verse></chapter></div></osisText></osis>"#).unwrap();
        let imp = import_path(&osis, &ImportOptions::default()).unwrap();
        assert_eq!((imp.format, imp.translation.meta.name.as_str()), (Format::Osis, "My OSIS"));
        let zef = d.join("z.xml");
        std::fs::write(&zef, r#"<XMLBIBLE><BIBLEBOOK bnumber="1"><CHAPTER cnumber="1"><VERS vnumber="1">In the beginning</VERS></CHAPTER></BIBLEBOOK></XMLBIBLE>"#).unwrap();
        let imp =
            import_path(&zef, &ImportOptions { name: Some("Zef".into()), ..Default::default() }).unwrap();
        assert_eq!((imp.format, imp.translation.meta.name.as_str()), (Format::Zefania, "Zef"));
        let bad = d.join("bad.txt");
        std::fs::write(&bad, "hello").unwrap();
        assert!(import_path(&bad, &ImportOptions::default()).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn imports_sword_folder_and_conf() {
        let d = tmp("sword");
        let files = sword::tests::build_module(
            &[(VerseRef::new(BookId(42), 3, 16), "For God so loved the world.")],
            false,
            None,
            "Abbreviation=TSW\nShortCopyright=Public domain test module\n",
        );
        for (k, v) in &files.0 {
            let p = d.join(k);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, v).unwrap();
        }
        let imp = import_path(&d, &ImportOptions::default()).unwrap();
        assert_eq!(imp.format, Format::Sword);
        assert_eq!(imp.translation.meta.abbrev, "TSW");
        assert_eq!(imp.translation.meta.name, "Test Module");
        assert_eq!(imp.translation.meta.copyright, "Public domain test module");
        let imp = import_path(&d.join("mods.d/test.conf"), &ImportOptions::default()).unwrap();
        assert_eq!(imp.translation.verse_count(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }
}
