//! The book picker: its grids keep the menu insets with cells edge to edge like menu rows, the
//! book and chapter being read are selected, and the keyboard alone can choose a chapter.

use bible_core::books::{BookId, ChapterIdx};
use bible_rs::actions::Action;
use bible_rs::book_picker;
use bible_rs::menu::{ITEM_HEIGHT, PADDING_X, PADDING_Y};
use bible_rs::theme::{self, Theme, menu as tokens};
use eframe::egui::{self, Key};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

const JOHN: BookId = BookId(42);
const GALATIANS: BookId = BookId(47);

/// A button standing in for the toolbar's location button, opening the picker on John 1, with the
/// picker open.
fn open() -> Harness<'static, Vec<Action>> {
    open_in(egui::vec2(1280.0, 800.0))
}

/// The same in a window of `size`.
fn open_in(size: egui::Vec2) -> Harness<'static, Vec<Action>> {
    let here = ChapterIdx::new(JOHN, 1).unwrap();
    let mut harness = Harness::builder().with_size(size).build_ui_state(
        move |ui, actions: &mut Vec<Action>| {
            let anchor = ui.button("John 1");
            book_picker::show(&anchor, Theme::Light.palette(), None, here, actions);
        },
        Vec::new(),
    );
    harness.ctx.set_fonts(theme::font_definitions());
    theme::apply(&harness.ctx, Theme::Light);
    harness.run();
    harness.get_by_label("John 1").click();
    harness.run();
    assert!(book_picker::is_open(&harness.ctx));
    harness
}

fn near(a: f32, b: f32, what: &str) {
    assert!((a - b).abs() < 0.01, "{what}: {a} ≠ {b}");
}

/// Whether a cell shows as selected (egui reports a selected button as toggled on).
fn selected(harness: &Harness<'_, Vec<Action>>, label: &str) -> bool {
    harness.get_by_label(label).accesskit_node().toggled() == Some(egui::accesskit::Toggled::True)
}

#[test]
fn books_sit_in_the_menu_insets_with_the_current_one_selected() {
    let harness = open();
    let area = harness.ctx.memory(|m| m.area_rect(egui::Id::new("book-picker"))).expect("the picker is open");
    let edge = area.shrink(Theme::Light.palette().shadows.popover.ring_width);
    let rect = |label: &str| harness.get_by_label(label).rect();
    let (genesis, exodus, leviticus, numbers) =
        (rect("Genesis"), rect("Exodus"), rect("Leviticus"), rect("Numbers"));
    let (matthew, luke, haggai) = (rect("Matthew"), rect("Luke"), rect("Haggai"));

    // The cells' highlights keep the menu's padding, under a section header.
    near(genesis.left() - edge.left(), PADDING_X, "left inset");
    near(genesis.top() - edge.top(), PADDING_Y + tokens::HEADER_HEIGHT, "top inset");
    near(edge.right() - luke.right(), PADDING_X, "right inset");
    near(edge.bottom() - haggai.bottom(), PADDING_Y, "bottom inset");
    // Rows of the menu's height, edge to edge across and down, as menu rows are.
    near(genesis.height(), ITEM_HEIGHT, "row height");
    near(exodus.left(), genesis.right(), "columns abut");
    near(numbers.top(), genesis.bottom(), "rows abut");
    near(numbers.left(), genesis.left(), "column aligned");
    // The testaments: one gutter between them, their first rows level.
    near(matthew.left() - leviticus.right(), 2.0 * PADDING_X, "testament gutter");
    near(matthew.top(), genesis.top(), "testaments level");

    assert!(selected(&harness, "John"), "the book being read is selected");
    assert!(!selected(&harness, "Genesis"));
}

#[test]
fn the_books_fit_the_narrowest_window() {
    // The window's minimum width.
    let harness = open_in(egui::vec2(620.0, 700.0));
    let area = harness.ctx.memory(|m| m.area_rect(egui::Id::new("book-picker"))).expect("the picker is open");
    assert!(area.left() >= 0.0 && area.right() <= 620.0, "the picker fits the window: {area:?}");
    let (genesis, exodus) = (harness.get_by_label("Genesis").rect(), harness.get_by_label("Exodus").rect());
    // The cells narrow but keep the text inset before each name and a gap after the longest.
    let longest = harness.ctx.fonts_mut(|f| {
        f.layout_no_wrap("Lamentations".into(), theme::text::UI.font(), egui::Color32::WHITE).size().x
    });
    assert!(genesis.width() >= longest + tokens::ITEM_PADDING, "{} < {longest} + inset", genesis.width());
    near(exodus.left(), genesis.right(), "columns abut");
}

#[test]
fn the_keyboard_chooses_a_book_and_a_chapter() {
    let mut harness = open();
    // From John: right to Acts and Romans, down to Galatians, return for its chapters.
    for key in [Key::ArrowRight, Key::ArrowRight, Key::ArrowDown, Key::Enter] {
        harness.key_press(key);
        harness.run();
    }
    assert!(harness.query_by_label("Genesis").is_none(), "the chapters replace the books");
    assert!(!selected(&harness, "1"), "John is not the book being read here");
    // Backspace goes back to the books, return on the highlighted book (Galatians) returns.
    harness.key_press(Key::Backspace);
    harness.run();
    assert!(harness.query_by_label("Genesis").is_some(), "back on the books");
    harness.key_press(Key::Enter);
    harness.run();
    // The highlight starts on chapter 1: right to 2, return.
    for key in [Key::ArrowRight, Key::Enter] {
        harness.key_press(key);
        harness.run();
    }
    let chapter = ChapterIdx::new(GALATIANS, 2).unwrap();
    assert_eq!(*harness.state(), vec![Action::Jump { chapter, verse: None, flash: false }]);
    assert!(!book_picker::is_open(&harness.ctx), "choosing a chapter closes the picker");
}

#[test]
fn the_current_chapter_is_selected_and_escape_closes() {
    let mut harness = open();
    harness.get_by_label("John").click();
    harness.run();
    assert!(selected(&harness, "1"), "John 1 is the chapter being read");
    assert!(!selected(&harness, "2"));
    assert!(harness.state().is_empty(), "a book with chapters opens them rather than jumping");
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!book_picker::is_open(&harness.ctx));
    // It opens on the books again.
    harness.get_by_label("John 1").click();
    harness.run();
    assert!(harness.query_by_label("Genesis").is_some());
}
