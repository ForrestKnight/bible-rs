//! Chapter typesetting: turns a chapter of the verse model into positioned text galleys.
//!
//! Layout depends only on the text, the typography settings and the column width — never on
//! highlights, selection or search hits. Those are painted underneath the text from glyph
//! geometry (see [`Block::char_rects`]), so clicking a verse or highlighting it never forces a
//! relayout. Each block records where its verses' text lies in it ([`TextRun`]), so a highlight
//! of part of a verse, counted in the verse's text, finds its glyphs.

use std::ops::Range;
use std::sync::Arc;

use bible_core::model::{MarkKind, Style, Translation, VerseView};
use bible_core::{BookId, ChapterIdx};
use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{Align, Color32, FontId, Galley, Rect, Vec2, pos2, vec2};
use eframe::epaint::text::FontsView;
use serde::{Deserialize, Serialize};

use crate::theme::{
    self, Palette, SERIF, SERIF_DISPLAY, SERIF_HEADING, SERIF_ITALIC, SERIF_SEMIBOLD, Theme, UI, UI_SEMIBOLD,
    reading, text,
};

/// The design is drawn at a 20 pt reading size; its reading metrics scale with the user's size.
pub const DESIGN_EM: f32 = text::BODY.size;
/// From the toolbar to the top of a chapter scrolled to its start (headings add their own space).
pub const PAGE_TOP: f32 = theme::space::S6;
/// From the text column to the margin markers.
pub const MARKER_GAP: f32 = theme::space::S8;
/// Blank space after a chapter. At least [`PAGE_TOP`] plus the toolbar, so the end of the previous
/// chapter is out of sight when a chapter is scrolled to its start.
const CHAPTER_END: f32 = 80.0;

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
    /// Focus mode: the chapter's head is one quiet line ("JOHN 15 · KJV").
    #[serde(skip)]
    pub quiet_head: bool,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            size: reading::SIZE_DEFAULT,
            line_height: reading::LINE_HEIGHT_RATIO,
            mode: Mode::Paragraph,
            red_letters: true,
            justify: false,
            verse_numbers: true,
            headings: true,
            measure: reading::MEASURE / reading::SIZE_DEFAULT,
            quiet_head: false,
        }
    }
}

/// Where a stretch of a verse's text is in a block: `len` chars from galley char `at` are the
/// verse's text from char `offset` on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRun {
    pub at: usize,
    /// Global verse index.
    pub verse: u32,
    pub offset: usize,
    pub len: usize,
}

/// A run of text placed in a chapter (relative to the chapter's top-left corner).
pub struct Block {
    pub pos: Vec2,
    pub galley: Arc<Galley>,
    /// `(first char, global verse index, verse number)` — chars from `first char` on belong to it.
    pub verses: Vec<(usize, u32, u8)>,
    /// The verses' text in the galley, in order. Verse numbers, psalm titles and the spaces the
    /// layout adds between verses or for line breaks aren't in any.
    pub runs: Vec<TextRun>,
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

    /// The part of a verse's char range that is its text: after its number, before the space
    /// that separates it from the next verse.
    pub fn text_chars(&self, range: Range<usize>) -> Range<usize> {
        let text = &self.galley.job.text;
        let chars: Vec<char> =
            text.chars().skip(range.start).take(range.end.saturating_sub(range.start)).collect();
        let mut start = 0;
        while start < chars.len() && (chars[start].is_ascii_digit() || chars[start] == '–') {
            start += 1;
        }
        if start > 0 && chars.get(start) == Some(&'\u{a0}') {
            start += 1;
        } else if start > 0 {
            start = 0;
        }
        let mut end = chars.len();
        while end > start && chars[end - 1].is_whitespace() {
            end -= 1;
        }
        range.start + start..range.start + end
    }

    /// The galley chars showing chars `range` of a verse's text, when the block has any of them.
    pub fn galley_range(&self, verse: u32, range: Range<usize>) -> Option<Range<usize>> {
        let mut out: Option<Range<usize>> = None;
        for r in self.runs.iter().filter(|r| r.verse == verse) {
            let s = range.start.max(r.offset);
            let e = range.end.min(r.offset + r.len);
            if s < e {
                let g = r.at + (s - r.offset)..r.at + (e - r.offset);
                out = Some(out.map_or(g.clone(), |o| o.start.min(g.start)..o.end.max(g.end)));
            }
        }
        out
    }

