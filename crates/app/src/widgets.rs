//! Small custom widgets shared across the UI, drawn to the design's component sheets
//! (`ds-06` title bar, `ds-07` buttons, `ds-10` search field in the Paper file).

use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, Galley, Painter, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, pos2,
    vec2,
};

use crate::theme::{self, Palette, metrics, text};

/// The fill behind a borderless button or row: `fill` on hover, `fill_strong` while pressed.
pub fn hover_fill(ui: &Ui, resp: &Response, p: &Palette, on: bool) -> Option<Color32> {
    if !ui.is_enabled() {
        return None;
    }
    if resp.is_pointer_button_down_on() {
        return Some(p.fill_strong);
    }
    let t = ui.ctx().animate_bool_with_time(resp.id.with("hover"), resp.hovered() || on, 0.1);
    (t > 0.0).then(|| p.fill.gamma_multiply(t))
}

/// A glass capsule behind toolbar controls.
pub fn capsule(painter: &Painter, rect: Rect, p: &Palette) {
    let r = rect.height() / 2.0;
    p.shadows.control.paint(painter, rect, r);
    painter.rect_filled(rect, r, p.glass);
}

/// The 1 × 16 hairline between two buttons of a capsule.
pub fn capsule_divider(painter: &Painter, x: f32, center_y: f32, p: &Palette) {
    let r = Rect::from_min_max(pos2(x, center_y - 8.0), pos2(x + 1.0, center_y + 8.0));
    painter.rect_filled(r, 0.0, p.separator);
}

/// A toolbar button at `rect` (inside a capsule, or on its own): a 20 pt icon in ink, a pill
/// on hover and press, accent on accent-soft when `on`, tertiary ink when disabled.
pub fn toolbar_button(
    ui: &mut Ui,
    rect: Rect,
    p: &Palette,
    tooltip: &str,
    enabled: bool,
    on: bool,
    paint: fn(&Painter, Rect, Color32),
) -> Response {
    let id = ui.id().with(("toolbar-button", tooltip));
    let resp = ui.interact(rect, id, if enabled { Sense::click() } else { Sense::hover() });
    let pill = rect.height() / 2.0;
    if on {
        ui.painter().rect_filled(rect, pill, p.accent_soft);
    } else if enabled && let Some(fill) = hover_fill(ui, &resp, p, false) {
        ui.painter().rect_filled(rect, pill, fill);
    }
    let color = if !enabled {
        p.ink_3
    } else if on {
        p.accent
    } else {
        p.ink
    };
    paint(ui.painter(), Rect::from_center_size(rect.center(), Vec2::splat(metrics::ICON)), color);
    resp.on_hover_text(tooltip)
}

/// A round icon button (28 pt) with a 16 pt glyph in secondary ink, for panels and popovers.
pub fn icon_button(
    ui: &mut Ui,
    p: &Palette,
    tooltip: &str,
    on: bool,
    paint: fn(&Painter, Rect, Color32),
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
    if on {
        ui.painter().circle_filled(rect.center(), 14.0, p.accent_soft);
    } else if let Some(fill) = hover_fill(ui, &resp, p, false) {
        ui.painter().circle_filled(rect.center(), 14.0, fill);
    }
    let color = if !ui.is_enabled() {
        p.ink_3
    } else if on {
        p.accent
    } else {
        p.ink_2
    };
    paint(ui.painter(), Rect::from_center_size(rect.center(), Vec2::splat(metrics::ICON_SM)), color);
    resp.on_hover_text(tooltip)
}

/// A capsule of icon buttons separated by hairlines (`dividers`) or not, at `left`.
/// Returns the capsule's rectangle and each button's response.
pub struct CapsuleButton<'a> {
    pub tooltip: &'a str,
    pub enabled: bool,
    pub on: bool,
    pub icon: fn(&Painter, Rect, Color32),
}

