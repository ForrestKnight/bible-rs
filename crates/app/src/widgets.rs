//! Small custom widgets shared across the UI.

use eframe::egui::{
    self, Color32, CornerRadius, Painter, Rect, Response, Sense, Stroke, StrokeKind, Ui, vec2,
};

use crate::theme;

/// A borderless square button with a painted icon.
pub fn icon_button(ui: &mut Ui, tooltip: &str, active: bool, paint: fn(&Painter, Rect, Color32)) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(30.0, 28.0), Sense::click());
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
    paint(ui.painter(), rect.shrink2(vec2(6.0, 5.0)), color);
    resp.on_hover_text(tooltip)
}

/// A rounded, clickable label (location and translation pickers).
pub fn pill(ui: &mut Ui, text: &str, chevron: bool, tooltip: &str) -> Response {
    let font = theme::ui_medium(14.0);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, ui.visuals().text_color());
    let w = galley.size().x + if chevron { 34.0 } else { 20.0 };
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 30.0), Sense::click());
    let v = ui.visuals();
    if resp.hovered() || resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(rect, CornerRadius::same(8), v.widgets.hovered.bg_fill);
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(8),
            Stroke::new(1.0, v.widgets.noninteractive.bg_stroke.color),
            StrokeKind::Inside,
        );
    }
    let text_pos = egui::pos2(rect.left() + 10.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(text_pos, galley, v.text_color());
    if chevron {
        let r =
            Rect::from_center_size(egui::pos2(rect.right() - 13.0, rect.center().y + 1.0), vec2(12.0, 12.0));
        crate::icons::chevron_down(ui.painter(), r, v.weak_text_color());
    }
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
