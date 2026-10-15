//! Chapter typesetting: turns a chapter of the verse model into positioned text galleys.
//!
//! Layout depends only on the text, the typography settings and the column width — never on
//! highlights, selection or search hits. Those are painted underneath the text from glyph
//! geometry (see [`Block::char_rects`]), so clicking a verse or highlighting it never forces a
//! relayout.

use std::ops::Range;
use std::sync::Arc;

use bible_core::model::{MarkKind, Style, Translation, VerseView};
use bible_core::{BookId, ChapterIdx};
use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{Align, Color32, FontId, Galley, Rect, Vec2, pos2, vec2};
use eframe::epaint::text::FontsView;
use serde::{Deserialize, Serialize};

use crate::theme::{self, Palette, SERIF, SERIF_DISPLAY, SERIF_ITALIC, SERIF_SEMIBOLD, Theme, UI, UI_MEDIUM};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Mode {
    /// Verses flow inside paragraphs, numbers as superscripts.
    #[default]
    Paragraph,
    /// Each verse starts on its own line with its number in the margin.
    VersePerLine,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Typography {
    pub size: f32,
    pub line_height: f32,
    pub mode: Mode,
    pub red_letters: bool,
    pub justify: bool,
    pub verse_numbers: bool,
    pub headings: bool,
    /// Maximum text measure in ems.
    pub measure: f32,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            size: 19.0,
            line_height: 1.62,
            mode: Mode::Paragraph,
            red_letters: true,
            justify: false,
            verse_numbers: true,
            headings: true,
            measure: 34.0,
        }
    }
}

/// A run of text placed in a chapter (relative to the chapter's top-left corner).
pub struct Block {
    pub pos: Vec2,
    pub galley: Arc<Galley>,
    /// `(first char, global verse index, verse number)` — chars from `first char` on belong to it.
    pub verses: Vec<(usize, u32, u8)>,
    /// A label painted in the left gutter (verse-per-line numbers).
    pub gutter: Option<(Arc<Galley>, Vec2)>,
    /// Which column (parallel view) this block belongs to.
    pub column: usize,
}

impl Block {
    pub fn rect(&self) -> Rect {
        self.galley.rect.translate(self.pos)
    }

    /// Verse at a char index of this block.
    pub fn verse_at_char(&self, ch: usize) -> Option<u32> {
        let i = self.verses.partition_point(|(start, _, _)| *start <= ch);
        (i > 0).then(|| self.verses[i - 1].1)
    }

    /// Char range of a verse within this block, if present.
    pub fn verse_chars(&self, verse: u32) -> Option<Range<usize>> {
        let i = self.verses.iter().position(|(_, v, _)| *v == verse)?;
        let start = self.verses[i].0;
        let end = self.verses.get(i + 1).map_or(usize::MAX, |(s, _, _)| *s);
        Some(start..end)
    }

    /// Rectangles (block-relative) covering a char range, one per row.
    pub fn char_rects(&self, range: Range<usize>) -> Vec<Rect> {
        let mut out = Vec::new();
        let mut base = 0usize;
        for row in &self.galley.rows {
            let n = row.glyphs.len();
            let row_range = base..base + n;
            let s = range.start.max(row_range.start);
            let e = range.end.min(row_range.end);
            if s < e {
                let g0 = &row.glyphs[s - base];
                let g1 = &row.glyphs[e - base - 1];
                let x0 = row.pos.x + g0.pos.x;
                let x1 = row.pos.x + g1.pos.x + g1.advance_width;
                out.push(Rect::from_min_max(pos2(x0, row.pos.y), pos2(x1, row.pos.y + row.height())));
            }
            base += n + usize::from(row.ends_with_newline);
        }
        out
    }
}

pub struct VerseAnchor {
    pub index: u32,
    pub number: u8,
    pub column: usize,
    /// Top of the verse's first line, relative to the chapter.
    pub top: f32,
    pub bottom: f32,
}

pub struct ChapterLayout {
    pub chapter: ChapterIdx,
    pub height: f32,
    pub blocks: Vec<Block>,
    /// One entry per verse of the first column, in order.
    pub anchors: Vec<VerseAnchor>,
    /// Non-interactive labels (book titles, chapter headings, messages).
    pub labels: Vec<(Vec2, Arc<Galley>)>,
}