pub fn button_capsule(
    ui: &mut Ui,
    p: &Palette,
    left: f32,
    center_y: f32,
    width: f32,
    dividers: bool,
    buttons: &[CapsuleButton<'_>],
) -> (Rect, Vec<Response>) {
    let h = metrics::CAPSULE_HEIGHT;
    let gaps = if dividers { buttons.len().saturating_sub(1) as f32 } else { 0.0 };
    let rect =
        Rect::from_min_size(pos2(left, center_y - h / 2.0), vec2(width * buttons.len() as f32 + gaps, h));
    capsule(ui.painter(), rect, p);
    let mut x = left;
    let mut out = Vec::new();
    for (i, b) in buttons.iter().enumerate() {
        if i > 0 && dividers {
            capsule_divider(ui.painter(), x, center_y, p);
            x += 1.0;
        }
        let r = Rect::from_min_size(pos2(x, rect.top()), vec2(width, h));
        out.push(toolbar_button(ui, r, p, b.tooltip, b.enabled, b.on, b.icon));
        x += width;
    }
    (rect, out)
}

/// A picker label inside a capsule: 13 pt semibold text, then a 12 pt chevron.
pub struct Picker {
    galley: std::sync::Arc<egui::Galley>,
    pad_left: f32,
    pad_right: f32,
}

const PICKER_GAP: f32 = 4.0;
const PICKER_CHEVRON: f32 = 12.0;

impl Picker {
    pub fn new(ui: &Ui, text: &str, pad_left: f32, pad_right: f32) -> Self {
        let font = text::UI_STRONG.font();
        let galley = ui.painter().layout_no_wrap(text.to_string(), font, Color32::PLACEHOLDER);
        Self { galley, pad_left, pad_right }
    }

    pub fn width(&self) -> f32 {
        (self.pad_left + self.galley.size().x + PICKER_GAP + PICKER_CHEVRON + self.pad_right).round()
    }

    /// Paints the picker at `rect` (its full height in the capsule) and makes it clickable.
    pub fn show(self, ui: &mut Ui, rect: Rect, p: &Palette, id: &str, tooltip: &str) -> Response {
        let resp = ui.interact(rect, ui.id().with(("picker", id)), Sense::click());
        if let Some(fill) = hover_fill(ui, &resp, p, false) {
            ui.painter().rect_filled(rect, rect.height() / 2.0, fill);
        }
        let size = self.galley.size();
        let text_pos = pos2(rect.left() + self.pad_left, (rect.center().y - size.y / 2.0).round());
        ui.painter().galley(text_pos, self.galley, p.ink);
        let chevron = Rect::from_center_size(
            pos2(text_pos.x + size.x + PICKER_GAP + PICKER_CHEVRON / 2.0, rect.center().y),
            Vec2::splat(PICKER_CHEVRON),
        );
        crate::icons::chevron_down(ui.painter(), chevron, p.ink_2);
        resp.on_hover_text(tooltip)
    }
}

/// The toolbar search field: a glass capsule with a magnifier and a text field. Focused, it
/// takes the field colour with an accent outline and focus ring; with text, a clear button.
/// Returns the text edit's response and whether the clear button was clicked.
pub fn search_field(
    ui: &mut Ui,
    rect: Rect,
    p: &Palette,
    query: &mut String,
    hint: &str,
    id: egui::Id,
) -> (Response, bool) {
    let r = rect.height() / 2.0;
    let focused = ui.memory(|m| m.has_focus(id));
    if focused {
        ui.painter().rect_filled(rect.expand(3.0), r + 3.0, p.focus);
        ui.painter().rect_filled(rect, r, p.field);
        ui.painter().rect_stroke(rect, r, Stroke::new(1.0, p.accent), StrokeKind::Inside);
    } else {
        capsule(ui.painter(), rect, p);
    }
    let glass = Rect::from_min_size(pos2(rect.left() + 10.0, rect.center().y - 8.0), vec2(16.0, 16.0));
    crate::icons::search(ui.painter(), glass, p.ink_2);
    let clear_w = if query.is_empty() { 0.0 } else { 22.0 };
    let edit_rect = Rect::from_min_max(
        pos2(glass.right() + 6.0, rect.top()),
        pos2(rect.right() - 12.0 - clear_w, rect.bottom()),
    );
    let font = text::UI.font();
    let resp = ui.put(
        edit_rect,
        egui::TextEdit::singleline(query)
            .id(id)
            .hint_text(egui::RichText::new(hint).color(p.ink_3).font(font.clone()))
            .font(font)
            .text_color(p.ink)
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::ZERO)
            .vertical_align(egui::Align::Center)
            .desired_width(edit_rect.width()),
    );
    let mut cleared = false;
    if !query.is_empty() {
        let c = Rect::from_center_size(pos2(rect.right() - 10.0 - 8.0, rect.center().y), vec2(16.0, 16.0));
        let cr = ui.interact(c.expand(2.0), id.with("clear"), Sense::click());
        crate::icons::clear(ui.painter(), c, p.ink_3, p.paper);
        cleared = cr.clicked();
    }
    (resp, cleared)
}