    /// The verse (global index) and the char of its text at galley char `ch`. A position in a
    /// verse number or an added space goes to the nearest verse text: the start of the verse that
    /// follows, or the end of the one before.
    pub fn text_pos(&self, ch: usize) -> Option<(u32, usize)> {
        let i = self.runs.partition_point(|r| r.at + r.len < ch);
        let r = match (self.runs.get(i), i.checked_sub(1).and_then(|k| self.runs.get(k))) {
            (Some(r), _) if r.at <= ch => r,
            // Before a run: the end of the previous verse's run if this is its trailing space,
            // else the start of the next run.
            (Some(next), Some(prev)) if prev.verse == next.verse || ch > prev.at + prev.len + 1 => {
                return Some((next.verse, next.offset));
            }
            (Some(next), None) => return Some((next.verse, next.offset)),
            (_, Some(prev)) => return Some((prev.verse, prev.offset + prev.len)),
            (None, None) => return None,
        };
        Some((r.verse, r.offset + (ch - r.at).min(r.len)))
    }

    /// The galley char at char `offset` of a verse's text, if the block has it: where a run starts
    /// rather than where the one before it ends, at a line break between them.
    pub fn galley_char(&self, verse: u32, offset: usize) -> Option<usize> {
        let runs = || self.runs.iter().filter(|r| r.verse == verse);
        let r = runs()
            .find(|r| (r.offset..r.offset + r.len).contains(&offset))
            .or_else(|| runs().find(|r| r.offset + r.len == offset))?;
        Some(r.at + offset - r.offset)
    }

    /// Baselines and horizontal extents (block-relative) covering a char range, one per row.
    pub fn char_runs(&self, range: Range<usize>) -> Vec<(f32, f32, f32)> {
        let mut out = Vec::new();
        let mut base = 0usize;
        for row in &self.galley.rows {
            let n = row.glyphs.len();
            let s = range.start.max(base);
            let e = range.end.min(base + n);
            if s < e {
                let g0 = &row.glyphs[s - base];
                let g1 = &row.glyphs[e - base - 1];
                let baseline = row.pos.y + row.glyphs.iter().map(|g| g.pos.y).fold(0.0, f32::max);
                out.push((row.pos.x + g0.pos.x, row.pos.x + g1.pos.x + g1.advance_width, baseline));
            }
            base += n + usize::from(row.ends_with_newline);
        }
        out
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
    /// Areas that open a web page when clicked (publishers' notices), relative to the chapter.
    pub links: Vec<(Rect, &'static str)>,
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
    /// How far blocks move down so their baselines land where the design (CSS) puts them; see
    /// [`css_shift`].
    pub body_shift: f32,
    /// Line height for inline verse numbers, bottom-aligned, chosen so they sit as in the design.
    pub number_lh: f32,
}

fn ascent(fonts: &mut FontsView<'_>, font: FontId) -> f32 {
    baseline(fonts, &TextFormat { font_id: font, ..Default::default() })
}

/// Where egui puts the baseline of a one-line galley in `format`, from the top of its row.
pub(crate) fn baseline(fonts: &mut FontsView<'_>, format: &TextFormat) -> f32 {
    let g = fonts.layout_job(LayoutJob::single_section("H".into(), format.clone()));
    g.rows.first().and_then(|r| r.glyphs.first()).map_or(0.0, |gl| gl.pos.y)
}

/// CSS centres a font's content area (ascender + descender) in its line box, so its baseline lies
/// this many ems below the box's centre: (ascender − descender) / 2, from the fonts' hhea tables.
pub const LITERATA_BASELINE: f32 = (1.177 - 0.308) / 2.0;
pub const INTER_BASELINE: f32 = (1984.0 - 494.0) / 2048.0 / 2.0;

/// egui places a baseline at the font's ascent from the top of its row, whatever the line height,
/// and nudges it to centre the font in its family's tallest fallback. This is how far to move a
/// galley in `format` (with an explicit line height) so its baseline matches CSS.
/// How far down egui's text sits from where CSS would put it in a line of `format`'s height.
pub(crate) fn css_shift(fonts: &mut FontsView<'_>, format: &TextFormat) -> f32 {
    let lh = format.line_height.unwrap_or_else(|| fonts.row_height(&format.font_id));
    let reading =
        matches!(&format.font_id.family, eframe::egui::FontFamily::Name(n) if n.starts_with("serif"));
    let k = if reading { LITERATA_BASELINE } else { INTER_BASELINE };
    lh / 2.0 + k * format.font_id.size - baseline(fonts, format)
}

impl<'a> Ctx<'a> {
    pub fn new(typo: &'a Typography, theme: Theme, width: f32, gutter: f32) -> Self {
        Self { typo, theme, width, gutter, small_caps_lh: 0.0, body_shift: 0.0, number_lh: 0.0 }
    }

    /// Fills in font-metric-dependent values.
    fn measured(&self, fonts: &mut FontsView<'_>) -> Self {
        let body = FontId::new(self.typo.size, theme::family(SERIF));
        let big = ascent(fonts, body.clone());
        let small = ascent(fonts, FontId::new(self.small_caps_size(), theme::family(SERIF)));
        let body_shift = css_shift(fonts, &self.body(0));
        // Numbers have their own 20 pt line box at the top of the line (at 20 pt): their CSS
        // baseline is that box's, less the body's shift, which the whole block gets.
        let mut number = self.verse_number_fmt();
        number.valign = Align::TOP;
        let target = self.s(10.0) + INTER_BASELINE * number.font_id.size - body_shift;
        let down = (target - baseline(fonts, &number)).clamp(0.0, self.lh() - 1.0);
        Self {
            small_caps_lh: self.lh() - (big - small),
            body_shift,
            number_lh: self.lh() - down,
            ..self.clone()
        }
    }
}

impl Ctx<'_> {
    fn p(&self) -> &'static Palette {
        self.theme.palette()
    }

