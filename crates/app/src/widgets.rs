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
    resp.on_hover_text(crate::keymap::shown(tooltip).as_ref())
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
    resp.on_hover_text(crate::keymap::shown(tooltip).as_ref())
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
        resp.on_hover_text(crate::keymap::shown(tooltip).as_ref())
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
/// returns its rectangle. `label` is written the macOS way ("⌘"; "Ctrl" elsewhere).
pub fn keycap(painter: &Painter, p: &Palette, left: f32, center_y: f32, label: &str) -> Rect {
    let label = crate::keymap::shown(label);
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
    let label = crate::keymap::shown(label);
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
    // In dark, an off switch's knob is grey rather than white.
    let dark = p.paper.r() < 64;
    let off_knob = if dark { Color32::from_rgb(0xC9, 0xC9, 0xCE) } else { Color32::WHITE };
    ui.painter().circle_filled(knob.center(), 10.0, Color32::WHITE.lerp_to_gamma(off_knob, 1.0 - t));
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

pub type IconFn = fn(&Painter, Rect, Color32);

/// Button kinds (`ds-07`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Accent with on-accent text, semibold.
    Primary,
    /// `raised` with the control shadow.
    Secondary,
    /// `fill`.
    Plain,
    /// Accent text, no fill.
    Ghost,
    /// `danger` with white text.
    Destructive,
}

/// A capsule button (`ds-07`): sm 24 (12 pt, 10 pt sides), md 32 (13 pt, 14), lg 40 (13 pt, 20),
/// with an optional 16 pt glyph 6 pt from the label.
#[derive(Clone, Copy)]
pub struct Button<'a> {
    pub kind: Kind,
    pub label: &'a str,
    pub height: f32,
    pub icon: Option<IconFn>,
    /// The glyph after the label rather than before it.
    pub trailing: bool,
    pub enabled: bool,
}

impl<'a> Button<'a> {
    pub fn new(kind: Kind, label: &'a str) -> Self {
        Self { kind, label, height: 32.0, icon: None, trailing: false, enabled: true }
    }

    pub fn small(self) -> Self {
        Self { height: 24.0, ..self }
    }

    pub fn large(self) -> Self {
        Self { height: 40.0, ..self }
    }

    pub fn icon(self, icon: IconFn) -> Self {
        Self { icon: Some(icon), ..self }
    }

    pub fn trailing_icon(self, icon: IconFn) -> Self {
        Self { icon: Some(icon), trailing: true, ..self }
    }

    pub fn enabled(self, enabled: bool) -> Self {
        Self { enabled, ..self }
    }

    fn font(&self) -> egui::FontId {
        let size = if self.height < 32.0 { 12.0 } else { text::UI.size };
        if matches!(self.kind, Kind::Primary | Kind::Destructive) {
            theme::ui_semibold(size)
        } else {
            theme::ui_medium(size)
        }
    }

    fn padding(&self) -> f32 {
        if self.height < 32.0 {
            10.0
        } else if self.height > 32.0 {
            20.0
        } else {
            14.0
        }
    }

    /// How far in from its rectangle the button shows at rest. A ghost button shows only its
    /// label until hovered.
    pub fn rest_inset(&self) -> f32 {
        if self.kind == Kind::Ghost { self.padding() } else { 0.0 }
    }

    pub fn width(&self, painter: &Painter) -> f32 {
        let w = painter.layout_no_wrap(self.label.into(), self.font(), Color32::PLACEHOLDER).size().x;
        let icon =
            if self.icon.is_some() { 16.0 + if self.label.is_empty() { 0.0 } else { 6.0 } } else { 0.0 };
        (w + icon + 2.0 * self.padding()).round()
    }

