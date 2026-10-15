//! The reading view: continuous scroll through all 1,189 chapters.
//!
//! The scroll position is an *anchor* — a chapter plus a pixel offset into it — rather than a
//! position in one giant document, so only the chapters on screen are ever laid out and
//! scrolling from Genesis to Revelation costs the same per frame as scrolling one psalm.
//! Layouts are cached per chapter and invalidated when typography, width, theme or the set of
//! columns changes; a verse anchor keeps the reading position stable across those changes.

pub mod layout;

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use bible_core::books::{CHAPTER_COUNT, ChapterIdx, VerseRef};
use bible_core::library::{Chapter, Library};
use bible_core::study::{HighlightColor, StudyStore};
use eframe::egui::{
    self, Color32, CornerRadius, CursorIcon, Id, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2,
};

use self::layout::{ChapterLayout, ColumnText, Ctx, Typography};
use crate::actions::Action;
use crate::icons;
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchor {
    pub chapter: ChapterIdx,
    /// Distance from the chapter's top to the top of the viewport.
    pub offset: f32,
}

pub struct Env<'a> {
    pub library: &'a Library,
    pub study: &'a StudyStore,
    pub typo: &'a Typography,
    pub theme: Theme,
    /// Translation ids shown side by side (one for normal reading).
    pub columns: &'a [String],
    /// Whether arrow/page keys should scroll the reader (no text field has focus).
    pub keyboard: bool,
    /// Incremented whenever online text arrives (provisional layouts are redone).
    pub online_generation: &'a AtomicU64,
}

struct Cached {
    layout: Arc<ChapterLayout>,
    /// Laid out while some column was still loading; redo when online text arrives.
    provisional: Option<u64>,
}

pub struct Reader {
    pub anchor: Anchor,
    cache: HashMap<ChapterIdx, Cached>,
    key: u64,
    em: f32,
    /// Inclusive verse range.
    pub selection: Option<(VerseRef, VerseRef)>,
    flash: Option<(VerseRef, f64)>,
    pending_jump: Option<(ChapterIdx, u8, bool)>,
    reanchor: Option<(ChapterIdx, u8, f32)>,
    scroll_remaining: f32,
    viewport_h: f32,
    top_pad: f32,
    /// Chapter/verse currently at the reading line (for the toolbar and saved position).
    pub location: (ChapterIdx, u8),
    frames: u64,
}

impl Reader {
    pub fn new(chapter: ChapterIdx, verse: u8) -> Self {
        let mut r = Self {
            anchor: Anchor { chapter, offset: 0.0 },
            cache: HashMap::new(),
            key: 0,
            em: 19.0,
            selection: None,
            flash: None,
            pending_jump: None,
            reanchor: None,
            scroll_remaining: 0.0,
            viewport_h: 800.0,
            top_pad: 12.0,
            location: (chapter, verse.max(1)),
            frames: 0,
        };
        if verse > 1 {
            r.pending_jump = Some((chapter, verse, false));
        }
        r
    }

    pub fn jump(&mut self, chapter: ChapterIdx, verse: Option<u8>, flash: bool) {
        self.scroll_remaining = 0.0;
        self.pending_jump = Some((chapter, verse.unwrap_or(1), flash && verse.is_some()));
        self.location = (chapter, verse.unwrap_or(1));
    }

    pub fn scroll_by(&mut self, dy: f32) {
        self.scroll_remaining += dy;
    }

    pub fn page(&self) -> f32 {
        (self.viewport_h - self.em * 3.0).max(self.em * 4.0)
    }

    pub fn invalidate(&mut self) {
        self.cache.clear();
    }

    pub fn selected_refs(&self) -> Vec<VerseRef> {
        let Some((a, b)) = self.selection else { return Vec::new() };
        if a.book != b.book || a.chapter != b.chapter {
            return vec![a];
        }
        (a.verse..=b.verse).map(|v| VerseRef::new(a.book, a.chapter, v)).collect()
    }

    fn select(&mut self, v: VerseRef, extend: bool) {
        match (self.selection, extend) {
            (Some((a, _)), true) if a.book == v.book && a.chapter == v.chapter => {
                self.selection = Some(if v < a { (v, a) } else { (a, v) });
            }
            (Some((a, b)), false) if a == v && b == v => self.selection = None,
            _ => self.selection = Some((v, v)),
        }
    }

    pub fn set_selection(&mut self, v: Option<VerseRef>) {
        self.selection = v.map(|v| (v, v));
    }