impl ChapterLayout {
    pub fn anchor(&self, column: usize, number: u8) -> Option<&VerseAnchor> {
        self.anchors
            .iter()
            .filter(|a| a.column == column)
            .find(|a| a.number >= number)
            .or_else(|| self.anchors.last())
    }

    /// The verse (first column) whose top is at or above `y`.
    pub fn verse_at_y(&self, y: f32) -> Option<&VerseAnchor> {
        let col0: Vec<&VerseAnchor> = self.anchors.iter().filter(|a| a.column == 0).collect();
        let i = col0.partition_point(|a| a.top <= y);
        col0.get(i.saturating_sub(1)).copied()
    }
}

/// Content of one parallel column for a chapter.
pub enum ColumnText<'a> {
    Ready(&'a Translation),
    Message(String),
}

#[derive(Clone)]
pub struct Ctx<'a> {
    pub typo: &'a Typography,
    pub theme: Theme,
    /// Width of a single text column.
    pub width: f32,
    pub gutter: f32,
    /// Line height for the smaller capitals of LORD, chosen so their baseline matches the text.
    /// (egui puts each section's baseline at its own ascent when line heights are explicit.)
    pub small_caps_lh: f32,
}

fn ascent(fonts: &mut FontsView<'_>, font: FontId) -> f32 {
    let g = fonts.layout_job(LayoutJob::single_section(
        "H".into(),
        TextFormat { font_id: font, ..Default::default() },
    ));
    g.rows.first().and_then(|r| r.glyphs.first()).map_or(0.0, |gl| gl.pos.y)
}

impl<'a> Ctx<'a> {
    pub fn new(typo: &'a Typography, theme: Theme, width: f32, gutter: f32) -> Self {
        Self { typo, theme, width, gutter, small_caps_lh: 0.0 }
    }

    /// Fills in font-metric-dependent values.
    fn measured(&self, fonts: &mut FontsView<'_>) -> Self {
        let big = ascent(fonts, FontId::new(self.typo.size, theme::family(SERIF)));
        let small = ascent(fonts, FontId::new((self.typo.size * 0.78).round(), theme::family(SERIF)));
        Self { small_caps_lh: self.lh() - (big - small), ..self.clone() }
    }
}

impl Ctx<'_> {
    fn p(&self) -> &'static Palette {
        self.theme.palette()
    }

    fn lh(&self) -> f32 {
        (self.typo.size * self.typo.line_height).round()
    }

    fn fmt(&self, family: &str, size: f32, color: Color32) -> TextFormat {
        TextFormat {
            font_id: FontId::new(size, theme::family(family)),
            color,
            line_height: Some(self.lh()),
            valign: Align::BOTTOM,
            ..Default::default()
        }
    }

    fn body(&self, styles: u8) -> TextFormat {
        let p = self.p();
        let family = if styles & Style::Bold.bit() != 0 {
            SERIF_SEMIBOLD
        } else if styles & (Style::Added.bit() | Style::Italic.bit() | Style::Selah.bit()) != 0 {
            SERIF_ITALIC
        } else {
            SERIF
        };
        let color = if self.typo.red_letters && styles & Style::WordsOfJesus.bit() != 0 {
            p.red_letter
        } else {
            p.text
        };
        self.fmt(family, self.typo.size, color)
    }

    fn verse_number_fmt(&self) -> TextFormat {
        TextFormat {
            font_id: FontId::new((self.typo.size * 0.56).round(), theme::family(UI)),
            color: self.p().verse_num,
            line_height: Some(self.lh()),
            valign: Align::TOP,
            ..Default::default()
        }
    }
}

/// `job.append`, but extends the previous section when the format is identical: every section
/// is shaped as its own run, so fewer sections means less shaping work.
fn append(job: &mut LayoutJob, text: &str, format: TextFormat) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = job.sections.last_mut()
        && last.format == format
        && last.byte_range.end.0 == job.text.len()
    {
        job.text.push_str(text);
        last.byte_range.end.0 = job.text.len();
        return;
    }
    job.append(text, 0.0, format);
}

