//! An online translation that can't load the chapter being read (frame 33). In the reader's
//! place: why, Retry or a local translation instead, and the chapters kept for reading offline.
//! The app adds an error toast with Retry that stays until dismissed or the chapter loads.

use eframe::egui::text::LayoutJob;
use eframe::egui::{self, Align, Rect, ScrollArea, Sense, TextFormat, Ui, pos2, vec2};

use bible_core::ChapterIdx;
use bible_core::library::Library;

use crate::actions::Action;
use crate::icons;
use crate::pages::{self, Empty, Tone};
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
/// chapter is being fetched again. It scrolls when the window is too short for it.
pub fn show(
    ui: &mut Ui,
    p: &Palette,
    area: Rect,
    library: &Library,
    state: &Offline,
    retrying: bool,
    actions: &mut Vec<Action>,
) {
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(area));
    ScrollArea::vertical().id_salt("offline").auto_shrink(false).show(&mut child, |ui| {
        // The viewport, in the page's scrolling coordinates.
        let area = ui.max_rect();
        let bottom = page(ui, p, area, library, state, retrying, actions);
        // Room to scroll the card clear of the error toast: a 40 pt capsule 24 pt above the
        // window's edge, and a 24 pt gap.
        let clear = 24.0 + 40.0 + 24.0;
        ui.allocate_rect(Rect::from_min_max(area.min, pos2(area.right(), bottom + clear)), Sense::hover());
    });
}

/// The page's content in `area`; returns its bottom.
fn page(
    ui: &mut Ui,
    p: &Palette,
    area: Rect,
    library: &Library,
    state: &Offline,
    retrying: bool,
    actions: &mut Vec<Action>,
) -> f32 {
    let painter = ui.painter().clone();
    let abbrev = library.get(&state.id).map_or_else(|| state.id.clone(), |e| e.meta.abbrev.clone());
    let cx = area.center().x;
    let title = format!("{abbrev} can’t load {} right now", state.chapter.display());
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

    // Retry, or read a local translation instead.
    let instead = fallback(library).map(|(id, abbrev)| (id, format!("Read in {abbrev} instead")));
    let mut buttons =
        vec![Button::new(Kind::Secondary, if retrying { "Retrying…" } else { "Retry" }).enabled(!retrying)];
    if let Some((_, label)) = &instead {
        buttons.push(Button::new(Kind::Ghost, label));
    }
    let empty = Empty {
        glyph: icons::cloud_slash,
        tone: Tone::Danger,
        title: &title,
        body: &body,
        buttons: &buttons,
        hints: &[],
        width: 520.0,
    };

    // The state and the card of chapters kept offline, optically centred in the reader's place.
    let room = area.width() - 2.0 * pages::SIDE;
    let cached = library.cached_chapters(&state.id);
    let state_h = empty.height(&painter, room);
    let h = state_h + if cached.is_empty() { 0.0 } else { 24.0 + 84.0 };
    let top = pages::optical_top(area.y_range(), h, area.top() + pages::TOP);
    let responses = empty.show(ui, p, cx, top, room);
    if responses.first().is_some_and(|r| r.clicked()) {
        state.retry(library);
    }
    if let (Some(r), Some((id, _))) = (responses.get(1), &instead)
        && r.clicked()
    {
        actions.push(Action::SetTranslation(id.clone()));
    }
    let y = top + state_h + 24.0;

    // The chapters kept for reading offline, as many as fit on a line.
    if cached.is_empty() {
        return top + state_h;
    }
    let card_w = room.min(520.0);
    let card = Rect::from_min_size(pos2(cx - card_w / 2.0, y), vec2(card_w, 84.0));
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
    card.bottom()
}