/// The disclosure icon for egui combo boxes (instead of egui's triangle).
pub fn combo_chevron(ui: &Ui, rect: Rect, visuals: &egui::style::WidgetVisuals, _open: bool) {
    let r = Rect::from_center_size(rect.center(), Vec2::splat(12.0));
    crate::icons::chevron_down(ui.painter(), r, visuals.fg_stroke.color.gamma_multiply(0.7));
}

/// A segmented control; returns the newly selected index if it changed.
pub fn segmented(ui: &mut Ui, p: &Palette, labels: &[&str], selected: usize) -> Option<usize> {
    let mut out = None;
    let h = metrics::CAPSULE_HEIGHT;
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same((h / 2.0) as u8), p.fill);
    let inner = rect.shrink(2.0);
    let seg_w = inner.width() / labels.len() as f32;
    for (i, label) in labels.iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(inner.left() + seg_w * i as f32, inner.top()),
            vec2(seg_w, inner.height()),
        );
        let resp = ui.interact(r, ui.id().with(("seg", label)), Sense::click());
        let radius = r.height() / 2.0;
        if i == selected {
            p.shadows.knob.paint(ui.painter(), r, radius);
            ui.painter().rect_filled(r, radius, p.raised);
        } else if let Some(fill) = hover_fill(ui, &resp, p, false) {
            ui.painter().rect_filled(r, radius, fill);
        }
        let (font, color) = if i == selected {
            (text::UI_STRONG.font(), p.ink)
        } else {
            (theme::ui_medium(text::UI.size), p.ink_2)
        };
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, *label, font, color);
        if resp.clicked() && i != selected {
            out = Some(i);
        }
    }
    out
}

/// A section caption, in the style of a macOS source list header.
pub fn caption(ui: &mut Ui, label: &str) {
    ui.label(egui::RichText::new(label).font(text::CAPTION.font()).color(ui.visuals().weak_text_color()));
}

/// A keycap (`ds-15`): 20 pt high, radius 5, `fill` with a hairline and a heavier bottom edge,
/// 11 pt medium secondary ink. Drawn with its left edge at `left`, centred on `center_y`;
/// returns its rectangle.
pub fn keycap(painter: &Painter, p: &Palette, left: f32, center_y: f32, label: &str) -> Rect {
    let galley = painter.layout_no_wrap(label.to_string(), theme::ui_medium(11.0), p.ink_2);
    let w = (galley.size().x + 10.0).max(20.0).round();
    let r = Rect::from_min_size(pos2(left, center_y - 10.0), vec2(w, 20.0));
    painter.rect_filled(r, 5.0, p.fill);
    painter.rect_stroke(r, 5.0, Stroke::new(0.5, p.border), StrokeKind::Outside);
    painter.hline(r.x_range().shrink(2.0), r.bottom() - 0.25, Stroke::new(0.5, p.border));
    let size = galley.size();
    painter.galley(pos2(r.center().x - size.x / 2.0, r.center().y - size.y / 2.0), galley, p.ink_2);
    r
}

/// Width of [`keycap`] for `label`.
pub fn keycap_width(painter: &Painter, label: &str) -> f32 {
    let w = painter.layout_no_wrap(label.to_string(), theme::ui_medium(11.0), Color32::PLACEHOLDER).size().x;
    (w + 10.0).max(20.0).round()
}