    /// Draws the button in `rect`.
    pub fn show(&self, ui: &mut Ui, p: &Palette, rect: Rect, id: egui::Id) -> Response {
        let sense = if self.enabled { Sense::click() } else { Sense::hover() };
        let resp = ui.interact(rect, id, sense);
        let mut painter = ui.painter().clone();
        if !self.enabled {
            painter.set_opacity(0.4);
        }
        let r = rect.height() / 2.0;
        let pressed = self.enabled && resp.is_pointer_button_down_on();
        let hovered = self.enabled && resp.hovered();
        let (fill, ink) = match self.kind {
            Kind::Primary => (Some(p.accent), p.on_accent),
            Kind::Secondary => (Some(p.raised), p.ink),
            Kind::Plain => (Some(p.fill), p.ink),
            Kind::Ghost => (None, p.accent),
            Kind::Destructive => (Some(p.danger), Color32::WHITE),
        };
        if self.kind == Kind::Secondary {
            p.shadows.control.paint(&painter, rect, r);
        }
        if let Some(fill) = fill {
            painter.rect_filled(rect, r, fill);
        }
        let dark = ui.visuals().dark_mode;
        let wash = match self.kind {
            Kind::Primary | Kind::Destructive if pressed || hovered => {
                let a = if pressed { 0.14 } else { 0.07 };
                Some(if dark {
                    Color32::from_white_alpha((a * 1.4 * 255.0) as u8)
                } else {
                    Color32::from_black_alpha((a * 255.0) as u8)
                })
            }
            _ if pressed => Some(p.fill_strong),
            _ if hovered => Some(p.fill),
            _ => None,
        };
        if let Some(w) = wash {
            painter.rect_filled(rect, r, w);
        }
        let galley = painter.layout_no_wrap(self.label.into(), self.font(), ink);
        let icon_w =
            if self.icon.is_some() { 16.0 + if self.label.is_empty() { 0.0 } else { 6.0 } } else { 0.0 };
        let mut x = rect.center().x - (galley.size().x + icon_w) / 2.0;
        let cy = rect.center().y;
        if let (Some(icon), false) = (self.icon, self.trailing) {
            crate::icons::heavy(|| {
                icon(&painter, Rect::from_min_size(pos2(x, cy - 8.0), vec2(16.0, 16.0)), ink)
            });
            x += icon_w;
        }
        let gw = galley.size().x;
        painter.galley(pos2(x, cy - galley.size().y / 2.0), galley, ink);
        if let (Some(icon), true) = (self.icon, self.trailing) {
            let r = Rect::from_min_size(pos2(x + gw + 6.0, cy - 8.0), vec2(16.0, 16.0));
            crate::icons::heavy(|| icon(&painter, r, ink));
        }
        resp
    }

    /// Draws the button with its right edge at `right`, centred on `cy`; returns its rectangle.
    pub fn show_right(
        &self,
        ui: &mut Ui,
        p: &Palette,
        right: f32,
        cy: f32,
        id: egui::Id,
    ) -> (Rect, Response) {
        let w = self.width(ui.painter());
        let rect = Rect::from_min_size(pos2(right - w, cy - self.height / 2.0), vec2(w, self.height));
        (rect, self.show(ui, p, rect, id))
    }

    /// Draws the button with its left edge at `left`, centred on `cy`; returns its rectangle.
    pub fn show_left(&self, ui: &mut Ui, p: &Palette, left: f32, cy: f32, id: egui::Id) -> (Rect, Response) {
        let w = self.width(ui.painter());
        let rect = Rect::from_min_size(pos2(left, cy - self.height / 2.0), vec2(w, self.height));
        (rect, self.show(ui, p, rect, id))
    }
}

/// Badge kinds (`ds-15`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BadgeKind {
    Neutral,
    Accent,
    Solid,
    /// A 14 % tint of `success`, with a dot.
    Success,
    Danger,
    Outline,
}

fn badge_font() -> egui::FontId {
    theme::ui_semibold(11.0)
}

/// Width of a [`badge`].
pub fn badge_width(painter: &Painter, kind: BadgeKind, label: &str) -> f32 {
    let w = painter.layout_no_wrap(label.into(), badge_font(), Color32::PLACEHOLDER).size().x;
    let dot = if kind == BadgeKind::Success { 6.0 + 5.0 } else { 0.0 };
    (w + dot + 14.0).max(20.0)
}

