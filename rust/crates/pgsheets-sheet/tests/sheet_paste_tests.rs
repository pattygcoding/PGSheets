//! Pasting a block into the sheet: how it tiles, what it clears, and that a formula follows the copy.

mod support;

use pgsheets_core::store::ImportSink;
use pgsheets_core::{CellAddress, CellBuffer, CellRange};
use pgsheets_formulas::Engine;
use pgsheets_sheet::{Workbook, paste_into, shift_for};
use support::LiteralEngine;

/// A workbook over the real engine: a paste has to rewrite formulas, so the engine has to be real.
fn sheet() -> Workbook {
    Workbook::new(Box::new(Engine::new()))
}

/// A workbook over the literal-only engine, for the cases that carry no formula.
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

/// A block from A1-relative addresses.
fn block(cells: &[(&str, &str)]) -> CellBuffer {
    let mut buffer = CellBuffer::new();

    for (a1, value) in cells {
        let address = at(a1);
        buffer.set_raw_input(address.row, address.column, value);
    }

    buffer
}

#[test]
fn a_shifted_paste_moves_relative_references() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "2");
    let block = block(&[("A1", "=A1")]);

    paste_into(&block, &mut workbook, CellRange::single(4, 0), 4, 0);

    assert_eq!(workbook.raw_input(4, 0), "=A5");
}

#[test]
fn a_shifted_paste_leaves_anchored_references_alone() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "2");
    let block = block(&[("A1", "=$A$1")]);

    paste_into(&block, &mut workbook, CellRange::single(4, 0), 4, 0);

    assert_eq!(workbook.raw_input(4, 0), "=$A$1");
}

#[test]
fn shift_for_is_the_distance_between_origins() {
    for (from, to, expected) in [
        (CellRange::single(0, 1), CellRange::single(5, 1), (5, 0)),
        (CellRange::single(0, 0), CellRange::single(0, 0), (0, 0)),
        (CellRange::single(2, 3), CellRange::single(0, 0), (-2, -3)),
        (CellRange::single(1, 1), CellRange::single(3, 4), (2, 3)),
    ] {
        assert_eq!(shift_for(from, to), expected);
    }
}

#[test]
fn writes_a_block_at_the_target_corner() {
    let mut workbook = literal_sheet();
    let block = block(&[("A1", "a"), ("B1", "b"), ("A2", "c"), ("B2", "d")]);

    let (written, covered) = paste_into(&block, &mut workbook, CellRange::single(2, 2), 0, 0);

    assert_eq!(written, 4);
    assert_eq!(covered, CellRange::new(2, 2, 3, 3));
    assert_eq!(display(&workbook, "C3"), "a");
    assert_eq!(display(&workbook, "D4"), "d");
}

#[test]
fn a_single_cell_fills_the_whole_selection() {
    let mut workbook = literal_sheet();
    let block = block(&[("A1", "x")]);

    let (written, covered) = paste_into(&block, &mut workbook, CellRange::new(0, 0, 2, 2), 0, 0);

    assert_eq!(written, 9);
    assert_eq!(covered, CellRange::new(0, 0, 2, 2));
    assert_eq!(display(&workbook, "C3"), "x");
}

#[test]
fn a_block_is_tiled_to_cover_a_larger_selection() {
    let mut workbook = literal_sheet();
    let block = block(&[("A1", "1"), ("B1", "2")]);

    let (written, covered) = paste_into(&block, &mut workbook, CellRange::new(0, 0, 3, 1), 0, 0);

    assert_eq!(written, 8);
    assert_eq!(covered, CellRange::new(0, 0, 3, 1));
    assert_eq!(display(&workbook, "A4"), "1");
    assert_eq!(display(&workbook, "B4"), "2");
}

#[test]
fn an_empty_field_clears_the_destination() {
    let mut workbook = literal_sheet();
    set(&mut workbook, "A1", "old");

    let block = block(&[("A1", ""), ("B1", "")]);

    paste_into(&block, &mut workbook, CellRange::new(0, 0, 0, 1), 0, 0);

    assert_eq!(display(&workbook, "A1"), "");
    assert_eq!(workbook.cell_count(), 0);
}

#[test]
fn the_paste_is_a_single_undo_step() {
    let mut workbook = literal_sheet();
    let block = block(&[("A1", "a"), ("B1", "b")]);

    paste_into(&block, &mut workbook, CellRange::new(0, 0, 0, 1), 0, 0);

    assert_eq!(workbook.undo_depth(), 1);

    workbook.undo();

    assert_eq!(workbook.cell_count(), 0);
}

#[test]
fn formulae_paste_as_formulae() {
    let mut workbook = sheet();
    set(&mut workbook, "A1", "2");
    let block = block(&[("A2", "=A1*3")]);

    paste_into(&block, &mut workbook, CellRange::single(1, 0), 0, 0);

    assert_eq!(display(&workbook, "A2"), "6");
    assert_eq!(workbook.raw_input(1, 0), "=A1*3");
}

#[test]
fn an_empty_block_writes_nothing() {
    let mut workbook = literal_sheet();

    let (written, covered) = paste_into(&CellBuffer::new(), &mut workbook, CellRange::single(1, 1), 0, 0);

    assert_eq!(written, 0);
    assert_eq!(covered, CellRange::single(1, 1));
    assert!(!workbook.can_undo());
}

#[test]
fn writes_stay_inside_the_sheet() {
    let mut workbook = literal_sheet();
    let block = block(&[("A1", "a"), ("B1", "b")]);

    // The target's second column is already the last column in the sheet.
    let last_column = pgsheets_core::MAX_COLUMNS - 1;

    let (_, covered) = paste_into(&block, &mut workbook, CellRange::single(0, last_column), 0, 0);

    assert_eq!(covered.column2, last_column);
    assert_eq!(display(&workbook, "XFD1"), "a");
}
