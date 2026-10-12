//! The sparse model's invariants: only populated cells are stored, raw input and display text are
//! kept apart, a bulk update recalcs once, and a removed cell stays removed.
//!
//! The formula-dependent cases from the C# suite arrive with the engine's own suite; everything here
//! holds for any engine.

mod support;

use std::cell::Cell;
use std::rc::Rc;

use support::LiteralEngine;

use pgsheets_core::CellAddress;
use pgsheets_sheet::Workbook;

/// A workbook over the literal-only engine.
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
fn only_populated_cells_are_stored() {
    let mut workbook = sheet();

    assert_eq!(workbook.cell_count(), 0);

    set(&mut workbook, "A1", "1");
    set(&mut workbook, "Z100", "2");

    assert_eq!(workbook.cell_count(), 2);

    set(&mut workbook, "A1", "");

    assert_eq!(workbook.cell_count(), 1);
}

#[test]
fn raw_input_and_display_value_are_kept_apart() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "1200");

    let address = at("A1");

    assert_eq!(workbook.raw_input(address.row, address.column), "1200");
    assert_eq!(display(&workbook, "A1"), "1200");
    assert!(!workbook.cell(address.row, address.column).unwrap().is_formula());
}

#[test]
fn a_cell_never_written_is_absent_and_empty() {
    let workbook = sheet();

    assert!(workbook.cell(5, 5).is_none());
    assert_eq!(workbook.display_value(5, 5), "");
    assert_eq!(workbook.raw_input(5, 5), "");
    assert!(workbook.value(5, 5).is_empty());
    assert_eq!(workbook.number_format(5, 5), None);
}

#[test]
fn bulk_update_defers_change_notifications_until_the_end() {
    let mut workbook = sheet();

    let notifications = Rc::new(Cell::new(0_u32));
    let counter = Rc::clone(&notifications);
    workbook.on_changed(move || counter.set(counter.get() + 1));

    workbook.begin_bulk_update();

    set(&mut workbook, "A1", "2");
    set(&mut workbook, "A2", "3");

    assert_eq!(notifications.get(), 0);

    workbook.end_bulk_update();

    assert_eq!(display(&workbook, "A1"), "2");
    assert_eq!(display(&workbook, "A2"), "3");
    assert_eq!(notifications.get(), 1);
}

#[test]
fn every_edit_outside_a_bulk_update_notifies() {
    let mut workbook = sheet();

    let notifications = Rc::new(Cell::new(0_u32));
    let counter = Rc::clone(&notifications);
    workbook.on_changed(move || counter.set(counter.get() + 1));

    set(&mut workbook, "A1", "1");
    set(&mut workbook, "B1", "2");

    assert_eq!(notifications.get(), 2);
}

#[test]
fn clearing_removes_everything() {
    let mut workbook = sheet();

    set(&mut workbook, "A1", "1");
    set(&mut workbook, "B2", "2");

    workbook.clear();

    assert_eq!(workbook.cell_count(), 0);
    assert_eq!(display(&workbook, "B2"), "");
}

#[test]
fn a_removed_cell_is_not_resurrected_by_the_recalculation_that_follows() {
    let mut workbook = sheet();

    set(&mut workbook, "A1", "1");
    set(&mut workbook, "A1", "");

    // The engine is told about the removal and its write-back is ignored, which is what keeps the
    // sheet sparse: the recalc that follows a delete must not put the cell back.
    assert_eq!(workbook.cell_count(), 0);
    assert!(workbook.cell(0, 0).is_none());
}

#[test]
fn a_change_notification_advances_the_revision() {
    let mut workbook = sheet();
    let before = workbook.revision();

    set(&mut workbook, "A1", "1");

    assert!(workbook.revision() > before);
}

#[test]
fn text_that_looks_like_a_number_keeps_the_value_it_was_given() {
    let mut workbook = sheet();

    set(&mut workbook, "A1", "007");

    // Nothing is normalised on the way in: the raw input is exactly what was typed, and the value is
    // the number it parses to.
    assert_eq!(workbook.raw_input(0, 0), "007");
    assert_eq!(workbook.value(0, 0).as_number(), Some(7.0));
    assert_eq!(display(&workbook, "A1"), "7");
}

#[test]
fn address_round_trips_through_a1_notation() {
    assert_eq!(CellAddress::to_a1(0, 0), "A1");
    assert_eq!(CellAddress::to_a1(6, 1), "B7");
    assert_eq!(CellAddress::to_a1(0, 26), "AA1");
    assert_eq!(
        CellAddress::to_a1(pgsheets_core::MAX_ROWS - 1, pgsheets_core::MAX_COLUMNS - 1),
        "XFD1048576"
    );

    assert_eq!(CellAddress::try_parse_a1("$B$7"), Some((6, 1)));
    assert_eq!(CellAddress::try_parse_a1("A0"), None);
    assert_eq!(CellAddress::try_parse_a1("1A"), None);
}