/// A slider (`ds-14`): a 4 pt track in `fill_strong` with the active part in the accent, a 20 pt
/// white knob with the knob shadow (a halo on hover, the focus ring while dragged), and optional
/// ticks under the track. `rect` is the slider's 24 pt high box; returns whether the value changed.
#[allow(clippy::too_many_arguments)]
pub fn slider(
    ui: &mut Ui,
    p: &Palette,
    rect: Rect,
    id: egui::Id,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: f32,
    ticks: usize,
) -> bool {
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let (lo, hi) = (*range.start(), *range.end());
    let track = Rect::from_min_max(
        pos2(rect.left() + 10.0, rect.center().y - 2.0),
        pos2(rect.right() - 10.0, rect.center().y + 2.0),
    );
    let mut changed = false;
    if let Some(pos) = resp.interact_pointer_pos()
        && (resp.dragged() || resp.clicked())
    {
        let t = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
        let v = ((lo + t * (hi - lo)) / step).round() * step;
        if (v - *value).abs() > f32::EPSILON {
            *value = v.clamp(lo, hi);
            changed = true;
        }
    }
    let t = ((*value - lo) / (hi - lo)).clamp(0.0, 1.0);
    let x = track.left() + t * track.width();
    let painter = ui.painter();
    painter.rect_filled(track, 2.0, p.fill_strong);
    painter.rect_filled(Rect::from_min_max(track.min, pos2(x, track.bottom())), 2.0, p.accent);
    if ticks > 1 {
        for k in 0..ticks {
            let tx = track.left() + track.width() * k as f32 / (ticks - 1) as f32;
            painter.rect_filled(
                Rect::from_min_size(pos2(tx - 1.0, rect.top() + 17.0), vec2(2.0, 4.0)),
                1.0,
                p.ink_3,
            );
        }
    }
    let knob = Rect::from_center_size(pos2(x, rect.center().y), vec2(20.0, 20.0));
    if resp.dragged() || resp.has_focus() {
        painter.circle_filled(knob.center(), 10.0 + 3.0, p.focus);
    } else if resp.hovered() {
        painter.circle_filled(knob.center(), 10.0 + 4.0, p.fill_strong);
    }
    p.shadows.knob.paint(painter, knob, 10.0);
    painter.circle_filled(knob.center(), 10.0, Color32::WHITE);
    changed
}

/// A switch (`ds-14`): 40 × 24, accent when on, `fill_strong` when off, a 20 pt white knob.
/// Draws at `rect`'s right edge, centred vertically; returns the response.
pub fn toggle(ui: &mut Ui, p: &Palette, right_center: egui::Pos2, id: egui::Id, on: &mut bool) -> Response {
    let r = Rect::from_min_size(pos2(right_center.x - 40.0, right_center.y - 12.0), vec2(40.0, 24.0));
    let resp = ui.interact(r, id, Sense::click());
    if resp.clicked() {
        *on = !*on;
    }
    let t = ui.ctx().animate_bool_with_time(id, *on, 0.12);
    let track = if *on { p.accent } else { p.fill_strong };
    ui.painter().rect_filled(r, 12.0, track);
    if resp.hovered() {
        ui.painter().rect_filled(r, 12.0, Color32::from_black_alpha(12));
    }
    let knob = Rect::from_center_size(pos2(r.left() + 12.0 + t * 16.0, r.center().y), vec2(20.0, 20.0));
    p.shadows.knob.paint(ui.painter(), knob, 10.0);
    ui.painter().circle_filled(knob.center(), 10.0, Color32::WHITE);
    resp
}

/// A secondary button (`ds-07`): a 32 pt capsule in `raised` with the control shadow, 13 pt medium.
pub fn secondary_button(ui: &mut Ui, p: &Palette, rect: Rect, label: &str, id: egui::Id) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    let r = rect.height() / 2.0;
    p.shadows.control.paint(ui.painter(), rect, r);
    ui.painter().rect_filled(rect, r, p.raised);
    if let Some(fill) = hover_fill(ui, &resp, p, false) {
        ui.painter().rect_filled(rect, r, fill);
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        theme::ui_medium(text::UI.size),
        p.ink,
    );
    resp
}

/// Width of a [`secondary_button`] for `label` (14 pt either side).
pub fn secondary_button_width(ui: &Ui, label: &str) -> f32 {
    ui.painter().layout_no_wrap(label.into(), theme::ui_medium(text::UI.size), Color32::PLACEHOLDER).size().x
        + 28.0
}