/// Appends verse text with styles applied (faux small caps for the divine name).
fn push_styled(job: &mut LayoutJob, cx: &Ctx<'_>, text: &str, styles_at: &dyn Fn(usize) -> u8, base: usize) {
    push_styled_lead(job, cx, text, styles_at, base, 0.0);
}

/// Like [`push_styled`], with `lead` points of leading space before the first character.
fn push_styled_lead(
    job: &mut LayoutJob,
    cx: &Ctx<'_>,
    text: &str,
    styles_at: &dyn Fn(usize) -> u8,
    base: usize,
    lead: f32,
) {
    let sections_before = job.sections.len();
    // Split into runs of constant style.
    let mut run_start = 0usize;
    let mut run_style = None;
    let bytes: Vec<(usize, char)> = text.char_indices().collect();
    let flush = |job: &mut LayoutJob, s: usize, e: usize, st: u8| {
        if s >= e {
            return;
        }
        let chunk = &text[s..e];
        if st & (Style::DivineName.bit() | Style::SmallCaps.bit()) != 0 {
            small_caps(job, cx, chunk, st);
        } else {
            append(job, chunk, cx.body(st));
        }
    };
    for &(i, _) in &bytes {
        let st = styles_at(base + i);
        match run_style {
            None => run_style = Some(st),
            Some(prev) if prev != st => {
                flush(job, run_start, i, prev);
                run_start = i;
                run_style = Some(st);
            }
            _ => {}
        }
    }
    if let Some(st) = run_style {
        flush(job, run_start, text.len(), st);
    }
    if lead > 0.0
        && let Some(sec) = job.sections.get_mut(sections_before)
    {
        sec.leading_space = lead;
    }
}

/// "LORD"/"Lord" → L + ᴏʀᴅ: the first letter of each word at full size, the rest upper-cased smaller.
fn small_caps(job: &mut LayoutJob, cx: &Ctx<'_>, text: &str, styles: u8) {
    let full = cx.body(styles & !(Style::DivineName.bit() | Style::SmallCaps.bit()));
    let mut small = full.clone();
    small.font_id.size = (cx.typo.size * 0.78).round();
    small.extra_letter_spacing = 0.4;
    if cx.small_caps_lh > 0.0 {
        small.line_height = Some(cx.small_caps_lh);
    }
    let mut word_start = true;
    let mut buf = String::new();
    let mut buf_small = false;
    let emit = |job: &mut LayoutJob, buf: &mut String, is_small: bool| {
        if !buf.is_empty() {
            append(job, buf, if is_small { small.clone() } else { full.clone() });
            buf.clear();
        }
    };
    for c in text.chars() {
        let is_small = c.is_alphabetic() && !word_start;
        if is_small != buf_small {
            emit(job, &mut buf, buf_small);
            buf_small = is_small;
        }
        let mut up = c.to_uppercase();
        if is_small && up.len() == 1 {
            buf.push(up.next().unwrap());
        } else {
            buf.push(c);
        }
        word_start = !c.is_alphabetic();
    }
    emit(job, &mut buf, buf_small);
}

