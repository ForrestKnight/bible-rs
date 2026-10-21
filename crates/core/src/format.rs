//! `.bbl` — the compact on-disk/in-binary translation format.
//!
//! ```text
//! "BIBL" u16 version  u32 meta_len  meta_json        (uncompressed: listing needs no decompression)
//! u32 body_len  zstd(body)
//!
//! body = u32 text_len text  u32 extra_len extra
//!        u16 × 1189 verses-per-chapter
//!        u32 n  then column arrays: number u8×n, number_end u8×n, text_len u16×n, marks_len u16×n, spans_len u16×n
//!        u32 m  marks: offset u16×m, kind u8×m, level u8×m, payload_start u32×m, payload_len u32×m
//!        u32 k  spans: start u16×k, end u16×k, style u8×k
//! ```
//!
//! Columns compress far better than interleaved records, and decoding is a few linear passes.

use std::io::Read;
use std::sync::OnceLock;

use anyhow::{Context, Result, bail, ensure};

use crate::books::CHAPTER_COUNT;
use crate::model::{Mark, MarkKind, Span, Style, Translation, TranslationMeta, Verse};

const MAGIC: &[u8; 4] = b"BIBL";
const VERSION: u16 = 1;

/// Serializes a translation. `level` is the zstd level (19 for bundled data, ~9 for imports); the
/// browser's encoder has one level, about zstd's 1.
pub fn encode(t: &Translation, level: i32) -> Result<Vec<u8>> {
    let mut body = Vec::with_capacity(t.text.len() + t.text.len() / 4);
    // Re-pack text, marks and spans in verse order so that starts are implicit.
    let mut text = String::with_capacity(t.text.len());
    let mut marks: Vec<Mark> = Vec::with_capacity(t.marks.len());
    let mut spans: Vec<Span> = Vec::with_capacity(t.spans.len());
    for v in &t.verses {
        let ts = v.text_start as usize;
        text.push_str(&t.text[ts..ts + v.text_len as usize]);
        let ms = v.marks_start as usize;
        marks.extend_from_slice(&t.marks[ms..ms + v.marks_len as usize]);
        let ss = v.spans_start as usize;
        spans.extend_from_slice(&t.spans[ss..ss + v.spans_len as usize]);
    }
    put_bytes(&mut body, text.as_bytes());
    put_bytes(&mut body, t.extra.as_bytes());
    for c in 0..CHAPTER_COUNT {
        let n = t.chapter_start[c + 1] - t.chapter_start[c];
        ensure!(n <= u16::MAX as u32, "chapter with too many verses");
        body.extend_from_slice(&(n as u16).to_le_bytes());
    }
    body.extend_from_slice(&(t.verses.len() as u32).to_le_bytes());
    body.extend(t.verses.iter().map(|v| v.number));
    body.extend(t.verses.iter().map(|v| v.number_end));
    for v in &t.verses {
        body.extend_from_slice(&v.text_len.to_le_bytes());
    }
    for v in &t.verses {
        body.extend_from_slice(&v.marks_len.to_le_bytes());
    }
    for v in &t.verses {
        body.extend_from_slice(&v.spans_len.to_le_bytes());
    }
    body.extend_from_slice(&(marks.len() as u32).to_le_bytes());
    for m in &marks {
        body.extend_from_slice(&m.offset.to_le_bytes());
    }
    body.extend(marks.iter().map(|m| m.kind as u8));
    body.extend(marks.iter().map(|m| m.level));
    for m in &marks {
        body.extend_from_slice(&m.payload.0.to_le_bytes());
    }
    for m in &marks {
        body.extend_from_slice(&m.payload.1.to_le_bytes());
    }
    body.extend_from_slice(&(spans.len() as u32).to_le_bytes());
    for s in &spans {
        body.extend_from_slice(&s.start.to_le_bytes());
    }
    for s in &spans {
        body.extend_from_slice(&s.end.to_le_bytes());
    }
    body.extend(spans.iter().map(|s| s.style as u8));

    #[cfg(not(target_arch = "wasm32"))]
    let compressed = zstd::bulk::compress(&body, level).context("zstd compression")?;
    #[cfg(target_arch = "wasm32")]
    let compressed = {
        let _ = level;
        ruzstd::encoding::compress_to_vec(&body[..], ruzstd::encoding::CompressionLevel::Fastest)
    };
    let meta = serde_json::to_vec(&t.meta)?;
    let mut out = Vec::with_capacity(compressed.len() + meta.len() + 16);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    put_bytes(&mut out, &meta);
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&compressed);
    Ok(out)
}