/// A pop-up button (`ds-14` select): the field colour, a hairline and soft drop, an optional
/// leading glyph, the value and up-down chevrons.
pub fn select_field(
    ui: &mut Ui,
    p: &Palette,
    r: Rect,
    value: &str,
    icon: Option<fn(&Painter, Rect, Color32)>,
    id: egui::Id,
) -> Response {
    let resp = ui.interact(r, id, Sense::click());
    let painter = ui.painter();
    painter.add(
        eframe::epaint::RectShape::filled(r.translate(vec2(0.0, 1.0)), 8.0, Color32::from_black_alpha(15))
            .with_blur_width(2.0),
    );
    painter.rect_filled(r, 8.0, if resp.hovered() { p.raised } else { p.field });
    painter.rect_stroke(r, 8.0, Stroke::new(0.5, p.border), StrokeKind::Outside);
    let cy = r.center().y;
    let mut x = r.left() + 12.0;
    if let Some(icon) = icon {
        crate::icons::heavy(|| {
            icon(painter, Rect::from_center_size(pos2(x + 8.0, cy), vec2(16.0, 16.0)), p.ink_2)
        });
        x += 16.0 + 8.0;
    }
    painter.text(pos2(x, cy), egui::Align2::LEFT_CENTER, value, text::UI.font(), p.ink);
    crate::icons::heavy(|| {
        crate::icons::chevrons_up_down(
            painter,
            Rect::from_center_size(pos2(r.right() - 8.0 - 8.0, cy), vec2(16.0, 16.0)),
            p.ink_2,
        )
    });
    resp
}

/// A segmented control sized to its labels (`ds-14`), right-aligned at `right_center`.
/// Returns the newly selected index if it changed.
pub fn segmented_fit(
    ui: &mut Ui,
    p: &Palette,
    right_center: egui::Pos2,
    id: egui::Id,
    labels: &[&str],
    selected: usize,
) -> Option<usize> {
    let font = text::UI_STRONG.font();
    let widths: Vec<f32> = labels
        .iter()
        .map(|l| {
            ui.painter().layout_no_wrap(l.to_string(), font.clone(), Color32::PLACEHOLDER).size().x + 26.0
        })
        .collect();
    let total: f32 = widths.iter().sum::<f32>() + 4.0;
    let track = Rect::from_min_size(pos2(right_center.x - total, right_center.y - 16.0), vec2(total, 32.0));
    ui.painter().rect_filled(track, 16.0, p.fill);
    let mut x = track.left() + 2.0;
    let mut out = None;
    for (i, (label, w)) in labels.iter().zip(widths).enumerate() {
        let r = Rect::from_min_size(pos2(x, track.top() + 2.0), vec2(w, 28.0));
        let resp = ui.interact(r, id.with(i), Sense::click());
        if i == selected {
            p.shadows.knob.paint(ui.painter(), r, 14.0);
            ui.painter().rect_filled(r, 14.0, p.raised);
        } else if let Some(fill) = hover_fill(ui, &resp, p, false) {
            ui.painter().rect_filled(r, 14.0, fill);
        }
        let (f, c) =
            if i == selected { (font.clone(), p.ink) } else { (theme::ui_medium(text::UI.size), p.ink_2) };
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, *label, f, c);
        if resp.clicked() && i != selected {
            out = Some(i);
        }
        x += w;
    }
    out
}

/// A checkbox (`ds-14`): 16 pt, radius 4; off, the field colour with a hairline; on, the accent
/// with a check. The label follows 8 pt after.
pub fn checkbox(ui: &mut Ui, p: &Palette, b: Rect, on: bool, label: &str) -> Response {
    let g = ui.painter().layout_no_wrap(label.to_string(), text::UI.font(), p.ink);
    let hit = Rect::from_min_size(b.min, vec2(16.0 + 8.0 + g.size().x, 16.0)).expand(4.0);
    let resp = ui.interact(hit, ui.id().with(("check", label)), Sense::click());
    let painter = ui.painter();
    if on {
        painter.rect_filled(b, 4.0, p.accent);
        let s = Stroke::new(1.75, p.on_accent);
        painter.add(egui::Shape::line(
            vec![b.min + vec2(4.0, 8.25), b.min + vec2(7.0, 11.0), b.min + vec2(12.0, 5.25)],
            s,
        ));
    } else {
        painter.rect_filled(b, 4.0, p.field);
        let border = if resp.hovered() { p.ink_3 } else { p.border };
        painter.rect_stroke(b, 4.0, Stroke::new(1.0, border), StrokeKind::Inside);
    }
    let size = g.size();
    painter.galley(pos2(b.right() + 8.0, b.center().y - size.y / 2.0), g, p.ink);
    resp
}

