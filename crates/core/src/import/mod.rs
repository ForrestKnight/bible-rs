//! Importing user-supplied translations: USFM (file, folder or zip), OSIS XML, Zefania XML and
//! SWORD modules (folder, `.conf` file or zip). Imported text is converted to the common model
//! and saved as a `.bbl` file in the user's data directory — never anywhere in this repository.

pub mod osis;
pub mod sword;
pub mod zefania;

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

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
        let mut archive = zip::ZipArchive::new(std::fs::File::open(path)?).context("reading zip archive")?;
        let mut total = 0u64;
        for i in 0..archive.len() {
            let mut f = archive.by_index(i)?;
            if !f.is_file() {
                continue;
            }
            total += f.size();
            if total > MAX_TOTAL {
                bail!("archive is larger than 512 MB uncompressed");
            }
            let mut buf = Vec::with_capacity(f.size() as usize);
            f.read_to_end(&mut buf)?;
            files.insert(f.name().replace('\\', "/"), buf);
        }
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

fn is_zip(path: &Path) -> Result<bool> {
    let mut magic = [0u8; 4];
    let mut f = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    Ok(f.read(&mut magic)? == 4 && magic == *b"PK\x03\x04")
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
    let src = load_source(path)?;
    let format = detect(&src)?;
    let stem =
        Path::new(&src.label).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let mut meta = TranslationMeta { language: "en".into(), ..Default::default() };
    let mut b = TranslationBuilder::new(meta.clone());
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
    let warnings = std::mem::take(&mut b.warnings);
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
    Ok(Imported { translation: t, format, warnings })
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