/// Reads only the metadata header.
pub fn read_meta(bytes: &[u8]) -> Result<TranslationMeta> {
    let mut r = Reader::new(bytes);
    header(&mut r)
}

fn header(r: &mut Reader<'_>) -> Result<TranslationMeta> {
    ensure!(r.take(4)? == MAGIC, "not a bible-rs translation file");
    let version = r.u16()?;
    ensure!(version == VERSION, "unsupported translation file version {version}");
    let meta = r.bytes()?;
    serde_json::from_slice(meta).context("translation metadata")
}

/// The size of a translation once decoded (its body before compression), from the header alone.
pub fn decoded_len(bytes: &[u8]) -> Option<u64> {
    let mut r = Reader::new(bytes);
    header(&mut r).ok()?;
    r.u32().ok().map(u64::from)
}

pub fn decode(bytes: &[u8]) -> Result<Translation> {
    let mut r = Reader::new(bytes);
    let meta = header(&mut r)?;
    let body_len = r.u32()? as usize;
    ensure!(body_len < 256 << 20, "implausible body size");
    let body = decompress(r.rest(), body_len)?;
    let mut r = Reader::new(&body);

    let text = String::from_utf8(r.bytes()?.to_vec()).context("text is not UTF-8")?;
    let extra = String::from_utf8(r.bytes()?.to_vec()).context("extra is not UTF-8")?;
    let mut chapter_start = Vec::with_capacity(CHAPTER_COUNT + 1);
    let mut acc = 0u32;
    chapter_start.push(0);
    for _ in 0..CHAPTER_COUNT {
        acc += r.u16()? as u32;
        chapter_start.push(acc);
    }
    let n = r.u32()? as usize;
    ensure!(n as u32 == acc, "verse count mismatch");
    let numbers = r.take(n)?;
    let ends = r.take(n)?;
    let text_lens = r.take(n * 2)?;
    let mark_lens = r.take(n * 2)?;
    let span_lens = r.take(n * 2)?;
    let mut verses = Vec::with_capacity(n);
    let (mut ts, mut ms, mut ss) = (0u32, 0u32, 0u32);
    for i in 0..n {
        let tl = le16(text_lens, i);
        let ml = le16(mark_lens, i);
        let sl = le16(span_lens, i);
        verses.push(Verse {
            number: numbers[i],
            number_end: ends[i],
            text_start: ts,
            text_len: tl,
            marks_start: ms,
            marks_len: ml,
            spans_start: ss,
            spans_len: sl,
        });
        ts += tl as u32;
        ms += ml as u32;
        ss += sl as u32;
    }
    ensure!(ts as usize == text.len(), "text length mismatch");
    // Verse boundaries must fall on char boundaries, or slicing would panic later.
    ensure!(verses.iter().all(|v| text.is_char_boundary(v.text_start as usize)), "corrupt verse offsets");

    let m = r.u32()? as usize;
    ensure!(m as u32 == ms, "mark count mismatch");
    let offs = r.take(m * 2)?;
    let kinds = r.take(m)?;
    let levels = r.take(m)?;
    let pstarts = r.take(m * 4)?;
    let plens = r.take(m * 4)?;
    let mut marks = Vec::with_capacity(m);
    for i in 0..m {
        let kind = MarkKind::from_u8(kinds[i]).context("unknown mark kind")?;
        let payload = (le32(pstarts, i), le32(plens, i));
        ensure!((payload.0 as u64 + payload.1 as u64) <= extra.len() as u64, "mark payload out of range");
        marks.push(Mark { offset: le16(offs, i), kind, level: levels[i], payload });
    }
    let k = r.u32()? as usize;
    ensure!(k as u32 == ss, "span count mismatch");
    let starts = r.take(k * 2)?;
    let endsb = r.take(k * 2)?;
    let styles = r.take(k)?;
    let mut spans = Vec::with_capacity(k);
    for (i, &st) in styles.iter().enumerate() {
        let style = Style::from_u8(st).context("unknown style")?;
        spans.push(Span { start: le16(starts, i), end: le16(endsb, i), style });
    }
    for v in &verses {
        let len = v.text_len;
        let ms = &marks[v.marks_start as usize..(v.marks_start + v.marks_len as u32) as usize];
        let ss = &spans[v.spans_start as usize..(v.spans_start + v.spans_len as u32) as usize];
        let base = v.text_start as usize;
        let ok = ms.iter().all(|m| m.offset <= len && text.is_char_boundary(base + m.offset as usize))
            && ss.iter().all(|s| {
                s.start <= s.end
                    && s.end <= len
                    && text.is_char_boundary(base + s.start as usize)
                    && text.is_char_boundary(base + s.end as usize)
            });
        if !ok {
            bail!("corrupt mark/span offsets");
        }
    }
    Ok(Translation { meta, text, extra, verses, chapter_start, marks, spans, search: OnceLock::new() })
}