/// A badge (`ds-15`): 20 pt high, radius 10, 11 pt semibold, 7 pt sides. Drawn from `left`,
/// centred on `cy`; returns its rectangle.
pub fn badge(painter: &Painter, p: &Palette, left: f32, cy: f32, kind: BadgeKind, label: &str) -> Rect {
    let w = badge_width(painter, kind, label);
    let r = Rect::from_min_size(pos2(left, cy - 10.0), vec2(w, 20.0));
    let (fill, ink) = match kind {
        BadgeKind::Neutral => (p.fill, p.ink_2),
        BadgeKind::Accent => (p.accent_soft, p.accent),
        BadgeKind::Solid => (p.accent, p.on_accent),
        BadgeKind::Success => (p.success.gamma_multiply(0.14), p.success),
        BadgeKind::Danger => (p.danger.gamma_multiply(0.14), p.danger),
        BadgeKind::Outline => (Color32::TRANSPARENT, p.ink_2),
    };
    painter.rect_filled(r, 10.0, fill);
    if kind == BadgeKind::Outline {
        painter.rect_stroke(r, 10.0, Stroke::new(1.0, p.border), StrokeKind::Inside);
    }
    let mut x = r.left() + 7.0;
    if kind == BadgeKind::Success {
        painter.circle_filled(pos2(x + 3.0, cy), 3.0, p.success);
        x += 11.0;
    }
    painter.text(pos2(x, cy), egui::Align2::LEFT_CENTER, label, badge_font(), ink);
    r
}

/// Width of a [`pill_tab`].
pub fn pill_tab_width(painter: &Painter, label: &str, count: Option<&str>, selected: bool, dot: bool) -> f32 {
    let font = if selected { theme::ui_semibold(13.0) } else { theme::ui_medium(13.0) };
    let mut w = painter.layout_no_wrap(label.into(), font, Color32::PLACEHOLDER).size().x + 24.0;
    if dot {
        w += 10.0 + 6.0;
    }
    if let Some(c) = count {
        w += 6.0 + painter.layout_no_wrap(c.into(), theme::ui_medium(12.0), Color32::PLACEHOLDER).size().x;
    }
    w
}

/// A pill tab (`ds-15`): 28 pt, radius 14, 13 pt medium secondary ink with an optional 10 pt
/// colour dot before it and a 12 pt count after; selected, accent on `accent_soft`.
#[allow(clippy::too_many_arguments)]
pub fn pill_tab(
    ui: &mut Ui,
    p: &Palette,
    rect: Rect,
    label: &str,
    count: Option<&str>,
    dot: Option<Color32>,
    selected: bool,
    id: egui::Id,
) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, 14.0, p.accent_soft);
    } else if let Some(fill) = hover_fill(ui, &resp, p, false) {
        painter.rect_filled(rect, 14.0, fill);
    }
    let (font, ink, count_ink) = if selected {
        (theme::ui_semibold(13.0), p.accent, p.accent)
    } else {
        (theme::ui_medium(13.0), p.ink_2, p.ink_3)
    };
    let g = painter.layout_no_wrap(label.into(), font, ink);
    let cy = rect.center().y;
    let mut x = rect.left() + 12.0;
    if let Some(c) = dot {
        painter.circle_filled(pos2(x + 5.0, cy), 5.0, c);
        x += 16.0;
    }
    let w = g.size().x;
    painter.galley(pos2(x, cy - g.size().y / 2.0), g, ink);
    if let Some(c) = count {
        painter.text(pos2(x + w + 6.0, cy), egui::Align2::LEFT_CENTER, c, theme::ui_medium(12.0), count_ink);
    }
    resp
}

/// A piece of a line in one font.
pub struct Run {
    pub text: String,
    pub font: egui::FontId,
    pub tracking: f32,
}