fn style_mask<'a>(v: &'a VerseView<'a>) -> impl Fn(usize) -> u8 + 'a {
    move |off: usize| {
        v.spans
            .iter()
            .filter(|s| (s.start as usize) <= off && off < s.end as usize)
            .fold(0u8, |m, s| m | s.style.bit())
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Prose { indent: bool },
    Poetry(u8),
    Right,
    Center,
}

/// Accumulates the current text block.
struct Pending {
    job: LayoutJob,
    chars: usize,
    verses: Vec<(usize, u32, u8)>,
    kind: Kind,
    gutter: Option<u8>,
}

struct Builder<'c, 'f> {
    cx: &'c Ctx<'c>,
    fonts: &'c mut FontsView<'f>,
    blocks: Vec<Block>,
    labels: Vec<(Vec2, Arc<Galley>)>,
    y: f32,
    x0: f32,
    column: usize,
    pending: Option<Pending>,
    /// Extra space requested before the next block.
    space: f32,
    after_heading: bool,
}

impl Builder<'_, '_> {
    fn em(&self) -> f32 {
        self.cx.typo.size
    }

    fn start(&mut self, kind: Kind) {
        self.flush();
        let job = LayoutJob {
            wrap: TextWrapping { max_width: self.cx.width, ..Default::default() },
            justify: self.cx.typo.justify && matches!(kind, Kind::Prose { .. }),
            halign: match kind {
                Kind::Right => Align::RIGHT,
                Kind::Center => Align::Center,
                _ => Align::LEFT,
            },
            ..Default::default()
        };
        self.pending = Some(Pending { job, chars: 0, verses: Vec::new(), kind, gutter: None });
    }

    fn ensure(&mut self, kind: Kind) -> &mut Pending {
        if self.pending.is_none() {
            self.start(kind);
        }
        self.pending.as_mut().unwrap()
    }

    fn flush(&mut self) {
        let Some(mut p) = self.pending.take() else { return };
        if p.job.text.is_empty() {
            return;
        }
        let em = self.em();
        let (indent, hang) = match p.kind {
            Kind::Poetry(level) => (em * 1.4 * (level.saturating_sub(1)) as f32, em * 1.4),
            _ => (0.0, 0.0),
        };
        if let Kind::Prose { indent: true } = p.kind
            && let Some(s) = p.job.sections.first_mut()
        {
            s.leading_space = em * 1.5;
        }
        p.job.wrap.max_width = (self.cx.width - indent - hang).max(em * 6.0);
        let gap = match p.kind {
            Kind::Prose { .. } => em * 0.32,
            _ => 0.0,
        };
        if !self.blocks.is_empty() || !self.labels.is_empty() {
            self.y += gap.max(self.space);
        } else {
            self.y += self.space;
        }
        self.space = 0.0;
        let mut galley = self.fonts.layout_job(p.job);
        if hang > 0.0 && galley.rows.len() > 1 {
            galley = hanging(galley, hang);
        }
        let x = match p.kind {
            Kind::Right | Kind::Center => {
                self.x0 + if p.kind == Kind::Right { self.cx.width } else { self.cx.width / 2.0 }
            }
            _ => self.x0 + indent,
        };
        let pos = vec2(x, self.y);
        let gutter = p.gutter.map(|n| {
            let fmt = TextFormat {
                font_id: FontId::new((em * 0.62).round(), theme::family(UI_MEDIUM)),
                color: self.cx.p().verse_num,
                ..Default::default()
            };
            let g = self.fonts.layout_job(LayoutJob::single_section(n.to_string(), fmt));
            let first_row = galley.rows.first().map_or(0.0, |r| r.height());
            // Right-aligned in the gutter, vertically centred on the first line's x-height.
            let gp = vec2(x - g.size().x - em * 0.55, self.y + (first_row - g.size().y) * 0.62);
            (g, gp)
        });
        self.y += galley.size().y;
        self.blocks.push(Block { pos, galley, verses: p.verses, gutter, column: self.column });
        self.after_heading = false;
    }

    fn label(&mut self, job: LayoutJob, space_before: f32, space_after: f32, center: bool) {
        self.flush();
        self.y += space_before.max(self.space);
        self.space = 0.0;
        let g = self.fonts.layout_job(job);
        let x = if center { self.x0 + (self.cx.width - g.size().x) / 2.0 } else { self.x0 };
        self.labels.push((vec2(x, self.y), g.clone()));
        self.y += g.size().y + space_after;
    }

    fn heading(&mut self, kind: MarkKind, level: u8, text: &str) {
        if !self.cx.typo.headings && kind == MarkKind::Heading {
            return;
        }
        let em = self.em();
        let p = self.cx.p();
        let mut job = LayoutJob::default();
        job.wrap.max_width = self.cx.width;
        let (fmt, before, after, center) = match (kind, level) {
            (MarkKind::PsalmTitle, _) => {
                (self.cx.fmt(SERIF_ITALIC, (em * 0.88).round(), p.muted), em * 0.2, em * 0.5, false)
            }
            (MarkKind::Speaker, _) => {
                (self.cx.fmt(SERIF_ITALIC, (em * 0.88).round(), p.muted), em * 0.6, em * 0.15, false)
            }
            (MarkKind::Heading, 0) => {
                let mut f = self.cx.fmt(UI_MEDIUM, (em * 0.62).round(), p.accent);
                f.extra_letter_spacing = 1.6;
                (f, em * 1.4, em * 0.6, true)
            }
            (MarkKind::Heading, 3) => {
                let mut f = self.cx.fmt(UI_MEDIUM, (em * 0.62).round(), p.accent);
                f.extra_letter_spacing = 1.2;
                (f, em * 1.0, em * 0.25, false)
            }
            _ => (self.cx.fmt(SERIF_SEMIBOLD, (em * 0.92).round(), p.heading), em * 1.25, em * 0.35, false),
        };
        let text = if matches!((kind, level), (MarkKind::Heading, 0 | 3)) {
            text.to_uppercase()
        } else {
            text.to_string()
        };
        let mut fmt = fmt;
        fmt.line_height = None;
        job.append(&text, 0.0, fmt);
        if center {
            job.halign = Align::Center;
            job.wrap.max_width = self.cx.width;
        }
        self.flush();
        self.y += before.max(self.space);
        self.space = 0.0;
        let g = self.fonts.layout_job(job);
        let x = if center { self.x0 + self.cx.width / 2.0 } else { self.x0 };
        self.labels.push((vec2(x, self.y), g.clone()));
        self.y += g.size().y + after;
        self.after_heading = true;
    }
}

/// Moves wrapped continuation rows right by `hang` (a hanging indent for poetry lines).
fn hanging(galley: Arc<Galley>, hang: f32) -> Arc<Galley> {
    let mut g = (*galley).clone();
    let mut prev_newline = true;
    for row in &mut g.rows {
        if !prev_newline {
            row.pos.x += hang;
        }
        prev_newline = row.ends_with_newline;
    }
    let mut rect = g.rows.first().map_or(Rect::NOTHING, |r| r.rect());
    for r in &g.rows {
        rect = rect.union(r.rect());
    }
    g.rect = rect;
    g.mesh_bounds = g.mesh_bounds.union(rect);
    Arc::new(g)
}

fn chapter_title(c: ChapterIdx) -> String {
    if c.book() == BookId::PSALMS {
        format!("PSALM {}", c.chapter())
    } else {
        format!("CHAPTER {}", c.chapter())
    }
}

/// Book title + chapter heading at the top of a chapter.
fn chapter_head(b: &mut Builder<'_, '_>, t: Option<&Translation>, c: ChapterIdx, single_column: bool) {
    let em = b.em();
    let p = b.cx.p();
    if c.chapter() == 1 {
        let short =
            t.map(|t| t.meta.book_short(c.book()).to_string()).unwrap_or_else(|| c.book().name().to_string());
        let mut job = LayoutJob::default();
        let mut f = b.cx.fmt(SERIF_DISPLAY, (em * if single_column { 2.4 } else { 1.8 }).round(), p.heading);
        f.line_height = None;
        job.append(&short, 0.0, f);
        job.halign = Align::Center;
        b.label(job, em * if single_column { 2.2 } else { 1.2 }, em * 0.25, false);
        if let Some((pos, g)) = b.labels.last_mut() {
            pos.x = b.x0 + b.cx.width / 2.0;
            let _ = g;
        }
        let long = t
            .map(|t| t.meta.books.get(c.book().index()).map(|b| b.long.clone()).unwrap_or_default())
            .unwrap_or_default();
        if !long.is_empty() && long != short && single_column {
            let mut job = LayoutJob::default();
            job.wrap.max_width = b.cx.width;
            let mut f = b.cx.fmt(SERIF_ITALIC, (em * 0.9).round(), p.muted);
            f.line_height = None;
            job.append(&long, 0.0, f);
            job.halign = Align::Center;
            b.label(job, 0.0, em * 0.4, false);
            if let Some((pos, _)) = b.labels.last_mut() {
                pos.x = b.x0 + b.cx.width / 2.0;
            }
        }
        b.y += em * 0.8;
    } else {
        b.y += em * 2.2;
    }
    let mut job = LayoutJob::default();
    let mut f = TextFormat {
        font_id: FontId::new((em * 0.6).round(), theme::family(UI_MEDIUM)),
        color: p.accent,
        extra_letter_spacing: 2.2,
        ..Default::default()
    };
    f.valign = Align::Center;
    job.append(&chapter_title(c), 0.0, f);
    job.halign = Align::Center;
    b.label(job, 0.0, em * 0.9, false);
    if let Some((pos, _)) = b.labels.last_mut() {
        pos.x = b.x0 + b.cx.width / 2.0;
    }
    b.after_heading = true;
}

/// Lays out one chapter of one translation in a single column.
pub fn layout_chapter(
    fonts: &mut FontsView<'_>,
    cx: &Ctx<'_>,
    t: &Translation,
    c: ChapterIdx,
) -> ChapterLayout {
    let cx = &cx.measured(fonts);
    let mut b = Builder {
        cx,
        fonts,
        blocks: Vec::new(),
        labels: Vec::new(),
        y: 0.0,
        x0: 0.0,
        column: 0,
        pending: None,
        space: 0.0,
        after_heading: false,
    };
    chapter_head(&mut b, Some(t), c, true);
    let em = b.em();
    let per_line = cx.typo.mode == Mode::VersePerLine;
    let mut kind = Kind::Prose { indent: false };
    for v in t.verses_in(c) {
        let styles = style_mask(&v);
        // Break points inside the verse: (offset, mark).
        let mut cursor = 0usize;
        let mut first = true;
        let marks: Vec<_> = v.marks.iter().collect();
        let mut mi = 0usize;
        loop {
            // Apply all marks at the current offset.
            let mut broke = false;
            while mi < marks.len() && marks[mi].offset as usize <= cursor {
                let m = marks[mi];
                mi += 1;
                match m.kind {
                    MarkKind::Heading | MarkKind::PsalmTitle | MarkKind::Speaker => {
                        b.heading(m.kind, m.level, t.mark_text(m));
                        kind = Kind::Prose { indent: false };
                    }
                    MarkKind::Blank => {
                        b.flush();
                        b.space = b.space.max(em * 0.75);
                    }
                    MarkKind::Paragraph => {
                        b.flush();
                        kind = Kind::Prose { indent: !b.after_heading && !per_line };
                        b.start(kind);
                        broke = true;
                    }
                    MarkKind::ParagraphFlush => {
                        kind = Kind::Prose { indent: false };
                        b.start(kind);
                        broke = true;
                    }
                    MarkKind::Poetry | MarkKind::List => {
                        if matches!(kind, Kind::Prose { .. }) && !b.blocks.is_empty() {
                            b.space = b.space.max(em * 0.3);
                        }
                        kind = Kind::Poetry(m.level.max(1));
                        b.start(kind);
                        broke = true;
                    }
                    MarkKind::PoetryRight => {
                        b.start(Kind::Right);
                        broke = true;
                    }
                    MarkKind::Centered => {
                        b.start(Kind::Center);
                        broke = true;
                    }
                }
            }
            if first {
                first = false;
                if per_line && !broke {
                    // Continue the current kind (poetry stays indented) but on a new line.
                    let k = match kind {
                        Kind::Prose { .. } => Kind::Prose { indent: false },
                        k => k,
                    };
                    b.start(k);
                }
                let p = b.ensure(kind);
                // Separate this verse from the previous one in the same paragraph.
                if p.chars > 0 && !p.job.text.ends_with([' ', '\u{a0}', '\n']) {
                    append(&mut p.job, " ", cx.body(0));
                    p.chars += 1;
                }
                p.verses.push((p.chars, v.index, v.number));
                if cx.typo.verse_numbers {
                    if per_line && p.chars == 0 {
                        p.gutter = Some(v.number);
                    } else {
                        let label = if v.number_end > v.number {
                            format!("{}–{}", v.number, v.number_end)
                        } else {
                            v.number.to_string()
                        };
                        let n = label.chars().count() + 1;
                        // The no-break space keeps the number with its verse at line ends.
                        p.job.append(&format!("{label}\u{a0}"), 0.0, cx.verse_number_fmt());
                        p.chars += n;
                    }
                }
            } else if broke {
                // A line break inside the verse: the new block still belongs to this verse.
                let p = b.ensure(kind);
                p.verses.push((p.chars, v.index, v.number));
            }
            let next = marks.get(mi).map_or(v.text.len(), |m| (m.offset as usize).min(v.text.len()));
            if next > cursor {
                let seg = &v.text[cursor..next];
                let p = b.ensure(kind);
                if p.verses.last().is_none_or(|(_, vi, _)| *vi != v.index) {
                    p.verses.push((p.chars, v.index, v.number));
                }
                push_styled(&mut p.job, cx, seg, &styles, cursor);
                p.chars += seg.chars().count();
                cursor = next;
            }
            if mi >= marks.len() && cursor >= v.text.len() {
                break;
            }
            if mi < marks.len() && marks[mi].offset as usize > v.text.len() {
                break;
            }
        }
    }
    b.flush();
    let Builder { blocks, labels, y, .. } = b;
    let anchors = anchors_for(&blocks);
    ChapterLayout { chapter: c, height: y + em * 1.2, blocks, anchors, labels }
}

fn anchors_for(blocks: &[Block]) -> Vec<VerseAnchor> {
    let mut anchors: Vec<VerseAnchor> = Vec::new();
    for bl in blocks {
        for (start, v, number) in &bl.verses {
            let top = bl.pos.y
                + bl.galley.pos_from_cursor(eframe::epaint::text::cursor::CCursor::new(*start)).min.y;
            match anchors.iter_mut().find(|a| a.index == *v && a.column == bl.column) {
                Some(a) => a.bottom = a.bottom.max(bl.pos.y + bl.galley.size().y),
                None => anchors.push(VerseAnchor {
                    index: *v,
                    number: *number,
                    column: bl.column,
                    top,
                    bottom: bl.pos.y + bl.galley.size().y,
                }),
            }
        }
    }
    anchors
}

/// Lays out a chapter as verse-aligned rows across several translations.
pub fn layout_parallel(
    fonts: &mut FontsView<'_>,
    cx: &Ctx<'_>,
    cols: &[(String, ColumnText<'_>)],
    c: ChapterIdx,
) -> ChapterLayout {
    let cx = &cx.measured(fonts);
    let em = cx.typo.size;
    let n = cols.len().max(1);
    let col_w = cx.width;
    let mut b = Builder {
        cx,
        fonts,
        blocks: Vec::new(),
        labels: Vec::new(),
        y: 0.0,
        x0: 0.0,
        column: 0,
        pending: None,
        space: 0.0,
        after_heading: false,
    };
    let full = col_w * n as f32 + cx.gutter * (n as f32 - 1.0);
    let first_t = cols.iter().find_map(|(_, c)| match c {
        ColumnText::Ready(t) => Some(*t),
        _ => None,
    });
    // Heading across all columns.
    {
        let wide = Ctx { width: full, ..cx.clone() };
        let mut hb = Builder {
            cx: &wide,
            fonts: b.fonts,
            blocks: Vec::new(),
            labels: Vec::new(),
            y: 0.0,
            x0: 0.0,
            column: 0,
            pending: None,
            space: 0.0,
            after_heading: false,
        };
        chapter_head(&mut hb, first_t, c, false);
        b.labels = hb.labels;
        b.y = hb.y;
    }
    // Column captions (translation abbreviations).
    let p = cx.theme.palette();
    for (i, (abbrev, _)) in cols.iter().enumerate() {
        let mut job = LayoutJob::default();
        job.append(
            abbrev,
            0.0,
            TextFormat {
                font_id: FontId::new(11.5, theme::family(UI_MEDIUM)),
                color: p.muted,
                extra_letter_spacing: 1.0,
                ..Default::default()
            },
        );
        let g = b.fonts.layout_job(job);
        b.labels.push((vec2(i as f32 * (col_w + cx.gutter), b.y), g));
    }
    b.y += em * 1.4;

    // Verse numbers to show: union across columns, in order.
    let mut numbers: Vec<u8> = Vec::new();
    for (_, col) in cols {
        if let ColumnText::Ready(t) = col {
            numbers.extend(t.verses_in(c).map(|v| v.number));
        }
    }
    numbers.sort_unstable();
    numbers.dedup();

    // Messages for columns that have no text (loading, missing, no key).
    let mut msg_rows = false;
    for (i, (_, col)) in cols.iter().enumerate() {
        if let ColumnText::Message(m) = col {
            let mut job = LayoutJob::default();
            job.wrap.max_width = col_w;
            job.append(m, 0.0, cx.fmt(SERIF_ITALIC, (em * 0.85).round(), p.muted));
            let g = b.fonts.layout_job(job);
            b.labels.push((vec2(i as f32 * (col_w + cx.gutter), b.y), g));
            msg_rows = true;
        }
    }
    if msg_rows && numbers.is_empty() {
        b.y += em * 3.0;
    }

    for num in numbers {
        let row_top = b.y;
        let mut row_h = 0.0f32;
        for (ci, (_, col)) in cols.iter().enumerate() {
            let ColumnText::Ready(t) = col else { continue };
            let Some(v) = t.verses_in(c).find(|v| v.number == num) else { continue };
            let mut job = LayoutJob::default();
            job.wrap.max_width = col_w;
            // Psalm title before verse 1.
            for m in v.marks.iter().filter(|m| m.offset == 0 && m.kind == MarkKind::PsalmTitle) {
                let mut f = cx.fmt(SERIF_ITALIC, (em * 0.85).round(), p.muted);
                f.line_height = Some((em * 1.35).round());
                job.append(&format!("{}\n", t.mark_text(m)), 0.0, f);
            }
            if cx.typo.verse_numbers {
                job.append(&format!("{}\u{a0}", v.number), 0.0, cx.verse_number_fmt());
            }
            let styles = style_mask(&v);
            let mut cursor = 0usize;
            let mut lead = 0.0;
            for m in v.marks.iter().filter(|m| m.kind.breaks_line() && m.offset > 0) {
                let at = (m.offset as usize).min(v.text.len());
                if at > cursor {
                    push_styled_lead(&mut job, cx, &v.text[cursor..at], &styles, cursor, lead);
                    cursor = at;
                    job.append("\n", 0.0, cx.body(0));
                    // Indent the next line (poetry level 2+) via leading space on its first section.
                    lead = if m.kind == MarkKind::Poetry {
                        em * 1.2 * m.level.saturating_sub(1) as f32
                    } else {
                        0.0
                    };
                }
            }
            if cursor < v.text.len() {
                push_styled_lead(&mut job, cx, &v.text[cursor..], &styles, cursor, lead);
            }
            let g = b.fonts.layout_job(job);
            row_h = row_h.max(g.size().y);
            b.blocks.push(Block {
                pos: vec2(ci as f32 * (col_w + cx.gutter), row_top),
                galley: g,
                verses: vec![(0, v.index, v.number)],
                gutter: None,
                column: ci,
            });
        }
        b.y = row_top + row_h + em * 0.55;
    }
    let anchors = anchors_for(&b.blocks);
    ChapterLayout { chapter: c, height: b.y + em * 1.2, blocks: b.blocks, anchors, labels: b.labels }
}

/// A short message laid out where a chapter would be (loading, missing text, …).
pub fn layout_message(
    fonts: &mut FontsView<'_>,
    cx: &Ctx<'_>,
    c: ChapterIdx,
    message: &str,
) -> ChapterLayout {
    let mut b = Builder {
        cx,
        fonts,
        blocks: Vec::new(),
        labels: Vec::new(),
        y: 0.0,
        x0: 0.0,
        column: 0,
        pending: None,
        space: 0.0,
        after_heading: false,
    };
    chapter_head(&mut b, None, c, true);
    let em = cx.typo.size;
    let mut job = LayoutJob::default();
    job.wrap.max_width = cx.width;
    job.halign = Align::Center;
    job.append(message, 0.0, cx.fmt(SERIF_ITALIC, em, cx.theme.palette().muted));
    b.label(job, em, em * 2.0, false);
    if let Some((pos, _)) = b.labels.last_mut() {
        pos.x = cx.width / 2.0;
    }
    let y = b.y;
    ChapterLayout {
        chapter: c,
        height: y + em * 4.0,
        blocks: Vec::new(),
        anchors: Vec::new(),
        labels: b.labels,
    }
}
