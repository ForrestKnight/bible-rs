//! Menu geometry: a menu built with `menu::popup` keeps its rows' highlights the `menu` padding
//! from its edges, and starts row content (text, checkboxes) one text column inside them.

use bible_rs::menu::{self, ITEM_HEIGHT, PADDING_X, PADDING_Y};
use bible_rs::theme::{self, Theme, menu as tokens};
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

#[derive(Default)]
struct Seen {
    popup: Option<egui::Id>,
    button_padding: egui::Vec2,
}

#[test]
fn rows_are_inset_from_the_edges_in_one_text_column() {
    let p = Theme::Light.palette();
    let mut harness = Harness::new_ui_state(
        |ui, seen: &mut Seen| {
            let open = ui.button("Open");
            seen.popup = Some(egui::Popup::default_response_id(&open));
            menu::popup(&open, p).show(|ui| {
                ui.set_min_width(240.0);
                seen.button_padding = ui.spacing().button_padding;
                ui.add(egui::Button::selectable(true, "King James Version").shortcut_text("⌘1"));
                ui.add(egui::Button::selectable(false, "World English Bible").shortcut_text("⌘2"));
                menu::separator(ui);
                menu::header(ui, p, "Parallel view");
                let mut on = false;
                menu::checkbox(ui, &mut on, "WEB side by side");
                menu::separator(ui);
                let _ = ui.button("Manage translations");
            });
        },
        Seen::default(),
    );
    // The app's fonts and styling, as at startup.
    harness.ctx.set_fonts(theme::font_definitions());
    theme::apply(&harness.ctx, Theme::Light);
    harness.run();
    harness.get_by_label("Open").click();
    harness.run();

    let id = harness.state().popup.unwrap();
    let area = harness.ctx.memory(|m| m.area_rect(id)).expect("the menu is open");
    // The frame's ring is drawn inside its edge.
    let edge = area.shrink(p.shadows.popover.ring_width);
    let first = harness.get_by_label_contains("King James").rect();
    let second = harness.get_by_label_contains("World English").rect();
    let check = harness.get_by_label("WEB side by side").rect();
    let last = harness.get_by_label("Manage translations").rect();
    let near = |a: f32, b: f32, what: &str| assert!((a - b).abs() < 0.01, "{what}: {a} ≠ {b}");

    // Highlights: the padding from every edge, rows edge to edge at the row height.
    near(first.left() - edge.left(), PADDING_X, "left inset");
    near(edge.right() - first.right(), PADDING_X, "right inset");
    near(first.top() - edge.top(), PADDING_Y, "top inset");
    near(edge.bottom() - last.bottom(), PADDING_Y, "bottom inset");
    near(first.height(), ITEM_HEIGHT, "row height");
    near(second.top(), first.bottom(), "rows abut");
    near(last.left(), first.left(), "last row aligned");

    // Content: labels the item padding inside the highlight, and checkboxes in the same column.
    near(harness.state().button_padding.x, tokens::ITEM_PADDING, "label inset");
    near(check.left() - first.left(), tokens::ITEM_PADDING, "checkbox column");
}
