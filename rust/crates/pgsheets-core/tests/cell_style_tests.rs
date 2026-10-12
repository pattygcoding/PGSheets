//! The cell style value, and how it sits on a cell.

use pgsheets_core::{CellData, CellStyle, HorizontalAlign};

#[test]
fn the_default_style_says_nothing() {
    let style = CellStyle::DEFAULT;

    assert!(style.is_default());
    assert!(!style.bold);
    assert!(!style.italic);
    assert!(!style.underline);
    assert!(!style.wrap);
    assert_eq!(style.align, HorizontalAlign::General);
    assert_eq!(style.text_color, None);
    assert_eq!(style.fill, None);
}

#[test]
fn a_fill_is_a_background_colour() {
    let style = CellStyle::DEFAULT.with_fill(Some([0xFF, 0xEE, 0x00]));

    assert_eq!(style.fill, Some([0xFF, 0xEE, 0x00]));
    assert!(!style.is_default(), "a filled cell is not an unstyled one");

    // Clearing it puts the style back to a plain one.
    assert!(style.with_fill(None).is_default());
}

#[test]
fn a_builder_sets_only_the_field_it_names() {
    let style = CellStyle::DEFAULT
        .with_bold(true)
        .with_align(HorizontalAlign::Right);

    assert!(style.bold);
    assert!(!style.italic, "the other attributes were left alone");
    assert_eq!(style.align, HorizontalAlign::Right);
    assert!(
        !style.is_default(),
        "a style that says something is not the default"
    );
}

#[test]
fn general_is_the_default_alignment() {
    assert_eq!(HorizontalAlign::default(), HorizontalAlign::General);
}

#[test]
fn a_cell_keeps_a_style_that_has_no_value() {
    let mut cell = CellData::new("");
    cell.set_style(CellStyle::DEFAULT.with_bold(true));

    assert!(cell.style().bold);
    assert!(cell.has_format(), "a style is a format, so the cell is not bare");
    assert!(cell.is_format_only(), "and it is held only for that format");
}