    /// Moves a single-verse selection by `delta` verses (within the chapter, else across).
    pub fn step_selection(&mut self, library: &Library, id: &str, delta: i32) {
        let current = self.selection.map(|(a, b)| if delta > 0 { b } else { a });
        let base = current.unwrap_or_else(|| {
            VerseRef::new(self.location.0.book(), self.location.0.chapter(), self.location.1)
        });
        let Ok(t) = library.load(id) else { return };
        let c = base.chapter_idx();
        let mut target = None;
        if let Some(v) = t.find(base) {
            let idx = v.index as i64 + delta as i64;
            if current.is_none() {
                target = Some(v.reference());
            } else if idx >= 0 {
                target = t.verse_view(idx as u32).map(|v| v.reference());
            }
        } else if let Some(v) = t.verses_in(c).next() {
            target = Some(v.reference());
        }
        if let Some(r) = target {
            self.selection = Some((r, r));
            self.ensure_visible(r);
        }
    }

    fn ensure_visible(&mut self, r: VerseRef) {
        let c = r.chapter_idx();
        let Some(cached) = self.cache.get(&c) else {
            self.pending_jump = Some((c, r.verse, false));
            return;
        };
        let Some(a) = cached.layout.anchor(0, r.verse) else { return };
        // Distance from the viewport top to the verse.
        let mut y = a.top - self.anchor.offset;
        if c != self.anchor.chapter {
            if c > self.anchor.chapter {
                match self.cache.get(&self.anchor.chapter) {
                    Some(cur) if c.0 == self.anchor.chapter.0 + 1 => y += cur.layout.height,
                    _ => {
                        self.pending_jump = Some((c, r.verse, false));
                        return;
                    }
                }
            } else {
                self.pending_jump = Some((c, r.verse, false));
                return;
            }
        }
        let margin = self.em * 3.0;
        if y < margin {
            self.scroll_remaining = y - margin;
        } else if y + (a.bottom - a.top) > self.viewport_h - margin {
            self.scroll_remaining = y + (a.bottom - a.top) - self.viewport_h + margin;
        }
    }

    fn layout_key(env: &Env<'_>, width: f32) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        width.to_bits().hash(&mut h);
        serde_json::to_string(env.typo).unwrap_or_default().hash(&mut h);
        env.theme.hash(&mut h);
        env.columns.hash(&mut h);
        h.finish()
    }

