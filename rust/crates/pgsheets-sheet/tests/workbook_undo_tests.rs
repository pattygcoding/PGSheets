//! Undo and redo as wired into the workbook: editing, grouping and suspension.

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

fn display(workbook: &Workbook, a1: &str) -> String {
    let address = at(a1);
    workbook.display_value(address.row, address.column).to_string()
}

#[test]
fn an_edit_is_undoable() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "value");

    assert!(workbook.can_undo());

    workbook.undo();

    assert_eq!(display(&workbook, "A1"), "");
    assert_eq!(workbook.cell_count(), 0);
}

#[test]
fn undo_restores_the_previous_value_and_redo_reapplies_it() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "first");
    set(&mut workbook, "A1", "second");

    workbook.undo();
    assert_eq!(display(&workbook, "A1"), "first");

    workbook.redo();
    assert_eq!(display(&workbook, "A1"), "second");
}

#[test]
fn undo_and_redo_report_when_there_is_nothing_to_do() {
    let mut workbook = sheet();

    assert!(!workbook.undo());
    assert!(!workbook.redo());

    set(&mut workbook, "A1", "value");

    assert!(workbook.can_undo());
    assert!(!workbook.can_redo());

    workbook.undo();

    assert!(!workbook.can_undo());
    assert!(workbook.can_redo());

    workbook.redo();

    assert!(workbook.can_undo());
    assert!(!workbook.can_redo());
}

#[test]
fn setting_the_same_value_records_nothing() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "value");

    let depth = workbook.undo_depth();
    set(&mut workbook, "A1", "value");

    assert_eq!(workbook.undo_depth(), depth);
}

#[test]
fn emptying_an_absent_cell_records_nothing() {
    let mut workbook = sheet();

    set(&mut workbook, "A1", "");

    assert!(!workbook.can_undo());
}

#[test]
fn a_bulk_update_is_one_undo_step() {
    let mut workbook = sheet();

    workbook.begin_bulk_update();

    for row in 0..5 {
        workbook.set_raw_input(row, 0, &format!("value {row}"));
    }

    workbook.end_bulk_update();

    assert_eq!(workbook.undo_depth(), 1);

    workbook.undo();

    assert_eq!(workbook.cell_count(), 0);
}

#[test]
fn clear_drops_the_undo_history() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "value");

    workbook.clear();

    assert!(!workbook.can_undo());
}

#[test]
fn suspend_keeps_a_wholesale_replacement_out_of_the_history() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "before");

    {
        let _suspension = workbook.suspend_undo_history();

        workbook.begin_bulk_update();
        workbook.clear();
        set(&mut workbook, "A1", "loaded");
        workbook.end_bulk_update();
    }

    assert_eq!(display(&workbook, "A1"), "loaded");
    assert!(!workbook.can_undo());
}

#[test]
fn editing_again_after_a_suspension_is_recorded() {
    let mut workbook = sheet();

    {
        let _suspension = workbook.suspend_undo_history();
        set(&mut workbook, "A1", "loaded");
    }

    set(&mut workbook, "A2", "edited");

    assert!(workbook.can_undo());

    workbook.undo();

    assert_eq!(display(&workbook, "A2"), "");
    assert_eq!(display(&workbook, "A1"), "loaded");
}
