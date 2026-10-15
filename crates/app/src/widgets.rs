//! Small custom widgets shared across the UI.

use eframe::egui::{
    self, Color32, CornerRadius, Painter, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, vec2,
};

use crate::theme::{self, metrics};

/// A borderless toolbar button with a painted icon.
pub fn icon_button(ui: &mut Ui, tooltip: &str, active: bool, paint: fn(&Painter, Rect, Color32)) -> Response {
    let [w, h] = metrics::TOOLBAR_BUTTON;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
    let v = ui.visuals();
    let enabled = ui.is_enabled();
    if enabled && (resp.hovered() || active) {
        ui.painter().rect_filled(rect, CornerRadius::same(6), v.widgets.hovered.bg_fill);
    }
    let color = if !enabled {
        v.weak_text_color().gamma_multiply(0.5)
    } else if active {
        v.hyperlink_color
    } else {
        v.text_color()
    };
    paint(ui.painter(), Rect::from_center_size(rect.center(), Vec2::splat(metrics::ICON)), color);
    resp.on_hover_text(tooltip)
}

const PILL_FONT: f32 = 14.0;
const PILL_PAD: f32 = 10.0;
const PILL_CHEVRON: f32 = 14.0;

/// Width of a [`pill`] showing `text`.
pub fn pill_width(ui: &Ui, text: &str) -> f32 {
    let galley =
        ui.painter().layout_no_wrap(text.to_string(), theme::ui_medium(PILL_FONT), Color32::PLACEHOLDER);
    (galley.size().x + 2.0 * PILL_PAD + PILL_CHEVRON).ceil()
}

/// A rounded, clickable label with a disclosure chevron (location and translation pickers).
pub fn pill(ui: &mut Ui, text: &str, tooltip: &str) -> Response {
    let font = theme::ui_medium(PILL_FONT);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, ui.visuals().text_color());
    let size = vec2(pill_width(ui, text), metrics::TOOLBAR_BUTTON[1]);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let v = ui.visuals();
    if resp.hovered() || resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(rect, CornerRadius::same(6), v.widgets.hovered.bg_fill);
    }
    let text_pos = egui::pos2(rect.left() + PILL_PAD, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(text_pos, galley, v.text_color());
    let r = Rect::from_center_size(
        egui::pos2(rect.right() - PILL_PAD - PILL_CHEVRON / 2.0 + 3.0, rect.center().y),
        vec2(10.0, 10.0),
    );
    crate::icons::chevron_down(ui.painter(), r, v.weak_text_color());
    resp.on_hover_text(tooltip)
}

/// A segmented control; returns the newly selected index if it changed.
pub fn segmented(ui: &mut Ui, labels: &[&str], selected: usize) -> Option<usize> {
    let mut out = None;
    let v = ui.visuals().clone();
    let h = 26.0;
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(7), v.widgets.hovered.bg_fill);
    let seg_w = w / labels.len() as f32;
    for (i, label) in labels.iter().enumerate() {
        let r = Rect::from_min_size(egui::pos2(rect.left() + seg_w * i as f32, rect.top()), vec2(seg_w, h))
            .shrink(2.0);
        let resp = ui.interact(r, ui.id().with(("seg", label)), Sense::click());
        if i == selected {
            ui.painter().rect_filled(r, CornerRadius::same(6), v.panel_fill);
            ui.painter().rect_stroke(
                r,
                CornerRadius::same(6),
                Stroke::new(1.0, v.widgets.noninteractive.bg_stroke.color),
                StrokeKind::Inside,
            );
        }
        let color = if i == selected { v.text_color() } else { v.weak_text_color() };
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, *label, theme::ui_medium(12.5), color);
        if resp.clicked() && i != selected {
            out = Some(i);
        }
    }
    out
}

/// Muted section caption.
pub fn caption(ui: &mut Ui, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .font(theme::ui_medium(10.5))
            .color(ui.visuals().weak_text_color())
            .extra_letter_spacing(0.8),
    );
}
