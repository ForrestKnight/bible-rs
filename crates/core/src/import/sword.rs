//! SWORD Bible modules (CrossWire format): `mods.d/<name>.conf` plus a data directory.
//!
//! Supported drivers: `zText`/`zText4` (zlib-compressed blocks) and `RawText`/`RawText4`, in KJV,
//! KJVA, NRSV or NRSVA versification, with OSIS, ThML, GBF or plain-text markup. Locked modules
//! (`CipherKey=`) are read with the user's unlock key using SWORD's Sapphire II cipher.

use std::collections::HashMap;
use std::io::Read;

use anyhow::{Context, Result, bail, ensure};

use super::osis::OsisHandler;
use crate::books::{BookId, ChapterIdx, FIRST_NT_BOOK};
use crate::model::{MarkKind, Style, TranslationBuilder};
use crate::xml::{Tok, attr, tokens};

/// A parsed `.conf` file.
#[derive(Debug, Clone, Default)]
pub struct SwordConf {
    pub module: String,
    values: HashMap<String, String>,
}

impl SwordConf {
    pub fn parse(text: &str) -> Result<Self> {
        let mut conf = SwordConf::default();
        let mut lines = text.lines();
        for line in lines.by_ref() {
            let line = line.trim();
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                conf.module = name.trim().to_string();
                break;
            }
        }
        ensure!(!conf.module.is_empty(), "not a SWORD .conf file (no [ModuleName] section)");
        let mut pending: Option<(String, String)> = None;
        for line in lines {
            let line = line.trim_end();
            if let Some((k, mut v)) = pending.take() {
                // Continuation of a value that ended in a backslash.
                v.push('\n');
                v.push_str(line.trim_end_matches('\\').trim());
                if line.ends_with('\\') {
                    pending = Some((k, v));
                } else {
                    conf.values.entry(k).or_insert(v);
                }
                continue;
            }
            if line.trim_start().starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            let key = k.trim().to_ascii_lowercase();
            if v.ends_with('\\') {
                pending = Some((key, v.trim_end_matches('\\').trim().to_string()));
            } else {
                conf.values.entry(key).or_insert_with(|| v.trim().to_string());
            }
        }
        if let Some((k, v)) = pending {
            conf.values.entry(k).or_insert(v);
        }
        Ok(conf)
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(&key.to_ascii_lowercase()).map(String::as_str)
    }

    pub fn description(&self) -> String {
        self.get("Description").map(strip_rtf).unwrap_or_else(|| self.module.clone())
    }

    pub fn copyright(&self) -> String {
        ["ShortCopyright", "Copyright", "DistributionLicense", "ShortPromo"]
            .iter()
            .filter_map(|k| self.get(k))
            .map(strip_rtf)
            .find(|s| !s.is_empty())
            .unwrap_or_default()
    }

    pub fn is_locked(&self) -> bool {
        self.values.contains_key("cipherkey")
    }
}