fn decompress(data: &[u8], len: usize) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(len);
    #[cfg(not(target_arch = "wasm32"))]
    zstd::stream::read::Decoder::with_buffer(data)?.take(len as u64 + 1).read_to_end(&mut out)?;
    #[cfg(target_arch = "wasm32")]
    ruzstd::decoding::StreamingDecoder::new(data)
        .map_err(|e| anyhow::anyhow!("zstd: {e}"))?
        .take(len as u64 + 1)
        .read_to_end(&mut out)?;
    ensure!(out.len() == len, "decompressed size mismatch");
    Ok(out)
}

fn put_bytes(out: &mut Vec<u8>, b: &[u8]) {
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.extend_from_slice(b);
}

fn le16(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[2 * i], b[2 * i + 1]])
}

fn le32(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]])
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end =
            self.pos.checked_add(n).filter(|&e| e <= self.b.len()).context("truncated translation file")?;
        let s = &self.b[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn bytes(&mut self) -> Result<&'a [u8]> {
        let n = self.u32()? as usize;
        self.take(n)
    }

    fn rest(&self) -> &'a [u8] {
        &self.b[self.pos..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::{BookId, ChapterIdx};
    use crate::model::TranslationBuilder;

    #[test]
    fn round_trip_preserves_everything() {
        let mut b = TranslationBuilder::new(TranslationMeta { id: "x".into(), ..Default::default() });
        b.begin_book(BookId(42));
        b.set_book_title(BookId(42), Some("John"), Some("The Gospel of John"));
        b.begin_chapter(1);
        b.heading(MarkKind::Heading, 1, "The Word");
        b.mark(MarkKind::Paragraph, 0);
        b.begin_verse(1, 1);
        b.text("In the beginning was the Word.");
        b.begin_verse(2, 2);
        b.push_style(Style::WordsOfJesus);
        b.text("“Follow me.”");
        b.pop_style(Style::WordsOfJesus);
        b.mark(MarkKind::Poetry, 2);
        b.text("Ünïcode ✓");
        let t = b.finish();
        let bytes = encode(&t, 3).unwrap();
        assert_eq!(read_meta(&bytes).unwrap().books[42].long, "The Gospel of John");
        let back = decode(&bytes).unwrap();
        let c = ChapterIdx::new(BookId(42), 1).unwrap();
        let a: Vec<_> = t
            .verses_in(c)
            .map(|v| (v.number, v.text.to_string(), v.marks.to_vec(), v.spans.to_vec()))
            .collect();
        let b: Vec<_> = back
            .verses_in(c)
            .map(|v| (v.number, v.text.to_string(), v.marks.to_vec(), v.spans.to_vec()))
            .collect();
        assert_eq!(a, b);
        let first = back.verses_in(c).next().unwrap();
        assert_eq!(back.mark_text(&first.marks[0]), "The Word");
        assert_eq!(back.meta, t.meta);
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode(b"nope").is_err());
        assert!(decode(b"BIBL\x01\x00\x02\x00\x00\x00{}\x00").is_err());
    }
}
