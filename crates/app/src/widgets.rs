//! Small custom widgets shared across the UI, drawn to the design's component sheets
//! (`ds-06` title bar, `ds-07` buttons, `ds-10` search field in the Paper file).

use eframe::egui::{
    self, Color32, CornerRadius, Painter, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
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
        crate::icons::clear(ui.painter(), c, p.ink_3, p.field);
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