fn strip_rtf(s: &str) -> String {
    s.replace("\\par", " ").replace("\\pard", " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Read access to the module's files, relative to the module root (the directory holding `mods.d`).
pub trait ModuleFiles {
    fn read(&self, rel: &str) -> Result<Vec<u8>>;
}

/// Is `rel` a relative path that stays inside the module root (no `..`, no absolute path, no
/// drive letter)? A module's `.conf` is untrusted input.
fn is_confined(rel: &str) -> bool {
    let rel = rel.replace('\\', "/");
    !rel.starts_with('/') && !rel.contains(':') && !rel.split('/').any(|part| part == "..")
}

/// Upper bound on one decompressed zText block. Real modules use blocks of a few hundred KB.
const MAX_BLOCK: usize = 64 << 20;

pub struct DirFiles(pub std::path::PathBuf);

impl ModuleFiles for DirFiles {
    fn read(&self, rel: &str) -> Result<Vec<u8>> {
        ensure!(is_confined(rel), "module file {rel:?} points outside the module");
        let p = self.0.join(rel.trim_start_matches("./"));
        std::fs::read(&p).with_context(|| format!("reading {}", p.display()))
    }
}

/// Files held in memory (from a zip archive).
pub struct MemFiles(pub HashMap<String, Vec<u8>>);

impl ModuleFiles for MemFiles {
    fn read(&self, rel: &str) -> Result<Vec<u8>> {
        let rel = rel.trim_start_matches("./").to_ascii_lowercase();
        self.0
            .iter()
            .find(|(k, _)| k.to_ascii_lowercase().trim_start_matches("./") == rel)
            .map(|(_, v)| v.clone())
            .with_context(|| format!("module file {rel} not found in archive"))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Driver {
    ZText { size4: bool },
    RawText { size4: bool },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Markup {
    Osis,
    Thml,
    Gbf,
    Plain,
}

/// Reads every canonical verse of the module into the builder.
pub fn read_module(
    b: &mut TranslationBuilder,
    conf: &SwordConf,
    files: &dyn ModuleFiles,
    unlock_key: Option<&str>,
) -> Result<()> {
    let driver = match conf.get("ModDrv").unwrap_or("").to_ascii_lowercase().as_str() {
        "ztext" => Driver::ZText { size4: false },
        "ztext4" => Driver::ZText { size4: true },
        "rawtext" => Driver::RawText { size4: false },
        "rawtext4" => Driver::RawText { size4: true },
        other => bail!("unsupported SWORD driver {other:?} (only Bible text modules are supported)"),
    };
    let v11n = conf.get("Versification").unwrap_or("KJV");
    let verses_in = versification(v11n).with_context(|| {
        format!("SWORD versification {v11n:?} is not supported yet (KJV, KJVA, NRSV and NRSVA are)")
    })?;
    if let Driver::ZText { .. } = driver {
        let c = conf.get("CompressType").unwrap_or("ZIP");
        ensure!(c.eq_ignore_ascii_case("ZIP"), "SWORD compression {c:?} is not supported (ZIP is)");
    }
    let key = match (conf.get("CipherKey").filter(|k| !k.is_empty()), unlock_key.filter(|k| !k.is_empty())) {
        (_, Some(user)) => Some(user.to_string()),
        (Some(k), None) => Some(k.to_string()),
        (None, None) if conf.is_locked() => {
            bail!("this module is locked; enter the unlock key you received from the publisher")
        }
        (None, None) => None,
    };
    let markup = match conf.get("SourceType").unwrap_or("Plaintext").to_ascii_lowercase().as_str() {
        "osis" => Markup::Osis,
        "thml" => Markup::Thml,
        "gbf" => Markup::Gbf,
        _ => Markup::Plain,
    };
    let utf8 = conf.get("Encoding").is_some_and(|e| e.eq_ignore_ascii_case("UTF-8"));
    let data_path = conf.get("DataPath").context("conf has no DataPath")?.trim_end_matches('/').to_string();
    ensure!(is_confined(&data_path), "the module's DataPath {data_path:?} points outside the module");

    let mut cipher_check: Option<usize> = None;
    for (testament, books) in [("ot", 0..FIRST_NT_BOOK.0), ("nt", FIRST_NT_BOOK.0..66)] {
        let reader = match TestamentReader::open(files, &data_path, testament, driver, key.as_deref()) {
            Ok(r) => r,
            // A module may cover only one testament.
            Err(_) => continue,
        };
        // Index layout: [0] module heading, [1] testament heading, then per book a heading entry
        // and per chapter a heading entry followed by its verses (so Genesis 1:1 is entry 4).
        let mut idx = 1usize;
        let mut handler = OsisHandler::new(b, true);
        for book in books.map(BookId) {
            idx += 1;
            handler.b.begin_book(book);
            for ch in 1..=book.chapters() {
                idx += 1;
                let c = ChapterIdx::new(book, ch).unwrap();
                handler.b.begin_chapter(ch as u32);
                let n = verses_in(c);
                for v in 1..=n {
                    let raw = reader.entry(idx + v as usize)?;
                    if raw.iter().all(u8::is_ascii_whitespace) {
                        continue;
                    }
                    let text = if utf8 {
                        String::from_utf8_lossy(&raw).into_owned()
                    } else {
                        raw.iter().map(|&b| b as char).collect()
                    };
                    if key.is_some() && cipher_check.is_none() {
                        cipher_check = Some(0);
                        // A wrong key produces binary garbage; refuse rather than import noise.
                        let printable = text.chars().filter(|c| !c.is_control() || c.is_whitespace()).count();
                        ensure!(
                            printable * 10 >= text.chars().count() * 9,
                            "the unlock key is not correct for this module"
                        );
                    }
                    handler.b.begin_verse(v as u32, v as u32);
                    match markup {
                        Markup::Osis => {
                            handler.feed(&text);
                            handler.end_fragment();
                        }
                        Markup::Thml => thml(handler.b, &text),
                        Markup::Gbf => gbf(handler.b, &text),
                        Markup::Plain => handler.b.text(&text),
                    }
                }
                idx += n as usize;
            }
        }
    }
    Ok(())
}

/// Verses per chapter for a SWORD versification whose 66 canonical books are in KJV order.
/// (KJVA and NRSVA add the Apocrypha after Malachi, which doesn't move any canonical entry.)
/// Tables from SWORD's `canon.h` / `canon_nrsv.h`.
fn versification(name: &str) -> Option<fn(ChapterIdx) -> u8> {
    fn kjv(c: ChapterIdx) -> u8 {
        c.kjv_verses()
    }
    fn nrsv(c: ChapterIdx) -> u8 {
        // 3 John 1 and Revelation 12 each have one more verse than in the KJV.
        match c.0 {
            1165 => 15,
            1178 => 18,
            _ => c.kjv_verses(),
        }
    }
    match name.to_ascii_uppercase().as_str() {
        "KJV" | "KJVA" => Some(kjv),
        "NRSV" | "NRSVA" => Some(nrsv),
        _ => None,
    }
}

struct TestamentReader {
    driver: Driver,
    index: Vec<u8>,
    data: Vec<u8>,
    blocks: Vec<u8>,
    key: Option<String>,
    cache: std::cell::RefCell<Option<(usize, std::rc::Rc<Vec<u8>>)>>,
}

impl TestamentReader {
    fn open(files: &dyn ModuleFiles, dir: &str, t: &str, driver: Driver, key: Option<&str>) -> Result<Self> {
        let (index, data, blocks) = match driver {
            Driver::ZText { .. } => (
                files.read(&format!("{dir}/{t}.bzv"))?,
                files.read(&format!("{dir}/{t}.bzz"))?,
                files.read(&format!("{dir}/{t}.bzs"))?,
            ),
            Driver::RawText { .. } => {
                (files.read(&format!("{dir}/{t}.vss"))?, files.read(&format!("{dir}/{t}"))?, Vec::new())
            }
        };
        Ok(Self { driver, index, data, blocks, key: key.map(str::to_string), cache: Default::default() })
    }

    fn entry(&self, idx: usize) -> Result<Vec<u8>> {
        match self.driver {
            Driver::RawText { size4 } => {
                let w = if size4 { 8 } else { 6 };
                let Some(rec) = self.index.get(idx * w..idx * w + w) else { return Ok(Vec::new()) };
                let off = u32::from_le_bytes(rec[0..4].try_into().unwrap()) as usize;
                let len = if size4 {
                    u32::from_le_bytes(rec[4..8].try_into().unwrap()) as usize
                } else {
                    u16::from_le_bytes(rec[4..6].try_into().unwrap()) as usize
                };
                let mut raw = self.data.get(off..off + len).context("verse out of range")?.to_vec();
                if let Some(k) = &self.key {
                    Sapphire::new(k.as_bytes()).decrypt_in_place(&mut raw);
                }
                Ok(raw)
            }
            Driver::ZText { size4 } => {
                let w = if size4 { 12 } else { 10 };
                let Some(rec) = self.index.get(idx * w..idx * w + w) else { return Ok(Vec::new()) };
                let block = u32::from_le_bytes(rec[0..4].try_into().unwrap()) as usize;
                let off = u32::from_le_bytes(rec[4..8].try_into().unwrap()) as usize;
                let len = if size4 {
                    u32::from_le_bytes(rec[8..12].try_into().unwrap()) as usize
                } else {
                    u16::from_le_bytes(rec[8..10].try_into().unwrap()) as usize
                };
                if len == 0 {
                    return Ok(Vec::new());
                }
                let data = self.block(block)?;
                Ok(data.get(off..off + len).context("verse outside its block")?.to_vec())
            }
        }
    }

    fn block(&self, n: usize) -> Result<std::rc::Rc<Vec<u8>>> {
        if let Some((i, d)) = &*self.cache.borrow()
            && *i == n
        {
            return Ok(d.clone());
        }
        let rec = self.blocks.get(n * 12..n * 12 + 12).context("block index out of range")?;
        let off = u32::from_le_bytes(rec[0..4].try_into().unwrap()) as usize;
        let size = u32::from_le_bytes(rec[4..8].try_into().unwrap()) as usize;
        let ucsize = u32::from_le_bytes(rec[8..12].try_into().unwrap()) as usize;
        let mut comp = self.data.get(off..off + size).context("block out of range")?.to_vec();
        if let Some(k) = &self.key {
            Sapphire::new(k.as_bytes()).decrypt_in_place(&mut comp);
        }
        // The sizes come from the module file: bound them before allocating or inflating, so a
        // hostile module cannot ask for gigabytes.
        ensure!(ucsize <= MAX_BLOCK, "a module block claims {ucsize} bytes; the limit is {MAX_BLOCK}");
        let mut out = Vec::with_capacity(ucsize);
        flate2::read::ZlibDecoder::new(&comp[..])
            .take(ucsize as u64 + 1)
            .read_to_end(&mut out)
            .context("could not decompress a module block (wrong unlock key?)")?;
        ensure!(out.len() <= ucsize, "a module block inflates past its declared size");
        let rc = std::rc::Rc::new(out);
        *self.cache.borrow_mut() = Some((n, rc.clone()));
        Ok(rc)
    }
}

/// ThML markup (older CrossWire modules).
fn thml(b: &mut TranslationBuilder, src: &str) {
    let mut skip = 0usize;
    let mut styles: Vec<(String, Option<Style>)> = Vec::new();
    let mut heading: Option<String> = None;
    for tok in tokens(src) {
        match tok {
            Tok::Text(t) if skip == 0 => match &mut heading {
                Some(h) => h.push_str(&t),
                None => b.text(&t),
            },
            Tok::Text(_) => {}
            Tok::Start { name, attrs } => {
                let n = name.to_ascii_lowercase();
                if skip > 0 || matches!(n.as_str(), "note" | "scripref" | "scripture") {
                    skip += 1;
                    continue;
                }
                let class = attr(attrs, "class").unwrap_or_default().to_ascii_lowercase();
                let color = attr(attrs, "color").unwrap_or_default().to_ascii_lowercase();
                let style = match n.as_str() {
                    "font" if color.contains("red") || color.contains("#ff0000") => Some(Style::WordsOfJesus),
                    "span" if class.contains("jesus") || class.contains("wj") => Some(Style::WordsOfJesus),
                    "span" | "added" if class.contains("added") || n == "added" => Some(Style::Added),
                    "i" | "em" => Some(Style::Italic),
                    "b" | "strong" => Some(Style::Bold),
                    _ => None,
                };
                if n == "p" {
                    b.mark(MarkKind::Paragraph, 0);
                } else if matches!(n.as_str(), "h1" | "h2" | "h3" | "h4") || class.contains("sechead") {
                    heading = Some(String::new());
                }
                if let Some(s) = style {
                    b.push_style(s);
                }
                styles.push((n, style));
            }
            Tok::Empty { name, .. } => {
                if skip == 0 && name.eq_ignore_ascii_case("br") {
                    b.mark(MarkKind::Poetry, 1);
                }
            }
            Tok::End { name } => {
                if skip > 0 {
                    skip -= 1;
                    continue;
                }
                let n = name.to_ascii_lowercase();
                if let Some(i) = styles.iter().rposition(|(s, _)| *s == n)
                    && let (_, Some(s)) = styles.remove(i)
                {
                    b.pop_style(s);
                }
                if let (Some(h), true) = (&heading, matches!(n.as_str(), "h1" | "h2" | "h3" | "h4" | "div")) {
                    let h = h.clone();
                    b.heading(MarkKind::Heading, 1, &h);
                    heading = None;
                }
            }
        }
    }
    b.clear_styles();
}

/// GBF (General Bible Format) markup: `<FR>…<Fr>`, `<RF>note<Rf>`, `<CM>`, `<TS>title<Ts>`, …
fn gbf(b: &mut TranslationBuilder, src: &str) {
    let mut rest = src;
    let mut skip = false;
    let mut title: Option<String> = None;
    while !rest.is_empty() {
        let (text, tag, next) = match rest.find('<') {
            Some(i) => match rest[i..].find('>') {
                Some(j) => (&rest[..i], Some(&rest[i + 1..i + j]), &rest[i + j + 1..]),
                None => (rest, None, ""),
            },
            None => (rest, None, ""),
        };
        if !skip {
            match &mut title {
                Some(t) => t.push_str(text),
                None => b.text(text),
            }
        }
        rest = next;
        let Some(tag) = tag else { continue };
        match tag {
            "RF" | "RX" => skip = true,
            "Rf" | "Rx" => skip = false,
            "FR" => b.push_style(Style::WordsOfJesus),
            "Fr" => b.pop_style(Style::WordsOfJesus),
            "FI" => b.push_style(Style::Italic),
            "Fi" => b.pop_style(Style::Italic),
            "FB" => b.push_style(Style::Bold),
            "Fb" => b.pop_style(Style::Bold),
            "FS" => b.push_style(Style::SmallCaps),
            "Fs" => b.pop_style(Style::SmallCaps),
            "CM" => b.mark(MarkKind::Paragraph, 0),
            "CL" => b.mark(MarkKind::Poetry, 1),
            "TS" => title = Some(String::new()),
            "Ts" => {
                if let Some(t) = title.take() {
                    b.heading(MarkKind::Heading, 1, &t);
                }
            }
            _ => {}
        }
    }
    b.clear_styles();
}

/// Sapphire II stream cipher (Michael Paul Johnson), as used by SWORD for locked modules.
pub struct Sapphire {
    cards: [u8; 256],
    rotor: u8,
    ratchet: u8,
    avalanche: u8,
    last_plain: u8,
    last_cipher: u8,
}

impl Sapphire {
    pub fn new(key: &[u8]) -> Self {
        let mut s =
            Sapphire { cards: [0; 256], rotor: 0, ratchet: 0, avalanche: 0, last_plain: 0, last_cipher: 0 };
        for (i, c) in s.cards.iter_mut().enumerate() {
            *c = i as u8;
        }
        if key.is_empty() {
            // SWORD never calls this without a key; match hash_init for completeness.
            s.rotor = 1;
            s.ratchet = 3;
            s.avalanche = 5;
            s.last_plain = 7;
            s.last_cipher = 11;
            for i in 0..256 {
                s.cards[i] = (255 - i) as u8;
            }
            return s;
        }
        let mut rsum: u8 = 0;
        let mut keypos: usize = 0;
        for i in (0..=255usize).rev() {
            let toswap = s.keyrand(i as u32, key, &mut rsum, &mut keypos) as usize;
            s.cards.swap(i, toswap);
        }
        s.rotor = s.cards[1];
        s.ratchet = s.cards[3];
        s.avalanche = s.cards[5];
        s.last_plain = s.cards[7];
        s.last_cipher = s.cards[rsum as usize];
        s
    }

    fn keyrand(&self, limit: u32, key: &[u8], rsum: &mut u8, keypos: &mut usize) -> u8 {
        if limit == 0 {
            return 0;
        }
        let mut retry = 0u32;
        let mut mask: u32 = 1;
        while mask < limit {
            mask = (mask << 1) + 1;
        }
        loop {
            *rsum = self.cards[*rsum as usize].wrapping_add(key[*keypos]);
            *keypos += 1;
            if *keypos >= key.len() {
                *keypos = 0;
                *rsum = rsum.wrapping_add(key.len() as u8);
            }
            let mut u = mask & *rsum as u32;
            retry += 1;
            if retry > 11 {
                u %= limit;
            }
            if u <= limit {
                return u as u8;
            }
        }
    }

    fn step(&mut self) -> u8 {
        self.ratchet = self.ratchet.wrapping_add(self.cards[self.rotor as usize]);
        self.rotor = self.rotor.wrapping_add(1);
        let swaptemp = self.cards[self.last_cipher as usize];
        self.cards[self.last_cipher as usize] = self.cards[self.ratchet as usize];
        self.cards[self.ratchet as usize] = self.cards[self.last_plain as usize];
        self.cards[self.last_plain as usize] = self.cards[self.rotor as usize];
        self.cards[self.rotor as usize] = swaptemp;
        self.avalanche = self.avalanche.wrapping_add(self.cards[swaptemp as usize]);
        let c = &self.cards;
        let a = c[(c[self.ratchet as usize].wrapping_add(c[self.rotor as usize])) as usize];
        let i = c[self.last_plain as usize]
            .wrapping_add(c[self.last_cipher as usize])
            .wrapping_add(c[self.avalanche as usize]);
        a ^ c[c[i as usize] as usize]
    }

    pub fn decrypt(&mut self, b: u8) -> u8 {
        let k = self.step();
        self.last_plain = b ^ k;
        self.last_cipher = b;
        self.last_plain
    }

    pub fn encrypt(&mut self, b: u8) -> u8 {
        let k = self.step();
        self.last_cipher = b ^ k;
        self.last_plain = b;
        self.last_cipher
    }

    pub fn decrypt_in_place(&mut self, buf: &mut [u8]) {
        for x in buf {
            *x = self.decrypt(*x);
        }
    }

    pub fn encrypt_in_place(&mut self, buf: &mut [u8]) {
        for x in buf {
            *x = self.encrypt(*x);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::books::VerseRef;
    use crate::model::TranslationMeta;
    use std::io::Write;

    /// Builds a zText (or RawText) module in memory from `(ref, osis fragment)` pairs.
    pub fn build_module(
        verses: &[(VerseRef, &str)],
        raw: bool,
        key: Option<&str>,
        conf_extra: &str,
    ) -> MemFiles {
        let mut files = HashMap::new();
        let dir = "modules/texts/ztext/test";
        for (t, books) in [("ot", 0..39u8), ("nt", 39..66u8)] {
            // Compute the index size for this testament.
            let mut entries: Vec<Vec<u8>> = vec![Vec::new(), Vec::new()];
            for book in books.map(BookId) {
                entries.push(Vec::new());
                for ch in 1..=book.chapters() {
                    entries.push(Vec::new());
                    let c = ChapterIdx::new(book, ch).unwrap();
                    for v in 1..=c.kjv_verses() {
                        let text = verses
                            .iter()
                            .find(|(r, _)| *r == VerseRef::new(book, ch, v))
                            .map(|(_, s)| s.as_bytes().to_vec())
                            .unwrap_or_default();
                        entries.push(text);
                    }
                }
            }
            if raw {
                let mut data = Vec::new();
                let mut index = Vec::new();
                for e in &entries {
                    let mut e = e.clone();
                    if let Some(k) = key {
                        Sapphire::new(k.as_bytes()).encrypt_in_place(&mut e);
                    }
                    index.extend_from_slice(&(data.len() as u32).to_le_bytes());
                    index.extend_from_slice(&(e.len() as u16).to_le_bytes());
                    data.extend_from_slice(&e);
                }
                files.insert(format!("{dir}/{t}.vss"), index);
                files.insert(format!("{dir}/{t}"), data);
            } else {
                // One block per 50 entries.
                let (mut bzs, mut bzv, mut bzz) = (Vec::new(), Vec::new(), Vec::new());
                for (bi, chunk) in entries.chunks(50).enumerate() {
                    let mut plain = Vec::new();
                    for e in chunk {
                        bzv.extend_from_slice(&(bi as u32).to_le_bytes());
                        bzv.extend_from_slice(&(plain.len() as u32).to_le_bytes());
                        bzv.extend_from_slice(&(e.len() as u16).to_le_bytes());
                        plain.extend_from_slice(e);
                    }
                    let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                    enc.write_all(&plain).unwrap();
                    let mut comp = enc.finish().unwrap();
                    if let Some(k) = key {
                        Sapphire::new(k.as_bytes()).encrypt_in_place(&mut comp);
                    }
                    bzs.extend_from_slice(&(bzz.len() as u32).to_le_bytes());
                    bzs.extend_from_slice(&(comp.len() as u32).to_le_bytes());
                    bzs.extend_from_slice(&(plain.len() as u32).to_le_bytes());
                    bzz.extend_from_slice(&comp);
                }
                files.insert(format!("{dir}/{t}.bzs"), bzs);
                files.insert(format!("{dir}/{t}.bzv"), bzv);
                files.insert(format!("{dir}/{t}.bzz"), bzz);
            }
        }
        let conf = format!(
            "[Test]\nDataPath=./{dir}/\nModDrv={}\nSourceType=OSIS\nEncoding=UTF-8\nCompressType=ZIP\nBlockType=BOOK\nDescription=Test Module\nVersification=KJV\n{conf_extra}",
            if raw { "RawText" } else { "zText" }
        );
        files.insert("mods.d/test.conf".into(), conf.into_bytes());
        MemFiles(files)
    }

    fn fixture() -> Vec<(VerseRef, &'static str)> {
        vec![
            (VerseRef::new(BookId::GENESIS, 1, 1), "In the beginning God created the heaven and the earth."),
            (
                VerseRef::new(BookId::PSALMS, 23, 1),
                "<title type=\"psalm\" canonical=\"true\">A Psalm of David.</title><l level=\"1\">The <divineName>Lord</divineName> is my shepherd; I shall not want.</l>",
            ),
            (
                VerseRef::new(BookId(42), 3, 16),
                "For God so loved the world, <note>n</note><q who=\"Jesus\" marker=\"\">that he gave</q>",
            ),
            (
                VerseRef::new(BookId::REVELATION, 22, 21),
                "The grace of our Lord Jesus Christ be with you all. Amen.",
            ),
        ]
    }

    fn import(files: &MemFiles, key: Option<&str>) -> Result<crate::model::Translation> {
        let conf = SwordConf::parse(std::str::from_utf8(&files.read("mods.d/test.conf")?)?)?;
        let mut b = TranslationBuilder::new(TranslationMeta::default());
        read_module(&mut b, &conf, files, key)?;
        Ok(b.finish())
    }

    #[test]
    fn reads_ztext_and_rawtext() {
        for raw in [false, true] {
            let files = build_module(&fixture(), raw, None, "");
            let t = import(&files, None).unwrap();
            assert_eq!(t.verse_count(), 4);
            assert_eq!(t.find(VerseRef::new(BookId::GENESIS, 1, 1)).unwrap().text, fixture()[0].1);
            let ps = t.find(VerseRef::new(BookId::PSALMS, 23, 1)).unwrap();
            assert_eq!(ps.text, "The Lord is my shepherd; I shall not want.");
            assert_eq!(ps.marks[0].kind, MarkKind::PsalmTitle);
            let jn = t.find(VerseRef::new(BookId(42), 3, 16)).unwrap();
            assert_eq!(jn.text, "For God so loved the world, that he gave");
            assert_eq!(jn.spans[0].style, Style::WordsOfJesus);
            assert!(t.find(VerseRef::new(BookId::REVELATION, 22, 21)).is_some());
        }
    }

    #[test]
    fn locked_modules_need_the_right_key() {
        let files = build_module(&fixture(), false, Some("s3cret-Key"), "CipherKey=\n");
        assert!(import(&files, None).unwrap_err().to_string().contains("locked"));
        assert!(import(&files, Some("wrong")).is_err());
        let t = import(&files, Some("s3cret-Key")).unwrap();
        assert_eq!(t.verse_count(), 4);
        let files = build_module(&fixture(), true, Some("k"), "CipherKey=k\n");
        assert_eq!(import(&files, None).unwrap().verse_count(), 4);
    }

    #[test]
    fn sapphire_matches_sword_reference_vector() {
        // Produced by SWORD's own src/modules/common/sapphire.cpp (encrypt, key "s3cret-Key").
        let expected = "c89e99c63528af3a109d5e0038245651262c36b68c73016046b3a9b2d9957119034efb5579c5bf9421534acb6d4214503b5113588c89";
        let mut buf = b"In the beginning God created the heaven and the earth.".to_vec();
        Sapphire::new(b"s3cret-Key").encrypt_in_place(&mut buf);
        let hex: String = buf.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, expected);
    }

    #[test]
    fn sapphire_round_trip() {
        let msg = b"The quick brown fox jumps over the lazy dog".to_vec();
        let mut enc = msg.clone();
        Sapphire::new(b"key").encrypt_in_place(&mut enc);
        assert_ne!(enc, msg);
        let mut dec = enc.clone();
        Sapphire::new(b"key").decrypt_in_place(&mut dec);
        assert_eq!(dec, msg);
    }

    #[test]
    fn nrsv_versification() {
        let total: u32 = ChapterIdx::all().map(|c| versification("NRSV").unwrap()(c) as u32).sum();
        assert_eq!(total, 31_104);
        assert_eq!(versification("nrsva").unwrap()(ChapterIdx::new(BookId(63), 1).unwrap()), 15);
        assert_eq!(versification("NRSV").unwrap()(ChapterIdx::new(BookId::REVELATION, 12).unwrap()), 18);
        assert!(versification("Vulg").is_none());
    }

    #[test]
    fn conf_parsing() {
        let c = SwordConf::parse("[KJV]\r\nDataPath=./modules/texts/ztext/kjv/\nAbout=Line one\\\nline two\nCipherKey=\nDescription=King James\n").unwrap();
        assert_eq!(c.module, "KJV");
        assert_eq!(c.get("datapath"), Some("./modules/texts/ztext/kjv/"));
        assert_eq!(c.get("About"), Some("Line one\nline two"));
        assert!(c.is_locked());
        assert_eq!(c.description(), "King James");
    }

    #[test]
    fn gbf_and_thml() {
        let mut b = TranslationBuilder::new(TranslationMeta::default());
        b.begin_book(BookId(42));
        b.begin_chapter(3);
        b.begin_verse(16, 16);
        gbf(&mut b, "For God<WG2316> so loved<RF>note<Rf> the world, <FR>that he gave<Fr>");
        b.begin_verse(17, 17);
        thml(&mut b, "For God <scripRef>x</scripRef>sent <font color=\"red\">not</font> his<br/>Son");
        let t = b.finish();
        let v = t.find(VerseRef::new(BookId(42), 3, 16)).unwrap();
        assert_eq!(v.text, "For God so loved the world, that he gave");
        assert_eq!(v.spans[0].style, Style::WordsOfJesus);
        let v = t.find(VerseRef::new(BookId(42), 3, 17)).unwrap();
        assert_eq!(v.plain_text(), "For God sent not his Son");
        assert_eq!(v.spans[0].style, Style::WordsOfJesus);
    }

    #[test]
    fn data_paths_stay_inside_the_module() {
        for ok in ["./modules/texts/ztext/kjv", "modules/texts/rawtext/web", "kjv"] {
            assert!(is_confined(ok), "{ok}");
        }
        for bad in ["../../etc", "./a/../../b", "/etc", "C:/x", "a\\..\\b"] {
            assert!(!is_confined(bad), "{bad}");
        }
        let dir = std::env::temp_dir().join(format!("bible-rs-sword-confine-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(DirFiles(dir.clone()).read("../outside").is_err());
        assert!(DirFiles(dir.clone()).read("/etc/hosts").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hostile_block_sizes_are_refused() {
        let reader = |ucsize: u32, payload: &[u8]| {
            let mut blocks = Vec::new();
            blocks.extend_from_slice(&0u32.to_le_bytes());
            blocks.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            blocks.extend_from_slice(&ucsize.to_le_bytes());
            TestamentReader {
                driver: Driver::ZText { size4: false },
                index: Vec::new(),
                data: payload.to_vec(),
                blocks,
                key: None,
                cache: Default::default(),
            }
        };
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(&vec![b'a'; 1 << 20]).unwrap();
        let bomb = z.finish().unwrap();
        // A declared size of 4 GiB is refused before anything is allocated.
        assert!(reader(u32::MAX, &bomb).block(0).is_err());
        // A block that inflates past what it declares is refused.
        assert!(reader(16, &bomb).block(0).is_err());
        // An honest block still reads.
        assert_eq!(reader(1 << 20, &bomb).block(0).unwrap().len(), 1 << 20);
    }
}
