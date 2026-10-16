//! An online translation that can't load the chapter being read (frame 33). In the reader's
//! place: why, Retry or a local translation instead, and the chapters kept for reading offline.
//! The app adds an error toast with Retry that stays until dismissed or the chapter loads.

use eframe::egui::text::LayoutJob;
use eframe::egui::{Align, Align2, Color32, FontId, Rect, TextFormat, Ui, pos2, vec2};

use bible_core::ChapterIdx;
use bible_core::library::Library;

use crate::actions::Action;
use crate::icons;
use crate::theme::{self, Palette};
use crate::widgets::{self, Button, Kind};

/// A chapter of an online translation that failed to load.
pub struct Offline {
    pub id: String,
    pub chapter: ChapterIdx,
    pub message: String,
    /// The error toast was dismissed.
    pub dismissed: bool,
}

impl Offline {
    /// No connection (as opposed to, say, a rejected key).
    fn unreachable(&self) -> bool {
        bible_core::online::is_offline(&self.message)
    }

    fn host(&self, library: &Library) -> &'static str {
        library.get(&self.id).and_then(|e| e.online()).map_or("the server", |o| o.config.provider.host())
    }

    /// The toast's message.
    pub fn toast(&self, library: &Library) -> String {
        if self.unreachable() {
            format!("Couldn’t reach {}", self.host(library))
        } else {
            format!("Couldn’t load {}", self.chapter.display())
        }
    }

    /// Asks for the chapter again.
    pub fn retry(&self, library: &Library) {
        if let Some(o) = library.get(&self.id).and_then(|e| e.online()) {
            o.retry(self.chapter);
        }
    }
}

fn css_shift(ui: &Ui, format: &TextFormat) -> f32 {
    ui.ctx().fonts_mut(|f| crate::reader::layout::css_shift(f, format))
}

/// A local translation to read instead: the KJV, else the first one there is.
fn fallback(library: &Library) -> Option<(String, String)> {
    let local = |e: &&std::sync::Arc<bible_core::library::Entry>| e.is_local();
    let entries = library.entries();
    entries
        .iter()
        .filter(local)
        .find(|e| e.meta.id == "kjv")
        .or_else(|| entries.iter().find(local))
        .map(|e| (e.meta.id.clone(), e.meta.abbrev.clone()))
}

/// Draws the page in `area` (the reader's place under the toolbar). `retrying` while the
/// chapter is being fetched again.
pub fn show(
    ui: &mut Ui,
    p: &Palette,
    area: Rect,
    library: &Library,
    state: &Offline,
    retrying: bool,
    actions: &mut Vec<Action>,
) {
    let painter = ui.painter().clone();
    let abbrev = library.get(&state.id).map_or_else(|| state.id.clone(), |e| e.meta.abbrev.clone());
    let cx = area.center().x;
    let mut y = area.top() + 96.0;

    // Glyph in a danger-tinted circle.
    let c = pos2(cx, y + 28.0);
    let [r, g, b, _] = p.danger.to_array();
    painter.circle_filled(c, 28.0, Color32::from_rgba_unmultiplied(r, g, b, 31));
    icons::cloud_slash(&painter, Rect::from_center_size(c, vec2(28.0, 28.0)), p.danger);
    y += 56.0 + 12.0;

    let title = format!("{abbrev} can’t load {} right now", state.chapter.display());
    painter.text(
        pos2(cx, y + 14.0),
        Align2::CENTER_CENTER,
        title,
        FontId::new(20.0, theme::family(theme::SERIF)),
        p.ink,
    );
    y += 28.0 + 4.0;
    let body = if state.unreachable() {
        format!(
            "Chapters from {} need a connection. Your highlights and notes on this chapter are safe \
             and will reappear.",
            state.host(library)
        )
    } else {
        // "ESV: the API key was rejected (HTTP 401)" → "The API key was rejected (HTTP 401)."
        let reason = state.message.split_once(": ").map_or(state.message.as_str(), |(_, r)| r);
        let mut chars = reason.chars();
        let reason: String =
            chars.next().map(|f| f.to_uppercase().chain(chars).collect()).unwrap_or_default();
        format!("{}. Your highlights and notes on this chapter are safe.", reason.trim_end_matches('.'))
    };
    let format = TextFormat {
        font_id: theme::ui_font(13.0),
        color: p.ink_2,
        line_height: Some(20.0),
        ..Default::default()
    };
    let shift = css_shift(ui, &format);
    let mut job = LayoutJob::single_section(body, format);
    job.wrap.max_width = 520.0;
    job.halign = Align::Center;
    let galley = painter.layout_job(job);
    let body_h = galley.size().y;
    painter.galley(pos2(cx, y + shift), galley, p.ink_2);
    y += body_h + 12.0 + 4.0;

    // Retry, or read a local translation instead.
    let retry = Button::new(Kind::Secondary, if retrying { "Retrying…" } else { "Retry" }).enabled(!retrying);
    let instead = fallback(library).map(|(id, abbrev)| (id, format!("Read in {abbrev} instead")));
    let other = instead.as_ref().map(|(_, label)| Button::new(Kind::Ghost, label));
    let wa = retry.width(&painter);
    let wb = other.as_ref().map_or(0.0, |b| 8.0 + b.width(&painter));
    let left = cx - (wa + wb) / 2.0;
    if retry.show_left(ui, p, left, y + 16.0, ui.id().with("offline-retry")).1.clicked() {
        state.retry(library);
    }
    if let (Some(b), Some((id, _))) = (&other, &instead)
        && b.show_left(ui, p, left + wa + 8.0, y + 16.0, ui.id().with("offline-instead")).1.clicked()
    {
        actions.push(Action::SetTranslation(id.clone()));
    }
    y += 32.0 + 24.0;

    // The chapters kept for reading offline, as many as fit on a line.
    let cached = library.cached_chapters(&state.id);
    if cached.is_empty() {
        return;
    }
    let card = Rect::from_min_size(pos2(cx - 260.0, y), vec2(520.0, 84.0));
    painter.rect_filled(card, 14.0, p.fill);
    let font = theme::ui_semibold(11.0);
    let format = TextFormat {
        font_id: font.clone(),
        color: p.ink_3,
        extra_letter_spacing: 0.12 * font.size,
        line_height: Some(16.0),
        ..Default::default()
    };
    let shift = css_shift(ui, &format);
    let mut job =
        LayoutJob::single_section(format!("AVAILABLE OFFLINE IN {}", abbrev.to_uppercase()), format);
    job.halign = Align::Center;
    painter.galley(pos2(cx, card.top() + 16.0 + shift), painter.layout_job(job), p.ink_3);
    let pills: Vec<(ChapterIdx, String, f32)> = cached
        .into_iter()
        .map(|c| {
            let label = c.display();
            let w = widgets::pill_tab_width(&painter, &label, None, false, false);
            (c, label, w)
        })
        .scan(0.0, |used, (c, label, w)| {
            *used += w + 4.0;
            (*used - 4.0 <= card.width() - 32.0).then_some((c, label, w))
        })
        .collect();
    let total = pills.iter().map(|(_, _, w)| w + 4.0).sum::<f32>() - 4.0;
    let mut x = cx - total / 2.0;
    for (c, label, w) in pills {
        let r = Rect::from_min_size(pos2(x, card.top() + 40.0), vec2(w, 28.0));
        if widgets::pill_tab(ui, p, r, &label, None, None, false, ui.id().with(("offline-pill", c.0)))
            .clicked()
        {
            actions.push(Action::Jump { chapter: c, verse: None, flash: false });
        }
        x += w + 4.0;
    }
}