    fn lh(&self) -> f32 {
        (self.typo.size * self.typo.line_height).round()
    }

    /// Design points scaled to the reading size.
    fn s(&self, pt: f32) -> f32 {
        pt * self.typo.size / DESIGN_EM
    }

    /// LORD's smaller capitals: 15 pt at 20.
    fn small_caps_size(&self) -> f32 {
        self.s(15.0).round()
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
        let color =
            if self.typo.red_letters && styles & Style::WordsOfJesus.bit() != 0 { p.rubric } else { p.ink };
        self.fmt(family, self.typo.size, color)
    }

    /// Inline verse numbers: 11 pt semibold at 20, raised toward the top of the line.
    fn verse_number_fmt(&self) -> TextFormat {
        TextFormat {
            font_id: FontId::new(self.s(11.0).round(), theme::family(UI_SEMIBOLD)),
            color: self.p().verse,
            line_height: Some(if self.number_lh > 0.0 { self.number_lh } else { self.lh() }),
            valign: Align::BOTTOM,
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

/// Like [`push_styled`], with `lead` points of leading space before the first character (the
/// first run then always starts a new section, since leading space belongs to a section).
fn push_styled_lead(
    job: &mut LayoutJob,
    cx: &Ctx<'_>,
    text: &str,
    styles_at: &dyn Fn(usize) -> u8,
    base: usize,
    lead: f32,
) {
    let mut lead = lead;
    let mut emit = |job: &mut LayoutJob, s: usize, e: usize, st: u8| {
        if s >= e {
            return;
        }
        let chunk = &text[s..e];
        if st & (Style::DivineName.bit() | Style::SmallCaps.bit()) != 0 {
            small_caps(job, cx, chunk, st, std::mem::take(&mut lead));
        } else if lead > 0.0 {
            job.append(chunk, std::mem::take(&mut lead), cx.body(st));
        } else {
            append(job, chunk, cx.body(st));
        }
    };
    let caps = caps_words(text);
    let mut run_start = 0usize;
    let mut run_style = None;
    for (i, _) in text.char_indices() {
        let mut st = styles_at(base + i);
        if caps.iter().any(|r| r.contains(&i)) {
            st |= Style::SmallCaps.bit();
        }
        match run_style {
            None => run_style = Some(st),
            Some(prev) if prev != st => {
                emit(job, run_start, i, prev);
                run_start = i;
                run_style = Some(st);
            }
            _ => {}
        }
    }
    if let Some(st) = run_style {
        emit(job, run_start, text.len(), st);
    }
}

pub(crate) use bible_core::model::caps_words;

/// "LORD"/"Lord" → L + ᴏʀᴅ: the first letter of each word at full size, the rest upper-cased smaller.
fn small_caps(job: &mut LayoutJob, cx: &Ctx<'_>, text: &str, styles: u8, lead: f32) {
    let full = cx.body(styles & !(Style::DivineName.bit() | Style::SmallCaps.bit()));
    let mut small = full.clone();
    small.font_id.size = cx.small_caps_size();
    small.extra_letter_spacing = 0.04 * small.font_id.size;
    if cx.small_caps_lh > 0.0 {
        small.line_height = Some(cx.small_caps_lh);
    }
    let mut lead = lead;
    let mut word_start = true;
    let mut buf = String::new();
    let mut buf_small = false;
    let mut emit = |job: &mut LayoutJob, buf: &mut String, is_small: bool| {
        if !buf.is_empty() {
            let fmt = if is_small { small.clone() } else { full.clone() };
            if lead > 0.0 {
                job.append(buf, std::mem::take(&mut lead), fmt);
            } else {
                append(job, buf, fmt);
            }
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
    runs: Vec<TextRun>,
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
    /// Inside a psalm's opening heading (between the PSALM label and the first verse).
    psalm_head: bool,
}

impl<'c, 'f> Builder<'c, 'f> {
    fn new(cx: &'c Ctx<'c>, fonts: &'c mut FontsView<'f>) -> Self {
        Self {
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
            psalm_head: false,
        }
    }
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
        self.pending =
            Some(Pending { job, chars: 0, verses: Vec::new(), runs: Vec::new(), kind, gutter: None });
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
        let poetry_indent = self.cx.s(reading::POETRY_INDENT);
        let (indent, hang) = match p.kind {
            Kind::Poetry(level) => (poetry_indent * (level.saturating_sub(1)) as f32, poetry_indent),
            _ => (0.0, 0.0),
        };
        if let Kind::Prose { indent: true } = p.kind
            && let Some(s) = p.job.sections.first_mut()
        {
            s.leading_space = self.cx.s(reading::FIRST_LINE_INDENT);
        }
        p.job.wrap.max_width = (self.cx.width - indent - hang).max(em * 6.0);
        // Paragraphs are 16 pt apart; in verse-per-line mode every verse is, 8 pt apart.
        let gap = match p.kind {
            Kind::Prose { .. } if self.cx.typo.mode == Mode::VersePerLine => self.cx.s(8.0),
            Kind::Prose { .. } => self.cx.s(reading::PARAGRAPH_GAP),
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
        let pos = vec2(x, self.y + self.cx.body_shift);
        let gutter = p.gutter.map(|n| {
            let fmt = TextFormat {
                font_id: FontId::new(self.cx.s(12.0).round(), theme::family(UI_SEMIBOLD)),
                color: self.cx.p().verse,
                ..Default::default()
            };
            let g = self.fonts.layout_job(LayoutJob::single_section(n.to_string(), fmt));
            // Right-aligned 12 pt before the text, centred on the first line.
            let gp = vec2(self.x0 - g.size().x - self.cx.s(12.0), self.y + (self.cx.lh() - g.size().y) / 2.0);
            (g, gp)
        });
        self.y += galley.size().y;
        self.blocks.push(Block { pos, galley, verses: p.verses, runs: p.runs, gutter, column: self.column });
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

    /// Places a heading or label at the current position. `line_height` is the CSS line box of
    /// `font`; the galley moves down by half its leading so the baseline lands where CSS puts it.
    fn place(&mut self, job: LayoutJob, center: bool) {
        let shift = job.sections.first().map_or(0.0, |s| css_shift(self.fonts, &s.format));
        let g = self.fonts.layout_job(job);
        let x = if center { self.x0 + self.cx.width / 2.0 } else { self.x0 };
        self.labels.push((vec2(x, self.y + shift), g.clone()));
        self.y += g.size().y;
    }

    /// A line in the design's label style (11 pt semibold capitals, tracked, in the accent).
    fn label_line(&mut self, text: &str, center: bool) {
        let accent = self.cx.p().accent;
        self.label_line_in(text, center, accent);
    }

    fn label_line_in(&mut self, text: &str, center: bool, color: Color32) {
        let font = FontId::new(self.cx.s(text::LABEL.size).round(), theme::family(UI_SEMIBOLD));
        let lh = self.cx.s(text::LABEL.line_height);
        let mut job = LayoutJob::default();
        job.append(
            text,
            0.0,
            TextFormat {
                font_id: font.clone(),
                color,
                extra_letter_spacing: text::LABEL.tracking * font.size,
                line_height: Some(lh),
                ..Default::default()
            },
        );
        job.wrap.max_width = self.cx.width;
        if center {
            job.halign = Align::Center;
        }
        self.place(job, center);
    }

    /// A line of Literata in one of the design's type styles; words in capitals (LORD, GOD) are
    /// set in small capitals at three quarters of the size, as in the text.
    #[allow(clippy::too_many_arguments)]
    fn serif_line(
        &mut self,
        text: &str,
        family: &str,
        size: f32,
        line_height: f32,
        tracking: f32,
        color: Color32,
        center: bool,
    ) {
        let font = FontId::new(size, theme::family(family));
        let fmt = TextFormat {
            font_id: font.clone(),
            color,
            extra_letter_spacing: tracking * size,
            line_height: Some(line_height),
            ..Default::default()
        };
        let mut small = fmt.clone();
        small.font_id.size = (size * 0.75).round();
        small.extra_letter_spacing = 0.03 * small.font_id.size;
        let (big, little) = (ascent(self.fonts, font.clone()), ascent(self.fonts, small.font_id.clone()));
        small.line_height = Some(line_height - (big - little));
        let mut job = LayoutJob::default();
        let mut has_small = false;
        for (i, word) in text.split(' ').enumerate() {
            if i > 0 {
                append(&mut job, " ", fmt.clone());
            }
            let caps = word.chars().filter(|c| c.is_alphabetic()).count() >= 2
                && word.chars().all(|c| !c.is_alphabetic() || c.is_uppercase());
            match word.char_indices().nth(1).filter(|_| caps) {
                Some((second, _)) => {
                    append(&mut job, &word[..second], fmt.clone());
                    append(&mut job, &word[second..], small.clone());
                    has_small = true;
                }
                None => append(&mut job, word, fmt.clone()),
            }
        }
        job.wrap.max_width = self.cx.width;
        if center {
            job.halign = Align::Center;
        }
        self.place(job, center);
        if has_small {
            // In the design the smaller capitals, aligned on the baseline, make the line box
            // taller by the difference of the two sizes' baselines.
            self.y += (size - small.font_id.size) * LITERATA_BASELINE;
        }
    }

    fn heading(&mut self, kind: MarkKind, _level: u8, text: &str) {
        if !self.cx.typo.headings && kind == MarkKind::Heading {
            return;
        }
        let p = self.cx.p();
        let s = |pt: f32| self.cx.s(pt);
        // A psalm opens with its heading and title under the "PSALM n" label.
        let psalm_head = self.psalm_head && self.blocks.is_empty();
        let (family, style, color, before, after) = match kind {
            MarkKind::PsalmTitle => {
                (SERIF_ITALIC, text::SUBTITLE, p.ink_2, if psalm_head { 0.0 } else { s(4.0) }, s(8.0))
            }
            MarkKind::Speaker => (SERIF_ITALIC, text::SUBTITLE, p.ink_2, s(12.0), s(4.0)),
            _ if psalm_head => (SERIF_HEADING, text::HEADING, p.ink, s(4.0), 0.0),
            _ => (SERIF_SEMIBOLD, text::SUBTITLE, p.ink, s(24.0), s(8.0)),
        };
        self.flush();
        self.y += if psalm_head { before } else { before.max(self.space) };
        self.space = 0.0;
        let (size, lh) = (s(style.size).round(), s(style.line_height));
        self.serif_line(text, family, size, lh, style.tracking, color, false);
        if psalm_head {
            // The poem starts 16 pt below whichever of heading and title comes last.
            self.space = s(16.0);
        } else {
            self.y += after;
        }
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

/// The heading at the top of a chapter. Chapter 1 shows the book's title, its long name and a
/// CHAPTER 1 label; later chapters a book-name label over a large numeral; psalms a left-aligned
/// PSALM n label followed by the psalm's own heading and title.
fn chapter_head(b: &mut Builder<'_, '_>, t: Option<&Translation>, c: ChapterIdx, single_column: bool) {
    let cx = b.cx;
    let p = cx.p();
    let short =
        t.map(|t| t.meta.book_short(c.book()).to_string()).unwrap_or_else(|| c.book().name().to_string());
    if cx.typo.quiet_head {
        // Focus mode: the book, chapter and translation in one tertiary line, 16 pt above a
        // 20 pt gap.
        let book = if c.book() == BookId::PSALMS { "Psalm".to_string() } else { short.clone() };
        let abbrev = t.map(|t| t.meta.abbrev.clone()).unwrap_or_default();
        let label = format!("{} {} · {abbrev}", book.to_uppercase(), c.chapter());
        b.label_line_in(&label, single_column, p.ink_3);
        // The design's spacing in points, whatever the size: 16 under the 16 pt label, then 20.
        b.y += 16.0 - (cx.s(text::LABEL.line_height) - text::LABEL.line_height);
        b.space = 20.0;
        b.after_heading = true;
        return;
    }
    if c.book() == BookId::PSALMS && single_column {
        b.label_line(&format!("PSALM {}", c.chapter()), false);
        b.psalm_head = true;
        b.space = cx.s(16.0);
        b.after_heading = true;
        return;
    }
    let display = (cx.s(text::DISPLAY.size).round(), cx.s(text::DISPLAY.line_height), text::DISPLAY.tracking);
    b.y += cx.s(16.0);
    if c.chapter() == 1 {
        b.serif_line(&short, SERIF_DISPLAY, display.0, display.1, display.2, p.ink, true);
        let long = t
            .map(|t| t.meta.books.get(c.book().index()).map(|b| b.long.clone()).unwrap_or_default())
            .unwrap_or_default();
        if !long.is_empty() && long != short && single_column {
            b.y += cx.s(4.0);
            let (size, lh) = (cx.s(text::SUBTITLE.size).round(), cx.s(text::SUBTITLE.line_height));
            b.serif_line(&long, SERIF_ITALIC, size, lh, 0.0, p.ink_2, true);
        }
        b.y += cx.s(16.0);
        b.label_line(&format!("CHAPTER {}", c.chapter()), true);
    } else {
        b.label_line(&short.to_uppercase(), true);
        b.y += cx.s(4.0);
        b.serif_line(&c.chapter().to_string(), SERIF_DISPLAY, display.0, display.1, display.2, p.ink, true);
    }
    // 24 pt under the heading, then the column's 16 pt gap before the text.
    b.y += cx.s(24.0);
    b.space = cx.s(16.0);
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
    let mut b = Builder::new(cx, fonts);
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
                    // Verse-per-line, and poetry lines that begin a verse, hang their number in
                    // the margin so the text of every line aligns.
                    if (per_line || matches!(p.kind, Kind::Poetry(_))) && p.chars == 0 {
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
                let len = seg.chars().count();
                p.runs.push(TextRun {
                    at: p.chars,
                    verse: v.index,
                    offset: v.text[..cursor].chars().count(),
                    len,
                });
                p.chars += len;
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
    ChapterLayout { chapter: c, height: y + cx.s(CHAPTER_END), blocks, anchors, labels, links: Vec::new() }
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

/// Parallel rows (frame 07): each verse a row across the columns, padded 6 × 8 and rounded so the
/// selected one can be marked whole; a 24 pt number lane, then Literata at 17/28 (at a 20 pt
/// reading size).
pub const PARALLEL_ROW_PAD: Vec2 = vec2(8.0, 6.0);
const PARALLEL_NUMBER_LANE: f32 = 24.0;

/// Lays out a chapter as verse-aligned rows across several translations.
pub fn layout_parallel(
    fonts: &mut FontsView<'_>,
    cx: &Ctx<'_>,
    cols: &[(String, ColumnText<'_>)],
    c: ChapterIdx,
) -> ChapterLayout {
    // The columns' own, smaller type; spacing still scales with the reading size.
    let outer = cx;
    let s = |pt: f32| outer.s(pt);
    let typo = Typography {
        size: s(17.0).round(),
        line_height: s(28.0).round() / s(17.0).round(),
        ..cx.typo.clone()
    };
    let small = Ctx { typo: &typo, ..cx.clone() };
    let cx = &small.measured(fonts);
    let col_w = cx.width;
    let n = cols.len().max(1);
    let full = PARALLEL_ROW_PAD.x * 2.0 + col_w * n as f32 + cx.gutter * (n as f32 - 1.0);
    let p = cx.theme.palette();
    let mut b = Builder::new(cx, fonts);
    // The chapter's label, centred across the columns.
    {
        let wide = Ctx { width: full, ..outer.clone() };
        let mut hb = Builder::new(&wide, b.fonts);
        hb.x0 = -PARALLEL_ROW_PAD.x;
        let first_t = cols.iter().find_map(|(_, c)| match c {
            ColumnText::Ready(t) => Some(*t),
            _ => None,
        });
        let label = if c.book() == BookId::PSALMS {
            format!("PSALM {}", c.chapter())
        } else {
            let name = first_t
                .map(|t| t.meta.book_short(c.book()).to_string())
                .unwrap_or_else(|| c.book().name().into());
            format!("{} {}", name.to_uppercase(), c.chapter())
        };
        hb.label_line(&label, true);
        b.labels = hb.labels;
        b.y = hb.y + s(16.0);
    }

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
    let col_x = |i: usize| PARALLEL_NUMBER_LANE + i as f32 * (col_w + cx.gutter);
    let text_w = col_w - PARALLEL_NUMBER_LANE;
    let mut msg_rows = false;
    for (i, (_, col)) in cols.iter().enumerate() {
        if let ColumnText::Message(m) = col {
            let mut job = LayoutJob::default();
            job.wrap.max_width = text_w;
            job.append(m, 0.0, cx.fmt(SERIF_ITALIC, cx.typo.size, p.ink_2));
            let g = b.fonts.layout_job(job);
            b.labels.push((vec2(col_x(i), b.y + PARALLEL_ROW_PAD.y), g));
            msg_rows = true;
        }
    }
    if msg_rows && numbers.is_empty() {
        b.y += cx.lh() * 3.0;
    }

    let number_fmt = TextFormat {
        font_id: FontId::new(s(11.0).round(), theme::family(UI_SEMIBOLD)),
        color: p.verse,
        ..Default::default()
    };
    for num in numbers {
        let row_top = b.y + PARALLEL_ROW_PAD.y;
        let mut row_h = 0.0f32;
        for (ci, (_, col)) in cols.iter().enumerate() {
            let ColumnText::Ready(t) = col else { continue };
            let Some(v) = t.verses_in(c).find(|v| v.number == num) else { continue };
            let mut job = LayoutJob::default();
            job.wrap.max_width = text_w;
            // Psalm title before verse 1.
            for m in v.marks.iter().filter(|m| m.offset == 0 && m.kind == MarkKind::PsalmTitle) {
                let mut f = cx.fmt(SERIF_ITALIC, (cx.typo.size * 0.88).round(), p.ink_2);
                f.line_height = Some(cx.lh());
                job.append(&format!("{}\n", t.mark_text(m)), 0.0, f);
            }
            // Each verse flows as one paragraph in its column; poetry's line breaks become spaces.
            let styles = style_mask(&v);
            let mut cursor = 0usize;
            let mut runs = Vec::new();
            let mut push = |job: &mut LayoutJob, cursor: usize, end: usize| {
                let at = job.text.chars().count();
                let seg = &v.text[cursor..end];
                push_styled(job, cx, seg, &styles, cursor);
                let (offset, len) = (v.text[..cursor].chars().count(), seg.chars().count());
                runs.push(TextRun { at, verse: v.index, offset, len });
            };
            for m in v.marks.iter().filter(|m| m.kind.breaks_line() && m.offset > 0) {
                let at = (m.offset as usize).min(v.text.len());
                if at > cursor {
                    push(&mut job, cursor, at);
                    cursor = at;
                    if !job.text.ends_with(' ') && !v.text[at..].starts_with(' ') {
                        append(&mut job, " ", cx.body(0));
                    }
                }
            }
            if cursor < v.text.len() {
                push(&mut job, cursor, v.text.len());
            }
            let g = b.fonts.layout_job(job);
            row_h = row_h.max(g.size().y);
            let gutter = cx.typo.verse_numbers.then(|| {
                let ng =
                    b.fonts.layout_job(LayoutJob::single_section(v.number.to_string(), number_fmt.clone()));
                let pos = vec2(ci as f32 * (col_w + cx.gutter), row_top + (cx.lh() - ng.size().y) / 2.0);
                (ng, pos)
            });
            b.blocks.push(Block {
                pos: vec2(col_x(ci), row_top + cx.body_shift),
                galley: g,
                verses: vec![(0, v.index, v.number)],
                runs,
                gutter,
                column: ci,
            });
        }
        b.y = row_top + row_h + PARALLEL_ROW_PAD.y;
    }
    let anchors = anchors_for(&b.blocks);
    ChapterLayout {
        chapter: c,
        height: b.y + s(CHAPTER_END),
        blocks: b.blocks,
        anchors,
        labels: b.labels,
        links: Vec::new(),
    }
}

/// A short message laid out where a chapter would be (loading, missing text, …).
pub fn layout_message(
    fonts: &mut FontsView<'_>,
    cx: &Ctx<'_>,
    c: ChapterIdx,
    message: &str,
) -> ChapterLayout {
    let mut b = Builder::new(cx, fonts);
    chapter_head(&mut b, None, c, true);
    let em = cx.typo.size;
    let mut job = LayoutJob::default();
    job.wrap.max_width = cx.width;
    job.halign = Align::Center;
    job.append(message, 0.0, cx.fmt(SERIF_ITALIC, em, cx.theme.palette().ink_2));
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
        links: Vec::new(),
    }
}

/// A publisher's attribution under a chapter of an online translation, and the site it links
/// to: the ESV, NLT, NET and API.Bible terms all ask for their notice with the text, and some for
/// a link.
pub struct Notice {
    pub abbrev: String,
    pub text: String,
    pub url: &'static str,
}

/// Adds notices under a laid-out chapter (`columns` side by side), each a link.
pub fn add_notices(
    fonts: &mut FontsView<'_>,
    cx: &Ctx<'_>,
    l: &mut ChapterLayout,
    notices: &[Notice],
    columns: usize,
) {
    if notices.is_empty() {
        return;
    }
    // Parallel rows start after the verse-number lane and span every column.
    let (x, width) = if columns > 1 {
        let n = columns as f32;
        (PARALLEL_NUMBER_LANE, cx.width * n + cx.gutter * (n - 1.0) - PARALLEL_NUMBER_LANE)
    } else {
        (0.0, cx.width)
    };
    let p = cx.theme.palette();
    let format = |family: &str, color| TextFormat {
        font_id: FontId::new(cx.s(11.0).round(), theme::family(family)),
        color,
        line_height: Some(cx.s(16.0).round()),
        ..Default::default()
    };
    let mut y = l.height - cx.s(CHAPTER_END) + cx.s(16.0);
    for n in notices {
        let mut job = LayoutJob::default();
        job.wrap.max_width = width;
        job.append(&n.abbrev, 0.0, format(UI_SEMIBOLD, p.accent));
        job.append(&n.text, cx.s(6.0), format(UI, p.ink_3));
        let g = fonts.layout_job(job);
        l.links.push((Rect::from_min_size(pos2(x, y), g.size()), n.url));
        l.labels.push((vec2(x, y), g.clone()));
        y += g.size().y + cx.s(8.0);
    }
    l.height = y + cx.s(CHAPTER_END);
}