/// `word`'s runs: words in capitals (LORD) are set in small capitals of `small` points.
pub fn word_runs(word: &str, font: &egui::FontId, small: f32) -> Vec<Run> {
    let caps = crate::reader::layout::caps_words(word);
    if caps.is_empty() {
        return vec![Run { text: word.into(), font: font.clone(), tracking: 0.0 }];
    }
    let mut runs = Vec::new();
    let mut last = 0;
    for r in caps {
        if r.start > last {
            runs.push(Run { text: word[last..r.start].into(), font: font.clone(), tracking: 0.0 });
        }
        let first = word[r.start..].chars().next().map_or(0, char::len_utf8);
        runs.push(Run { text: word[r.start..r.start + first].into(), font: font.clone(), tracking: 0.0 });
        runs.push(Run {
            text: word[r.start + first..r.end].into(),
            font: egui::FontId::new(small, font.family.clone()),
            tracking: 0.04,
        });
        last = r.end;
    }
    if last < word.len() {
        runs.push(Run { text: word[last..].into(), font: font.clone(), tracking: 0.0 });
    }
    runs
}

pub fn runs_width(painter: &Painter, runs: &[Run]) -> f32 {
    runs.iter()
        .map(|r| line(painter, &r.text, r.font.clone(), Color32::PLACEHOLDER, r.tracking).size().x)
        .sum()
}

/// Wraps `text` into at most `rows` lines `width` wide; when it doesn't fit, the last line ends
/// with an ellipsis after a whole word, as the design's samples do.
pub fn flow(
    painter: &Painter,
    text: &str,
    font: &egui::FontId,
    small: f32,
    width: f32,
    rows: usize,
) -> Vec<Vec<Run>> {
    flow_indented(painter, text, font, small, width, width, rows)
}

/// [`flow`] with a narrower first line (`first` wide), as after a verse number.
pub fn flow_indented(
    painter: &Painter,
    text: &str,
    font: &egui::FontId,
    small: f32,
    width: f32,
    first: f32,
    rows: usize,
) -> Vec<Vec<Run>> {
    let space = line(painter, " ", font.clone(), Color32::PLACEHOLDER, 0.0).size().x;
    let ellipsis = line(painter, "…", font.clone(), Color32::PLACEHOLDER, 0.0).size().x;
    // A word followed by a line break ends its line.
    let words: Vec<(Vec<Run>, f32, bool)> = text
        .split(' ')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let runs = word_runs(w.trim_end_matches('\n'), font, small);
            let width = runs_width(painter, &runs);
            (runs, width, w.ends_with('\n'))
        })
        .collect();
    let mut lines: Vec<Vec<Run>> = Vec::new();
    let mut i = 0;
    while i < words.len() && lines.len() < rows {
        let last_row = lines.len() + 1 == rows;
        let mut used = 0.0;
        let mut l: Vec<Run> = Vec::new();
        while i < words.len() {
            let (runs, w, ends_line) = &words[i];
            let gap = if l.is_empty() { 0.0 } else { space };
            // On the last row, keep room for the ellipsis unless this is the final word.
            let tail = if last_row && i + 1 < words.len() { ellipsis } else { 0.0 };
            let width = if lines.is_empty() { first } else { width };
            if !l.is_empty() && used + gap + w + tail > width {
                break;
            }
            if !l.is_empty() {
                l.push(Run { text: " ".into(), font: font.clone(), tracking: 0.0 });
            }
            l.extend(runs.iter().map(|r| Run {
                text: r.text.clone(),
                font: r.font.clone(),
                tracking: r.tracking,
            }));
            used += gap + w;
            i += 1;
            if *ends_line {
                break;
            }
        }
        if last_row && i < words.len() {
            if let Some(end) = l.last_mut() {
                end.text = end.text.trim_end_matches([',', ';', ':']).to_string();
            }
            l.push(Run { text: "…".into(), font: font.clone(), tracking: 0.0 });
        }
        lines.push(l);
    }
    lines
}

pub fn paint_runs(painter: &Painter, x: f32, baseline: f32, runs: &[Run], color: Color32) {
    let mut x = x;
    for r in runs {
        let g = line(painter, &r.text, r.font.clone(), color, r.tracking);
        let w = g.size().x;
        paint_on_baseline(painter, x, baseline, g, color);
        x += w;
    }
}

