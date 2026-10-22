//! Menus (design system ds-12, the `menu` tokens): the frame, rows, section headers and
//! separators every dropdown and context menu shares. egui lays out most menus ([`popup`]); the
//! verse menu and the More menu paint their rows by hand with the same geometry ([`paint_row`]).
//!
//! Row highlights sit `PADDING_X` from the menu's sides and `PADDING_Y` from its top and bottom;
//! everything in a row (text, icons, checkboxes) starts `ITEM_PADDING` inside the highlight, and
//! section headers and separators line up with that text column.

use eframe::egui::{
    self, Align2, Color32, CornerRadius, Frame, Margin, Painter, Rect, Response, Sense, Stroke, Style, Ui,
    WidgetText, pos2, vec2,
};

use crate::theme::{Palette, icon, menu as m, text};
use crate::widgets::IconFn;

pub use crate::theme::menu::{ITEM_HEIGHT, PADDING_X, PADDING_Y, SEPARATOR_HEIGHT};

/// From a header's top to its text's centre: the text sits in the lower 20 pt (ds-12).
const HEADER_TEXT_Y: f32 = 14.0;

/// Between the menu's edge and its rows.
pub fn margin() -> Margin {
    Margin::symmetric(m::PADDING_X as i8, m::PADDING_Y as i8)
}

/// The menu itself: the surface, rounded, with the popover ring and shadow and the padding.
pub fn frame(p: &Palette) -> Frame {
    let e = p.shadows.popover;
    Frame::new()
        .fill(p.surface)
        .corner_radius(CornerRadius::same(m::RADIUS as u8))
        .inner_margin(margin())
        .stroke(Stroke::new(e.ring_width, e.ring))
        .shadow(e.shadow())
}

/// An egui menu opening from `anchor`, in the menu frame, with its buttons and selectable labels
/// as full-width rows.
pub fn popup<'a>(anchor: &Response, p: &Palette) -> egui::Popup<'a> {
    egui::Popup::menu(anchor).frame(frame(p)).style(style)
}

/// egui's menu style with the rows' metrics: 28 pt high, edge to edge, text inset from the
/// highlight.
fn style(style: &mut Style) {
    egui::containers::menu::menu_style(style);
    style.spacing.button_padding = vec2(m::ITEM_PADDING, 0.0);
    style.spacing.interact_size.y = m::ITEM_HEIGHT;
    style.spacing.item_spacing.y = 0.0;
    let w = &mut style.visuals.widgets;
    for state in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        state.corner_radius = CornerRadius::same(m::ITEM_RADIUS as u8);
    }
}

/// A separator between groups of rows, inset to the text column.
pub fn separator(ui: &mut Ui) {
    ui.add(egui::Separator::default().spacing(m::SEPARATOR_HEIGHT).shrink(m::ITEM_PADDING));
}

/// A section header ("Parallel view"): caption type, tertiary ink, in the text column.
pub fn header(ui: &mut Ui, p: &Palette, label: &str) {
    let width = ui.painter().layout_no_wrap(label.into(), text::CAPTION.font(), p.ink_3).size().x;
    let (r, _) = ui.allocate_at_least(vec2(width + 2.0 * m::ITEM_PADDING, m::HEADER_HEIGHT), Sense::hover());
    paint_header(ui.painter(), p, r, label);
}

/// A checkbox row, its box in the text column.
pub fn checkbox(ui: &mut Ui, checked: &mut bool, label: impl Into<WidgetText>) -> Response {
    let pad = m::ITEM_PADDING as i8;
    Frame::new()
        .inner_margin(Margin { left: pad, right: pad, top: 0, bottom: 0 })
        .show(ui, |ui| ui.checkbox(checked, label))
        .inner
}

/// The header of a painted menu, in a `HEADER_HEIGHT` rect as wide as the rows.
pub fn paint_header(painter: &Painter, p: &Palette, r: Rect, label: &str) {
    painter.text(
        pos2(r.left() + m::ITEM_PADDING, r.top() + HEADER_TEXT_Y),
        Align2::LEFT_CENTER,
        label,
        text::CAPTION.font(),
        p.ink_3,
    );
}

