//! Number formats through the workbook: what the grid shows, what survives an edit, and that a
//! format is one undo step like any other change.

mod support;

use support::LiteralEngine;

use pgsheets_core::CellAddress;
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

fn format(workbook: &mut Workbook, a1: &str, code: Option<&str>) {
    let address = at(a1);
    workbook.set_number_format(address.row, address.column, code);
}

fn display(workbook: &Workbook, a1: &str) -> String {
    let address = at(a1);
    workbook.display_value(address.row, address.column).to_string()
}

fn stored_format(workbook: &Workbook, a1: &str) -> Option<String> {
    let address = at(a1);
    workbook
        .number_format(address.row, address.column)
        .map(str::to_string)
}

#[test]
fn a_format_changes_what_the_grid_shows() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "0.155");

    assert_eq!(display(&workbook, "A1"), "0.155");

    format(&mut workbook, "A1", Some("0.0%"));

    assert_eq!(display(&workbook, "A1"), "15.5%");

    // The value itself is untouched: a format changes what a cell shows, not what it holds.
    assert_eq!(workbook.value(0, 0).as_number(), Some(0.155));
}

#[test]
fn a_format_turns_a_serial_into_a_date() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "45356");
    format(&mut workbook, "A1", Some("yyyy-mm-dd"));

    assert_eq!(display(&workbook, "A1"), "2024-03-05");
}

#[test]
fn typing_a_new_value_keeps_the_format() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "1");
    format(&mut workbook, "A1", Some("0.00"));

    set(&mut workbook, "A1", "2");

    assert_eq!(display(&workbook, "A1"), "2.00");
    assert_eq!(stored_format(&workbook, "A1"), Some("0.00".to_string()));
}

#[test]
fn text_is_not_formatted() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "North");

    format(&mut workbook, "A1", Some("0.00"));

    assert_eq!(display(&workbook, "A1"), "North");
}

#[test]
fn a_format_is_one_undo_step_and_undo_leaves_the_value_alone() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "1");
    workbook.clear();
    set(&mut workbook, "A1", "1");

    let depth_before = workbook.undo_depth();
    format(&mut workbook, "A1", Some("0.00"));

    assert_eq!(display(&workbook, "A1"), "1.00");
    assert_eq!(workbook.undo_depth(), depth_before + 1);

    workbook.undo();

    assert_eq!(display(&workbook, "A1"), "1");
    assert_eq!(stored_format(&workbook, "A1"), None);

    workbook.redo();

    assert_eq!(display(&workbook, "A1"), "1.00");
}

#[test]
fn a_format_can_be_cleared() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "1");
    format(&mut workbook, "A1", Some("0.00"));

    format(&mut workbook, "A1", None);

    assert_eq!(display(&workbook, "A1"), "1");
    assert_eq!(stored_format(&workbook, "A1"), None);
}

#[test]
fn a_blank_format_means_general() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "1");
    format(&mut workbook, "A1", Some("0.00"));

    format(&mut workbook, "A1", Some("   "));

    assert_eq!(display(&workbook, "A1"), "1");
    assert_eq!(stored_format(&workbook, "A1"), None);
}

#[test]
fn applying_the_same_format_twice_records_nothing() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "1");
    format(&mut workbook, "A1", Some("0.00"));

    let depth = workbook.undo_depth();
    format(&mut workbook, "A1", Some("0.00"));

    assert_eq!(workbook.undo_depth(), depth);
}

#[test]
fn clearing_the_value_leaves_the_format_behind_for_the_next_one() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "1");
    format(&mut workbook, "A1", Some("0.00"));

    set(&mut workbook, "A1", "");

    // The format is still remembered, which is what formats the value typed next; the cell therefore
    // survives as a format-only record.
    assert_eq!(workbook.cell_count(), 1);
    assert_eq!(display(&workbook, "A1"), "");
    assert_eq!(stored_format(&workbook, "A1"), Some("0.00".to_string()));
    assert!(workbook.cell(0, 0).unwrap().is_format_only());

    set(&mut workbook, "A1", "5");

    assert_eq!(display(&workbook, "A1"), "5.00");
}

#[test]
fn formatting_an_empty_cell_remembers_the_format_for_later() {
    let mut workbook = sheet();

    format(&mut workbook, "D3", Some("0.0%"));

    assert_eq!(stored_format(&workbook, "D3"), Some("0.0%".to_string()));
    assert_eq!(display(&workbook, "D3"), "");

    set(&mut workbook, "D3", "0.25");

    assert_eq!(display(&workbook, "D3"), "25.0%");
}

#[test]
fn clearing_a_format_from_an_empty_cell_removes_it_entirely() {
    let mut workbook = sheet();
    format(&mut workbook, "A1", Some("0.00"));

    // Start from a clean history so the assertion is about the format, not the steps before it.
    workbook.clear();
    format(&mut workbook, "A1", Some("0.00"));

    format(&mut workbook, "A1", None);

    assert_eq!(workbook.cell_count(), 0);

    // Clearing a format is a change like any other, so it can be undone.
    workbook.undo();
    assert_eq!(stored_format(&workbook, "A1"), Some("0.00".to_string()));
}

#[test]
fn a_format_only_cell_counts_as_a_cell() {
    let mut workbook = sheet();

    format(&mut workbook, "B2", Some("0.00"));

    assert_eq!(workbook.cell_count(), 1);
    assert!(workbook.cell(1, 1).unwrap().is_format_only());
}
