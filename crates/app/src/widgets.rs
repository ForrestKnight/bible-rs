//! Small custom widgets shared across the UI.

use eframe::egui::{
    self, Color32, CornerRadius, Painter, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, vec2,
};

use crate::theme::{self, Palette, metrics};

/// The fill behind a borderless button or row: fades in on hover, darker while pressed.
pub fn hover_fill(ui: &Ui, resp: &Response, p: &Palette, on: bool) -> Option<Color32> {
    if !ui.is_enabled() {
        return None;
    }
    if resp.is_pointer_button_down_on() {
        return Some(p.pressed);
    }
    let t = ui.ctx().animate_bool_with_time(resp.id.with("hover"), resp.hovered() || on, 0.1);
    (t > 0.0).then(|| p.hover.gamma_multiply(t))
}

/// A borderless toolbar button with a painted icon. `on` marks a toggled-on state (accent icon).
pub fn icon_button(
    ui: &mut Ui,
    p: &Palette,
    tooltip: &str,
    on: bool,
    paint: fn(&Painter, Rect, Color32),
) -> Response {
    let [w, h] = metrics::TOOLBAR_BUTTON;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
    if let Some(fill) = hover_fill(ui, &resp, p, on) {
        ui.painter().rect_filled(rect, CornerRadius::same(6), fill);
    }
    let color = if !ui.is_enabled() {
        p.faint
    } else if on {
        p.accent
    } else {
        p.icon
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
pub fn pill(ui: &mut Ui, p: &Palette, text: &str, tooltip: &str) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_string(), theme::ui_medium(PILL_FONT), p.text);
    let size = vec2(pill_width(ui, text), metrics::TOOLBAR_BUTTON[1]);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if let Some(fill) = hover_fill(ui, &resp, p, false) {
        ui.painter().rect_filled(rect, CornerRadius::same(6), fill);
    }
    let text_pos = egui::pos2(rect.left() + PILL_PAD, (rect.center().y - galley.size().y / 2.0).round());
    ui.painter().galley(text_pos, galley, p.text);
    let chevron = Rect::from_center_size(
        egui::pos2(rect.right() - PILL_PAD - PILL_CHEVRON / 2.0 + 2.0, rect.center().y),
        Vec2::splat(10.0),
    );
    crate::icons::chevron_down(ui.painter(), chevron, p.muted);
    resp.on_hover_text(tooltip)
}

/// The disclosure icon for egui combo boxes (instead of egui's triangle).
pub fn combo_chevron(ui: &Ui, rect: Rect, visuals: &egui::style::WidgetVisuals, _open: bool) {
    let r = Rect::from_center_size(rect.center(), Vec2::splat(10.0));
    crate::icons::chevron_down(ui.painter(), r, visuals.fg_stroke.color.gamma_multiply(0.7));
}

/// A segmented control; returns the newly selected index if it changed.
pub fn segmented(ui: &mut Ui, labels: &[&str], selected: usize) -> Option<usize> {
    let mut out = None;
    let v = ui.visuals().clone();
    let h = 24.0;
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

/// A section caption, in the style of a macOS source list header.
pub fn caption(ui: &mut Ui, text: &str) {
    ui.label(egui::RichText::new(text).font(theme::ui_medium(11.5)).color(ui.visuals().weak_text_color()));
}
