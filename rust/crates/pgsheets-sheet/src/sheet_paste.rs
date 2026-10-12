//! Writes a rectangular block of pasted cells into the sheet, repeating it to cover the target the
//! way a spreadsheet paste does.

use pgsheets_core::{CellBuffer, CellRange, MAX_COLUMNS, MAX_ROWS};
use pgsheets_formulas::rewriter;

use crate::workbook::Workbook;

/// Applies a pasted block to the target selection.
///
/// A single copied cell fills the whole selection; a larger block is tiled enough times to cover it.
/// Every write happens inside one bulk update, so a paste is a single undo step.
///
/// `row_delta` and `column_delta` say how far the block has travelled, which is what shifts its
/// relative references. The clipboard carries only text, so the caller supplies them from the region
/// it copied.
///
/// Returns the number of cells written and the range the paste actually covers.
pub fn apply(
    block: &CellBuffer,
    workbook: &mut Workbook,
    target: CellRange,
    row_delta: i32,
    column_delta: i32,
) -> (i32, CellRange) {
    let Some(source) = block.try_get_bounds() else {
        return (0, target);
    };

    let copies_down = (target.row_count() / source.row_count()).max(1);
    let copies_right = (target.column_count() / source.column_count()).max(1);

    let mut written = 0;

    workbook.begin_bulk_update();

    for down in 0..copies_down {
        for right in 0..copies_right {
            for (row, column, value) in block.cells() {
                let destination_row = target.row1 + down * source.row_count() + (row - source.row1);
                let destination_column =
                    target.column1 + right * source.column_count() + (column - source.column1);

                if !(0..MAX_ROWS).contains(&destination_row)
                    || !(0..MAX_COLUMNS).contains(&destination_column)
                {
                    continue;
                }

                workbook.set_raw_input(
                    destination_row,
                    destination_column,
                    &translate(value, row_delta, column_delta),
                );

                written += 1;
            }
        }
    }

    workbook.end_bulk_update();

    let covered = CellRange::new(
        target.row1,
        target.column1,
        (target.row1 + copies_down * source.row_count() - 1).min(MAX_ROWS - 1),
        (target.column1 + copies_right * source.column_count() - 1).min(MAX_COLUMNS - 1),
    );

    (written, covered)
}

/// How far a block has moved when it is pasted: the distance between the two top-left corners.
///
/// A spreadsheet applies exactly this shift to relative references and none to anchored ones.
pub fn shift_for(copied_from: CellRange, target: CellRange) -> (i32, i32) {
    (
        target.row1 - copied_from.row1,
        target.column1 - copied_from.column1,
    )
}

/// A pasted formula keeps its meaning relative to where it lands: relative references shift by the
/// distance moved and anchored ones stay put, which is what a spreadsheet does.
fn translate(raw: &str, row_delta: i32, column_delta: i32) -> String {
    if raw.starts_with('=') {
        rewriter::translate(raw, row_delta, column_delta)
    } else {
        raw.to_string()
    }
}