/// A radio button (`ds-14`): 16 pt; off, the field colour with a hairline; on, a 5 pt accent ring
/// around a `raised` centre.
pub fn radio(painter: &Painter, p: &Palette, center: egui::Pos2, on: bool, hovered: bool) {
    if on {
        painter.circle_filled(center, 8.0, p.accent);
        painter.circle_filled(center, 3.0, p.raised);
    } else {
        painter.circle_filled(center, 8.0, p.field);
        let border = if hovered { p.ink_3 } else { p.border };
        painter.circle_stroke(center, 7.5, Stroke::new(1.0, border));
    }
}

/// A primary button (`ds-07`): a 32 pt accent capsule, 13 pt semibold on-accent, with an optional
/// trailing 16 pt glyph 6 pt after the label.
pub fn primary_button(
    ui: &mut Ui,
    p: &Palette,
    rect: Rect,
    label: &str,
    trailing: Option<fn(&Painter, Rect, Color32)>,
    id: egui::Id,
) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    let r = rect.height() / 2.0;
    let painter = ui.painter();
    painter.rect_filled(rect, r, p.accent);
    if resp.is_pointer_button_down_on() {
        painter.rect_filled(rect, r, Color32::from_black_alpha(40));
    } else if resp.hovered() {
        painter.rect_filled(rect, r, Color32::from_black_alpha(20));
    }
    let galley = painter.layout_no_wrap(label.into(), theme::ui_semibold(text::UI.size), p.on_accent);
    let x = rect.left() + 14.0;
    painter.galley(pos2(x, rect.center().y - galley.size().y / 2.0), galley.clone(), p.on_accent);
    if let Some(icon) = trailing {
        let r = Rect::from_min_size(pos2(x + galley.size().x + 6.0, rect.center().y - 8.0), vec2(16.0, 16.0));
        crate::icons::heavy(|| icon(painter, r, p.on_accent));
    }
    resp
}

/// Width of a [`primary_button`]: 14 pt either side, plus the trailing glyph.
pub fn primary_button_width(ui: &Ui, label: &str, trailing: bool) -> f32 {
    let w = ui
        .painter()
        .layout_no_wrap(label.into(), theme::ui_semibold(text::UI.size), Color32::PLACEHOLDER)
        .size()
        .x;
    w + 28.0 + if trailing { 6.0 + 16.0 } else { 0.0 }
}

/// A plain button (`ds-07`): a 32 pt capsule in `fill`, 13 pt medium ink; `fill_strong` on hover.
pub fn plain_button(ui: &mut Ui, p: &Palette, rect: Rect, label: &str, id: egui::Id) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    let r = rect.height() / 2.0;
    let fill = if resp.hovered() || resp.is_pointer_button_down_on() { p.fill_strong } else { p.fill };
    ui.painter().rect_filled(rect, r, fill);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        theme::ui_medium(text::UI.size),
        p.ink,
    );
    resp
}

/// Width of a [`plain_button`] for `label`.
pub fn plain_button_width(ui: &Ui, label: &str) -> f32 {
    secondary_button_width(ui, label)
}

/// One line of text in `font` with `tracking` (in ems) and no wrapping.
pub fn line(painter: &Painter, text: &str, font: egui::FontId, color: Color32, tracking: f32) -> Arc<Galley> {
    let size = font.size;
    let mut job = egui::text::LayoutJob::single_section(
        text.into(),
        egui::TextFormat {
            font_id: font,
            color,
            extra_letter_spacing: tracking * size,
            ..Default::default()
        },
    );
    job.wrap.max_width = f32::INFINITY;
    painter.layout_job(job)
}

/// Where CSS puts the baseline of `size` pt text in a line box `lh` tall whose top is `top`
/// (Literata when `reading`, else Inter).
pub fn css_baseline(top: f32, lh: f32, size: f32, reading: bool) -> f32 {
    use crate::reader::layout::{INTER_BASELINE, LITERATA_BASELINE};
    top + lh / 2.0 + size * if reading { LITERATA_BASELINE } else { INTER_BASELINE }
}

/// The baseline of a galley's first row, from its top.
pub fn galley_baseline(g: &Galley) -> f32 {
    g.rows.first().and_then(|r| r.glyphs.first()).map_or(g.size().y * 0.8, |gl| gl.pos.y)
}

/// Paints `galley` with its left edge at `x` and its first baseline at `baseline`.
pub fn paint_on_baseline(painter: &Painter, x: f32, baseline: f32, galley: Arc<Galley>, color: Color32) {
    let y = baseline - galley_baseline(&galley);
    painter.galley(pos2(x, y), galley, color);
}
