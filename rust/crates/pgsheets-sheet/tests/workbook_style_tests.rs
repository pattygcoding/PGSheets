//! Cell styles through the workbook: that they sit on a cell, travel with it, and undo in one step
//! like any other change to that cell.

mod support;

use support::LiteralEngine;

use pgsheets_core::{CellAddress, CellStyle, HorizontalAlign};
use pgsheets_sheet::Workbook;

fn sheet() -> Workbook {
    Workbook::new(Box::new(LiteralEngine))
}

fn at(a1: &str) -> CellAddress {
    let (row, column) = CellAddress::try_parse_a1(a1).unwrap_or_else(|| panic!("unparseable address '{a1}'"));
    CellAddress::new(row, column)
}

fn set(workbook: &mut Workbook, a1: &str, raw: &str) {
    let address = at(a1);
    workbook.set_raw_input(address.row, address.column, raw);
}

fn style(workbook: &mut Workbook, a1: &str, style: CellStyle) {
    let address = at(a1);
    workbook.set_cell_style(address.row, address.column, style);
}

fn style_of(workbook: &Workbook, a1: &str) -> Option<CellStyle> {
    let address = at(a1);
    workbook.cell_style(address.row, address.column)
}

#[test]
fn a_style_is_applied_to_a_cell() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "North");

    style(&mut workbook, "A1", CellStyle::DEFAULT.with_bold(true));

    assert_eq!(style_of(&workbook, "A1").map(|s| s.bold), Some(true));
    // The value is untouched: a style changes how a cell looks, not what it holds.
    assert_eq!(workbook.raw_input(0, 0), "North");
}

#[test]
fn a_style_is_one_undo_step() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "1");
    workbook.clear();
    set(&mut workbook, "A1", "1");

    let depth_before = workbook.undo_depth();
    style(&mut workbook, "A1", CellStyle::DEFAULT.with_italic(true));

    assert_eq!(workbook.undo_depth(), depth_before + 1);
    assert_eq!(style_of(&workbook, "A1").map(|s| s.italic), Some(true));

    workbook.undo();
    assert_eq!(style_of(&workbook, "A1"), None);

    workbook.redo();
    assert_eq!(style_of(&workbook, "A1").map(|s| s.italic), Some(true));
}

#[test]
fn typing_a_value_keeps_the_style() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "1");
    style(
        &mut workbook,
        "A1",
        CellStyle::DEFAULT.with_align(HorizontalAlign::Right),
    );

    set(&mut workbook, "A1", "2");

    assert_eq!(
        style_of(&workbook, "A1").map(|s| s.align),
        Some(HorizontalAlign::Right)
    );
}

#[test]
fn a_style_added_to_an_empty_cell_remembers_it_for_later() {
    let mut workbook = sheet();

    style(&mut workbook, "D3", CellStyle::DEFAULT.with_bold(true));

    // The cell exists to carry the style, and shows nothing yet.
    assert_eq!(workbook.cell_count(), 1);
    assert_eq!(style_of(&workbook, "D3").map(|s| s.bold), Some(true));
    assert!(workbook.cell(2, 3).unwrap().is_format_only());

    set(&mut workbook, "D3", "later");

    assert_eq!(style_of(&workbook, "D3").map(|s| s.bold), Some(true));
}

#[test]
fn clearing_the_style_removes_a_bare_cell() {
    let mut workbook = sheet();
    style(&mut workbook, "A1", CellStyle::DEFAULT.with_bold(true));

    workbook.clear();
    style(&mut workbook, "A1", CellStyle::DEFAULT.with_bold(true));

    style(&mut workbook, "A1", CellStyle::DEFAULT);

    assert_eq!(workbook.cell_count(), 0);
}

#[test]
fn applying_the_same_style_twice_records_nothing() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "1");
    style(&mut workbook, "A1", CellStyle::DEFAULT.with_bold(true));

    let depth = workbook.undo_depth();
    style(&mut workbook, "A1", CellStyle::DEFAULT.with_bold(true));

    assert_eq!(workbook.undo_depth(), depth);
}

#[test]
fn a_style_follows_its_cell_through_an_inserted_row() {
    let mut workbook = sheet();
    set(&mut workbook, "A2", "moves down");
    style(&mut workbook, "A2", CellStyle::DEFAULT.with_underline(true));

    assert!(workbook.insert_rows(0, 1));

    assert_eq!(workbook.raw_input(2, 0), "moves down");
    assert_eq!(
        style_of(&workbook, "A3").map(|s| s.underline),
        Some(true),
        "the style moved with the cell"
    );
    assert_eq!(style_of(&workbook, "A2"), None);
}