/// The separator of a painted menu, in a `SEPARATOR_HEIGHT` band from `top` across rows from
/// `left` to `right`.
pub fn paint_separator(painter: &Painter, p: &Palette, left: f32, right: f32, top: f32) {
    painter.hline(
        left + m::ITEM_PADDING..=right - m::ITEM_PADDING,
        top + m::SEPARATOR_HEIGHT / 2.0,
        Stroke::new(1.0, p.separator),
    );
}

/// One row of a painted menu.
pub struct Row<'a> {
    pub icon: IconFn,
    pub label: &'a str,
    pub shortcut: &'a str,
    pub enabled: bool,
    /// Under the keyboard or the pointer: the accent highlight.
    pub lit: bool,
    pub pressed: bool,
}

/// Where a painted row's left-hand content starts: the icon, then the label after it.
pub fn text_left(r: Rect) -> f32 {
    r.left() + m::ITEM_PADDING
}

/// Paints a row: the highlight, a 16 pt icon in the text column, the label after it and the
/// shortcut at the trailing edge.
pub fn paint_row(painter: &Painter, p: &Palette, r: Rect, row: Row<'_>) {
    let lit = row.enabled && row.lit;
    if lit {
        let radius = m::ITEM_RADIUS;
        painter.rect_filled(r, radius, p.accent);
        if row.pressed {
            painter.rect_filled(r, radius, Color32::from_black_alpha(18));
        }
    }
    let (ink, ink2, ink3) = match (row.enabled, lit) {
        (false, _) => (p.ink_3, p.ink_3, p.ink_3),
        (true, true) => (p.on_accent, p.on_accent, p.on_accent),
        (true, false) => (p.ink, p.ink_2, p.ink_3),
    };
    let cy = r.center().y;
    let x = text_left(r);
    let glyph = Rect::from_min_size(pos2(x, cy - icon::MENU / 2.0), vec2(icon::MENU, icon::MENU));
    crate::icons::heavy(|| (row.icon)(painter, glyph, ink2));
    let font = text::UI.font();
    painter.text(pos2(glyph.right() + m::ICON_GAP, cy), Align2::LEFT_CENTER, row.label, font.clone(), ink);
    let shortcut = crate::keymap::shown(row.shortcut);
    painter.text(pos2(r.right() - m::ITEM_PADDING, cy), Align2::RIGHT_CENTER, shortcut, font, ink3);
}

/// One cell of a grid menu (the book picker): a row-height highlight like a menu row's.
pub struct Cell<'a> {
    /// Drawn in the text column before the label.
    pub icon: Option<IconFn>,
    pub label: &'a str,
    /// Numbers sit centred in their cell; names start in the text column.
    pub centred: bool,
    /// The current choice (the book or chapter being read).
    pub selected: bool,
    /// Under the pointer or the keyboard.
    pub lit: bool,
    pub pressed: bool,
}

/// Paints a grid menu cell in the colours egui menus give their rows (`theme::apply`): the
/// selection fill for the current choice, a fill under the pointer and a stronger one pressed.
pub fn paint_cell(painter: &Painter, p: &Palette, r: Rect, cell: Cell<'_>) {
    let fill = if cell.selected {
        p.selection
    } else if cell.pressed {
        p.fill_strong
    } else if cell.lit {
        p.fill
    } else {
        Color32::TRANSPARENT
    };
    painter.rect_filled(r, m::ITEM_RADIUS, fill);
    let cy = r.center().y;
    let mut x = text_left(r);
    if let Some(icon) = cell.icon {
        let glyph = Rect::from_min_size(pos2(x, cy - icon::MENU / 2.0), vec2(icon::MENU, icon::MENU));
        crate::icons::heavy(|| icon(painter, glyph, p.ink_2));
        x = glyph.right() + m::ICON_GAP;
    }
    let (pos, align) =
        if cell.centred { (r.center(), Align2::CENTER_CENTER) } else { (pos2(x, cy), Align2::LEFT_CENTER) };
    painter.text(pos, align, cell.label, text::UI.font(), p.ink);
}
