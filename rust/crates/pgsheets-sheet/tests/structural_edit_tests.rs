//! Inserting and deleting rows and columns: what moves, what a formula becomes, and that a whole
//! structural edit undoes in one step.

mod support;

use pgsheets_core::CellAddress;
use pgsheets_formulas::Engine;
use pgsheets_sheet::Workbook;
use support::LiteralEngine;

fn sheet() -> Workbook {
    Workbook::new(Box::new(Engine::new()))
}

fn literal_sheet() -> Workbook {
    Workbook::new(Box::new(LiteralEngine))
}

fn at(a1: &str) -> CellAddress {
    let (row, column) = CellAddress::try_parse_a1(a1).unwrap_or_else(|| panic!("bad address {a1}"));
    CellAddress::new(row, column)
}

fn set(workbook: &mut Workbook, a1: &str, raw: &str) {
    let address = at(a1);
    workbook.set_raw_input(address.row, address.column, raw);
}

fn display(workbook: &Workbook, a1: &str) -> String {
    let address = at(a1);
    workbook.display_value(address.row, address.column).to_string()
}

#[test]
fn inserting_a_row_pushes_cells_down() {
    let mut workbook = literal_sheet();
    set(&mut workbook, "A1", "first");
    set(&mut workbook, "A2", "second");

    assert!(workbook.insert_rows(1, 1));

    assert_eq!(display(&workbook, "A1"), "first");
    assert_eq!(display(&workbook, "A2"), "");
    assert_eq!(display(&workbook, "A3"), "second");
}

#[test]
fn inserting_rows_moves_the_formulas_that_point_below_them() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "1");
    set(&mut workbook, "A2", "2");
    set(&mut workbook, "B1", "=SUM(A1:A2)");

    assert_eq!(display(&workbook, "B1"), "3");

    // The sum range grows to take in the inserted row, which is what a spreadsheet does.
    assert!(workbook.insert_rows(1, 1));

    assert_eq!(workbook.raw_input(0, 1), "=SUM(A1:A3)");
}

#[test]
fn inserting_columns_moves_the_references_that_point_at_them() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "2");
    set(&mut workbook, "B1", "=A1*5");

    assert!(workbook.insert_columns(0, 1));

    assert_eq!(workbook.raw_input(0, 2), "=B1*5");
    assert_eq!(display(&workbook, "C1"), "10");
}

#[test]
fn deleting_a_row_removes_it_and_pulls_the_rest_up() {
    let mut workbook = literal_sheet();
    set(&mut workbook, "A1", "1");
    set(&mut workbook, "A2", "2");
    set(&mut workbook, "A3", "3");

    assert!(workbook.delete_rows(0, 1));

    assert_eq!(workbook.cell_count(), 2);
    assert_eq!(display(&workbook, "A1"), "2");
    assert_eq!(display(&workbook, "A2"), "3");
}

#[test]
fn deleting_a_referenced_row_leaves_a_reference_error_and_keeps_calculating() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "1");
    set(&mut workbook, "A2", "2");
    set(&mut workbook, "B1", "=A2*10+1");

    assert!(workbook.delete_rows(1, 1));

    assert_eq!(workbook.raw_input(0, 1), "=#REF!*10+1");
    assert_eq!(display(&workbook, "B1"), "#REF!");
}

#[test]
fn a_structural_edit_is_one_undo_step() {
    let mut workbook = literal_sheet();
    set(&mut workbook, "A1", "first");
    set(&mut workbook, "A2", "second");

    let depth = workbook.undo_depth();
    assert!(workbook.insert_rows(0, 1));
    assert_eq!(workbook.undo_depth(), depth + 1);

    workbook.undo();

    assert_eq!(display(&workbook, "A1"), "first");
    assert_eq!(display(&workbook, "A2"), "second");
}

#[test]
fn a_number_format_travels_with_its_cell_through_a_structural_edit() {
    let mut workbook = literal_sheet();
    set(&mut workbook, "A2", "0.155");
    workbook.set_number_format(1, 0, Some("0.0%"));

    assert!(workbook.insert_rows(0, 1));

    assert_eq!(display(&workbook, "A3"), "15.5%");
    assert_eq!(workbook.number_format(2, 0), Some("0.0%"));
    assert_eq!(workbook.number_format(1, 0), None);
}

#[test]
fn a_format_survives_undoing_a_structural_edit() {
    let mut workbook = literal_sheet();
    set(&mut workbook, "A2", "0.155");
    workbook.set_number_format(1, 0, Some("0.0%"));

    assert!(workbook.insert_rows(0, 1));
    workbook.undo();

    assert_eq!(display(&workbook, "A2"), "15.5%");
    assert_eq!(workbook.number_format(1, 0), Some("0.0%"));
}

#[test]
fn deleting_a_formatted_row_takes_the_format_with_it() {
    let mut workbook = literal_sheet();
    set(&mut workbook, "A1", "1");
    workbook.set_number_format(0, 0, Some("0.00"));
    set(&mut workbook, "A2", "2");
    workbook.set_number_format(1, 0, Some("0.0%"));

    assert!(workbook.delete_rows(0, 1));

    // The deleted row is gone, format and all; the row below moved up carrying its own format.
    assert_eq!(workbook.cell_count(), 1);
    assert_eq!(workbook.number_format(0, 0), Some("0.0%"));
    assert_eq!(display(&workbook, "A1"), "200.0%");
}

#[test]
fn an_edit_that_would_push_data_off_the_sheet_is_refused() {
    let mut workbook = literal_sheet();
    let last_row = pgsheets_core::MAX_ROWS - 1;
    set(&mut workbook, "A1048576", "last");

    assert!(!workbook.insert_rows(last_row, 1));

    // Nothing moved, because refusing is better than silently dropping a cell.
    assert_eq!(display(&workbook, "A1048576"), "last");
}

#[test]
fn a_formula_that_cannot_be_read_is_left_alone_by_a_structural_edit() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "=1+");
    set(&mut workbook, "A2", "value");

    assert!(workbook.insert_rows(0, 1));

    // The unreadable formula moved with its cell and its text is untouched. It still evaluates to the
    // error a broken formula produces, rather than to something the rewriter guessed at.
    assert_eq!(workbook.raw_input(1, 0), "=1+");
    assert_eq!(display(&workbook, "A2"), "#VALUE!");
    assert_eq!(workbook.raw_input(2, 0), "value");
    assert_eq!(display(&workbook, "A3"), "value");
}

#[test]
fn deleting_with_a_zero_count_changes_nothing() {
    let mut workbook = literal_sheet();
    set(&mut workbook, "A1", "value");

    assert!(workbook.delete_rows(0, 0));
    assert!(workbook.insert_rows(0, 0));

    assert_eq!(display(&workbook, "A1"), "value");
}