    fn chapter_layout(
        &mut self,
        ui: &Ui,
        env: &Env<'_>,
        geom: &Geometry,
        c: ChapterIdx,
    ) -> Arc<ChapterLayout> {
        let gen_now = env.online_generation.load(Ordering::Relaxed);
        if let Some(cached) = self.cache.get(&c)
            && cached.provisional.is_none_or(|g| g == gen_now)
        {
            return cached.layout.clone();
        }
        let cx = Ctx::new(env.typo, env.theme, geom.col_w, geom.gutter);
        let mut provisional = false;
        let mut chapters: Vec<(String, Result<Arc<bible_core::Translation>, String>)> = Vec::new();
        for id in env.columns {
            let abbrev = env.library.get(id).map(|e| e.meta.abbrev.clone()).unwrap_or_else(|| id.clone());
            let state = match env.library.chapter(id, c) {
                Chapter::Ready(t) => Ok(t),
                Chapter::Pending => {
                    provisional = true;
                    Err("Loading…".to_string())
                }
                Chapter::NoKey => {
                    provisional = true;
                    Err("Add your API key in Settings → Translations to read this translation.".into())
                }
                Chapter::Failed(m) => {
                    provisional = true;
                    Err(format!("Couldn't load this chapter: {m}"))
                }
                Chapter::Missing => Err("This translation doesn't include this chapter.".into()),
            };
            chapters.push((abbrev, state));
        }
        let t_layout = std::time::Instant::now();
        let layout = ui.ctx().fonts_mut(|fonts| {
            if chapters.len() == 1 {
                match &chapters[0].1 {
                    Ok(t) => layout::layout_chapter(fonts, &cx, t, c),
                    Err(m) => layout::layout_message(fonts, &cx, c, m),
                }
            } else {
                let cols: Vec<(String, ColumnText<'_>)> = chapters
                    .iter()
                    .map(|(a, s)| {
                        (
                            a.clone(),
                            match s {
                                Ok(t) => ColumnText::Ready(t),
                                Err(m) => ColumnText::Message(m.clone()),
                            },
                        )
                    })
                    .collect();
                layout::layout_parallel(fonts, &cx, &cols, c)
            }
        });
        if std::env::var_os("BIBLE_RS_TRACE_STARTUP").is_some() && self.frames < 3 {
            eprintln!(
                "          layout {} took {:.2} ms",
                c.display(),
                t_layout.elapsed().as_secs_f64() * 1000.0
            );
        }
        let layout = Arc::new(layout);
        self.cache.insert(c, Cached { layout: layout.clone(), provisional: provisional.then_some(gen_now) });
        if self.cache.len() > 48 {
            // Keep the neighbourhood of the reading position.
            let here = self.anchor.chapter.0 as i32;
            self.cache.retain(|k, _| (k.0 as i32 - here).abs() <= 12);
        }
        layout
    }

    /// Shows the reader in `ui`, returning actions for the app (selection, notes, …).
    pub fn show(&mut self, ui: &mut Ui, env: &Env<'_>, actions: &mut Vec<Action>) {
        let rect = ui.available_rect_before_wrap();
        let p = env.theme.palette();
        ui.painter().rect_filled(rect, 0.0, p.paper);
        let id = Id::new("reader");
        let resp = ui.interact(rect, id, Sense::click_and_drag());
        let geom = Geometry::new(rect, env.typo, env.columns.len());
        self.em = env.typo.size;
        self.viewport_h = rect.height();

        // Re-key the cache when anything that affects layout changes, keeping the position.
        let key = Self::layout_key(env, geom.col_w);
        if key != self.key {
            if self.key != 0
                && self.pending_jump.is_none()
                && let Some(cached) = self.cache.get(&self.anchor.chapter)
                && let Some(a) = cached.layout.verse_at_y(self.anchor.offset)
            {
                let span = (a.bottom - a.top).max(1.0);
                self.reanchor = Some((self.anchor.chapter, a.number, (self.anchor.offset - a.top) / span));
            }
            self.cache.clear();
            self.key = key;
        }
        if let Some((c, verse, frac)) = self.reanchor.take() {
            let l = self.chapter_layout(ui, env, &geom, c);
            if let Some(a) = l.anchor(0, verse) {
                self.anchor = Anchor { chapter: c, offset: a.top + frac * (a.bottom - a.top) };
            }
        }
        if let Some((c, verse, flash)) = self.pending_jump.take() {
            let l = self.chapter_layout(ui, env, &geom, c);
            let offset = if verse <= 1 {
                0.0
            } else {
                l.anchor(0, verse).map_or(0.0, |a| (a.top - self.em * 2.2).max(0.0))
            };
            self.anchor = Anchor { chapter: c, offset };
            if flash {
                self.flash = Some((VerseRef::new(c.book(), c.chapter(), verse), ui.input(|i| i.time)));
            }
        }

        // Scrolling: wheel/trackpad (already smoothed by egui) and animated keyboard steps.
        let mut dy = 0.0;
        if resp.hovered() || resp.contains_pointer() {
            dy -= ui.input(|i| i.smooth_scroll_delta.y);
        }
        if self.scroll_remaining.abs() > 0.5 {
            let step = self.scroll_remaining * 0.28;
            let step = if step.abs() < 0.5 { self.scroll_remaining } else { step };
            dy += step;
            self.scroll_remaining -= step;
            ui.ctx().request_repaint();
        } else {
            self.scroll_remaining = 0.0;
        }
        // Scrollbar.
        let bar = Rect::from_min_max(
            pos2(rect.right() - 12.0, rect.top() + 4.0),
            pos2(rect.right() - 2.0, rect.bottom() - 4.0),
        );
        let bar_resp = ui.interact(bar, id.with("bar"), Sense::click_and_drag());
        if let Some(pos) =
            bar_resp.interact_pointer_pos().filter(|_| bar_resp.dragged() || bar_resp.clicked())
        {
            let f = ((pos.y - bar.top()) / bar.height()).clamp(0.0, 0.9999) * CHAPTER_COUNT as f32;
            let c = ChapterIdx(f.floor() as u16);
            let l = self.chapter_layout(ui, env, &geom, c);
            self.anchor = Anchor { chapter: c, offset: f.fract() * l.height };
            self.scroll_remaining = 0.0;
            dy = 0.0;
        }
        self.anchor.offset += dy;
        self.normalize(ui, env, &geom);

        // Paint chapters from the anchor down.
        let painter = ui.painter_at(rect);
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let now = ui.input(|i| i.time);
        let flash_alpha = self.flash.map(|(_, t0)| {
            let t = (now - t0) as f32;
            if t > 2.2 { 0.0 } else { (1.0 - (t - 0.8).max(0.0) / 1.4).clamp(0.0, 1.0) }
        });
        if flash_alpha.is_some_and(|a| a > 0.0) {
            ui.ctx().request_repaint();
        } else {
            self.flash = None;
        }
        let selected: Vec<VerseRef> = self.selected_refs();
        let mut y = rect.top() + self.top_pad - self.anchor.offset;
        let mut c = Some(self.anchor.chapter);
        let mut hit: Option<VerseRef> = None;
        let mut marker_hit: Option<VerseRef> = None;
        let mut first_visible: Option<(ChapterIdx, u8)> = None;
        let reading_line = rect.top() + rect.height() * 0.22;
        while let Some(ch) = c {
            if y > rect.bottom() {
                break;
            }
            let l = self.chapter_layout(ui, env, &geom, ch);
            let origin = vec2(geom.x0, y);
            for (pos, g) in &l.labels {
                let at = (origin + *pos).to_pos2();
                if at.y < rect.bottom() && at.y + g.size().y > rect.top() {
                    painter.galley(at, g.clone(), p.ink);
                }
            }
            for block in &l.blocks {
                let br = block.rect().translate(origin);
                if br.bottom() < rect.top() || br.top() > rect.bottom() {
                    continue;
                }
                let bo = origin + block.pos;
                // Underlays: highlights, selection, flash.
                for (k, (start, _vi, number)) in block.verses.iter().enumerate() {
                    let end = block.verses.get(k + 1).map_or(usize::MAX, |(s, _, _)| *s);
                    let vref = VerseRef::new(ch.book(), ch.chapter(), *number);
                    let mut fills: Vec<Color32> = Vec::new();
                    if let Some(hc) = env.study.highlight(vref) {
                        fills.push(p.highlight(hc));
                    }
                    let is_selected = selected.contains(&vref);
                    if is_selected {
                        fills.push(p.selection);
                    }
                    if let (Some((fv, _)), Some(a)) = (self.flash, flash_alpha)
                        && fv == vref
                        && a > 0.0
                    {
                        fills.push(p.accent_soft.gamma_multiply(a));
                    }
                    if fills.is_empty() {
                        continue;
                    }
                    for r in block.char_rects(*start..end) {
                        let r = r.translate(bo);
                        let shrink = ((r.height() - self.em * 1.34) / 2.0).max(0.0);
                        let r = Rect::from_min_max(
                            pos2(r.left() - 2.0, r.top() + shrink + 1.0),
                            pos2(r.right() + 2.0, r.bottom() - shrink + 1.0),
                        );
                        for f in &fills {
                            painter.rect_filled(r, CornerRadius::same(3), *f);
                        }
                        if is_selected {
                            painter.rect_stroke(
                                r,
                                CornerRadius::same(3),
                                Stroke::new(1.25, p.accent),
                                StrokeKind::Inside,
                            );
                        }
                    }
                }
                painter.galley(bo.to_pos2(), block.galley.clone(), p.ink);
                if let Some((g, gp)) = &block.gutter {
                    painter.galley((origin + *gp).to_pos2(), g.clone(), p.verse);
                }
                if let Some(ptr) = pointer.filter(|pt| br.expand2(vec2(8.0, 2.0)).contains(*pt)) {
                    let cur = block.galley.cursor_from_pos((ptr - bo).to_vec2());
                    if let Some(vi) = block.verse_at_char(cur.index.0) {
                        let num =
                            block.verses.iter().find(|(_, v, _)| *v == vi).map(|(_, _, n)| *n).unwrap_or(1);
                        hit = Some(VerseRef::new(ch.book(), ch.chapter(), num));
                    }
                }
            }
            // Margin markers (notes, bookmarks) at each verse's first line, right of the text.
            let right = geom.x0 + geom.total_w + self.em * 0.9;
            for a in l.anchors.iter().filter(|a| a.column == 0) {
                let vref = VerseRef::new(ch.book(), ch.chapter(), a.number);
                let top = y + a.top;
                if top > rect.bottom() || top + self.em * 2.0 < rect.top() {
                    continue;
                }
                if first_visible.is_none() && top + (a.bottom - a.top) > reading_line {
                    first_visible = Some((ch, a.number));
                }
                let mut x = right;
                let cy = top + self.em * env.typo.line_height * 0.5;
                if env.study.is_bookmarked(vref) {
                    let r = Rect::from_center_size(pos2(x + 6.0, cy), vec2(10.0, 14.0));
                    icons::bookmark(&painter, r, p.accent, true);
                    if pointer.is_some_and(|pt| r.expand(4.0).contains(pt)) {
                        marker_hit = Some(vref);
                    }
                    x += 18.0;
                }
                if env.study.note(vref).is_some() {
                    let r = Rect::from_center_size(pos2(x + 7.0, cy), vec2(13.0, 13.0));
                    icons::note(&painter, r, p.accent);
                    if pointer.is_some_and(|pt| r.expand(4.0).contains(pt)) {
                        marker_hit = Some(vref);
                    }
                }
            }
            if first_visible.is_none() && y + l.height > reading_line {
                first_visible = Some((ch, l.anchors.first().map_or(1, |a| a.number)));
            }
            if let Some(o) =
                env.columns.iter().find_map(|id| env.library.get(id).and_then(|e| e.online().cloned()))
                && !l.blocks.is_empty()
            {
                o.displayed(ch);
            }
            y += l.height;
            c = ch.next();
        }
        if let Some(fv) = first_visible {
            self.location = fv;
        }
        let last_visible = c.and_then(|n| n.prev()).unwrap_or(ChapterIdx::LAST);
        // While idle, typeset the neighbouring chapters (one per frame) so that scrolling or
        // turning the page into them never has to lay out text first.
        self.frames += 1;
        // Not during the first frames: nothing should delay the first paint.
        let idle = self.frames > 2 && dy == 0.0 && self.scroll_remaining == 0.0 && !resp.dragged();
        if idle {
            let wanted = [
                last_visible.next(),
                self.anchor.chapter.prev(),
                last_visible.next().and_then(ChapterIdx::next),
            ];
            if let Some(next) = wanted.into_iter().flatten().find(|c| !self.cache.contains_key(c)) {
                self.chapter_layout(ui, env, &geom, next);
                ui.ctx().request_repaint();
            }
        }

        // Interaction.
        if marker_hit.is_some() || hit.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if resp.clicked() {
            if let Some(v) = marker_hit {
                actions.push(Action::EditNote(v));
            } else if let Some(v) = hit {
                let shift = ui.input(|i| i.modifiers.shift);
                self.select(v, shift);
            } else {
                self.selection = None;
            }
        }
        if resp.double_clicked()
            && let Some(v) = hit
        {
            self.selection = Some((v, v));
            actions.push(Action::EditNote(v));
        }
        if resp.secondary_clicked()
            && let Some(v) = hit
            && !selected.contains(&v)
        {
            self.selection = Some((v, v));
        }
        let has_selection = self.selection.is_some();
        resp.context_menu(|ui| {
            if !has_selection {
                ui.label("Click a verse to select it");
                return;
            }
            context_menu(ui, env, actions);
        });

        self.paint_scrollbar(ui, &bar, &bar_resp, p, env);
    }

    /// Keeps the anchor inside its chapter, moving to neighbours as needed.
    fn normalize(&mut self, ui: &Ui, env: &Env<'_>, geom: &Geometry) {
        for _ in 0..64 {
            if self.anchor.offset < 0.0 {
                match self.anchor.chapter.prev() {
                    Some(prev) => {
                        let h = self.chapter_layout(ui, env, geom, prev).height;
                        self.anchor = Anchor { chapter: prev, offset: self.anchor.offset + h };
                    }
                    None => {
                        self.anchor.offset = 0.0;
                        self.scroll_remaining = 0.0;
                        break;
                    }
                }
                continue;
            }
            let h = self.chapter_layout(ui, env, geom, self.anchor.chapter).height;
            if self.anchor.offset >= h {
                match self.anchor.chapter.next() {
                    Some(next) => self.anchor = Anchor { chapter: next, offset: self.anchor.offset - h },
                    None => {
                        self.anchor.offset = (h - self.viewport_h * 0.5).max(0.0).min(self.anchor.offset);
                        self.scroll_remaining = 0.0;
                        break;
                    }
                }
                continue;
            }
            if self.anchor.chapter == ChapterIdx::LAST {
                let max = (h - self.viewport_h * 0.5).max(0.0);
                if self.anchor.offset > max {
                    self.anchor.offset = max;
                    self.scroll_remaining = 0.0;
                }
            }
            break;
        }
    }

    fn paint_scrollbar(
        &self,
        ui: &Ui,
        bar: &Rect,
        resp: &egui::Response,
        p: &crate::theme::Palette,
        env: &Env<'_>,
    ) {
        let painter = ui.painter();
        let active = resp.hovered() || resp.dragged();
        if active {
            painter.rect_filled(bar.expand(1.0), CornerRadius::same(6), p.fill.gamma_multiply(0.8));
        }
        let h = self.cache.get(&self.anchor.chapter).map_or(1.0, |c| c.layout.height.max(1.0));
        let frac =
            (self.anchor.chapter.0 as f32 + (self.anchor.offset / h).clamp(0.0, 1.0)) / CHAPTER_COUNT as f32;
        let thumb_h = 36.0;
        let ty = bar.top() + frac * (bar.height() - thumb_h);
        let thumb = Rect::from_min_size(pos2(bar.left() + 2.0, ty), vec2(bar.width() - 4.0, thumb_h));
        painter.rect_filled(
            thumb,
            CornerRadius::same(4),
            if active { p.ink_2 } else { p.ink_3.gamma_multiply(0.7) },
        );
        // Testament divider tick.
        let nt = bible_core::books::FIRST_NT_BOOK.first_chapter().0 as f32 / CHAPTER_COUNT as f32;
        let yy = bar.top() + nt * bar.height();
        painter.line_segment(
            [pos2(bar.left() + 1.0, yy), pos2(bar.right() - 1.0, yy)],
            Stroke::new(1.0, p.separator),
        );
        if let Some(pos) = resp.hover_pos() {
            let f = ((pos.y - bar.top()) / bar.height()).clamp(0.0, 0.9999) * CHAPTER_COUNT as f32;
            let c = ChapterIdx(f.floor() as u16);
            let label = env
                .columns
                .first()
                .and_then(|id| env.library.get(id))
                .map(|e| format!("{} {}", e.meta.ref_name(c.book()), c.chapter()))
                .unwrap_or_else(|| c.display());
            egui::Tooltip::always_open(
                ui.ctx().clone(),
                ui.layer_id(),
                Id::new("bar-tip"),
                egui::PopupAnchor::Pointer,
            )
            .show(|ui| {
                ui.label(label);
            });
        }
    }
}

fn context_menu(ui: &mut Ui, env: &Env<'_>, actions: &mut Vec<Action>) {
    let p = env.theme.palette();
    ui.set_min_width(190.0);
    ui.horizontal(|ui| {
        for c in HighlightColor::ALL {
            let (r, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
            ui.painter().circle_filled(r.center(), 8.0, p.swatch(c));
            if resp.hovered() {
                ui.painter().circle_stroke(r.center(), 10.0, Stroke::new(1.5, p.accent));
            }
            if resp.on_hover_text(format!("Highlight {}", c.name())).clicked() {
                actions.push(Action::Highlight(Some(c)));
                ui.close();
            }
        }
    });
    ui.separator();
    if ui.button("Remove highlight").clicked() {
        actions.push(Action::Highlight(None));
        ui.close();
    }
    if ui.button("Note…").clicked() {
        actions.push(Action::EditSelectedNote);
        ui.close();
    }
    if ui.button("Bookmark").clicked() {
        actions.push(Action::ToggleBookmark);
        ui.close();
    }
    if ui.button("Copy with reference").clicked() {
        actions.push(Action::Copy);
        ui.close();
    }
    if ui.button("Compare translations").clicked() {
        actions.push(Action::Compare);
        ui.close();
    }
}

/// Column placement for the current window width.
pub struct Geometry {
    pub x0: f32,
    pub col_w: f32,
    pub gutter: f32,
    pub total_w: f32,
}

impl Geometry {
    pub fn new(rect: Rect, typo: &Typography, columns: usize) -> Self {
        let n = columns.max(1) as f32;
        // At least enough room on the right for the note/bookmark markers.
        let margin = (rect.width() * 0.06).clamp(56.0, 96.0);
        let avail = (rect.width() - margin * 2.0).max(200.0);
        let gutter = if n > 1.0 { typo.size * 1.8 } else { 0.0 };
        let max_col = typo.size * if n > 1.0 { typo.measure * 0.82 } else { typo.measure };
        let col_w = ((avail - gutter * (n - 1.0)) / n).min(max_col).floor();
        let total_w = col_w * n + gutter * (n - 1.0);
        let x0 = (rect.center().x - total_w / 2.0).round();
        Self { x0, col_w, gutter, total_w }
    }
}