/// A text field (`ds-14`): 32 pt, radius 8, the field colour with a hairline; focused, an
/// accent outline and focus ring; in error, a danger outline and halo. An optional 16 pt glyph
/// leads; `mono` sets the text in the monospace face (keys).
#[allow(clippy::too_many_arguments)]
pub fn text_field(
    ui: &mut Ui,
    p: &Palette,
    rect: Rect,
    value: &mut String,
    placeholder: &str,
    icon: Option<IconFn>,
    mono: bool,
    error: bool,
    id: egui::Id,
) -> Response {
    let focused = ui.memory(|m| m.has_focus(id));
    let hovered = ui.rect_contains_pointer(rect);
    let painter = ui.painter();
    if error {
        painter.rect_filled(rect.expand(3.0), 11.0, p.danger.gamma_multiply(0.25));
    } else if focused {
        painter.rect_filled(rect.expand(3.0), 11.0, p.focus);
    }
    painter.rect_filled(rect, 8.0, p.field);
    let stroke = if error {
        p.danger
    } else if focused {
        p.accent
    } else if hovered {
        p.ink_3
    } else {
        p.border
    };
    painter.rect_stroke(rect, 8.0, Stroke::new(1.0, stroke), StrokeKind::Inside);
    let mut x = rect.left() + 10.0;
    if let Some(icon) = icon {
        let r = Rect::from_min_size(pos2(x, rect.center().y - 8.0), vec2(16.0, 16.0));
        crate::icons::heavy(|| icon(painter, r, p.ink_3));
        x += 16.0 + 8.0;
    }
    let edit = Rect::from_min_max(pos2(x, rect.top()), pos2(rect.right() - 10.0, rect.bottom()));
    let font = if mono && !value.is_empty() { egui::FontId::monospace(13.0) } else { text::UI.font() };
    // A detached child: `ui.put` would move the parent's cursor back up to the field.
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(edit));
    child.add_sized(
        edit.size(),
        egui::TextEdit::singleline(value)
            .id(id)
            .hint_text(egui::RichText::new(placeholder).color(p.ink_3).font(text::UI.font()))
            .font(font)
            .text_color(p.ink)
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::ZERO)
            .vertical_align(egui::Align::Center)
            .desired_width(edit.width()),
    )
}

/// A segmented control sized to its labels (`ds-07`): segments 28 pt with 10 pt sides, 2 pt
/// apart in a 32 pt `fill` track; the selected one `raised` with the knob shadow. Drawn with its
/// right edge at `right_center`; returns the track and the newly selected index.
pub fn segmented_tight(
    ui: &mut Ui,
    p: &Palette,
    right_center: egui::Pos2,
    id: egui::Id,
    labels: &[&str],
    selected: usize,
) -> (Rect, Option<usize>) {
    let widths: Vec<f32> = labels
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let font = if i == selected { text::UI_STRONG.font() } else { theme::ui_medium(text::UI.size) };
            ui.painter().layout_no_wrap(l.to_string(), font, Color32::PLACEHOLDER).size().x + 20.0
        })
        .collect();
    let total: f32 = widths.iter().sum::<f32>() + 2.0 * (labels.len() as f32 - 1.0) + 4.0;
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
        let (f, c) = if i == selected {
            (text::UI_STRONG.font(), p.ink)
        } else {
            (theme::ui_medium(text::UI.size), p.ink_2)
        };
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, *label, f, c);
        if resp.clicked() && i != selected {
            out = Some(i);
        }
        x += w + 2.0;
    }
    (track, out)
}

/// A 28 pt round icon button: no fill until hovered (`fill`) or pressed (`fill_strong`); a
/// 16 pt glyph in secondary ink.
pub fn round_button(ui: &mut Ui, p: &Palette, rect: Rect, icon: IconFn, id: egui::Id) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    if let Some(fill) = hover_fill(ui, &resp, p, false) {
        ui.painter().circle_filled(rect.center(), rect.width() / 2.0, fill);
    }
    let size = if rect.width() < 26.0 { 12.0 } else { 16.0 };
    let r = Rect::from_center_size(rect.center(), vec2(size, size));
    crate::icons::heavy(|| icon(ui.painter(), r, p.ink_2));
    resp
}
