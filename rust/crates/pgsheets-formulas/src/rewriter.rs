//! Adjusts the references inside a formula for a structural change: a copy, a paste, or rows and
//! columns being inserted or removed.
//!
//! A formula that cannot be parsed, or that contains something the writer cannot render, is returned
//! unchanged. That is deliberate: leaving a formula exactly as it was is always safer than replacing
//! it with something subtly different.

use pgsheets_core::{CellReference, MAX_COLUMNS, MAX_ROWS};

use crate::parser;
use crate::writer;

/// Shifts every *relative* reference by a delta and leaves anchored parts alone.
///
/// This is what copying, pasting or filling a formula does.
pub fn translate(formula: &str, row_delta: i32, column_delta: i32) -> String {
    if row_delta == 0 && column_delta == 0 {
        return formula.to_string();
    }

    rewrite(formula, &|reference| {
        let row = if reference.absolute_row {
            reference.row
        } else {
            reference.row + row_delta
        };

        let column = if reference.absolute_column {
            reference.column
        } else {
            reference.column + column_delta
        };

        on_sheet(row, column).then_some(CellReference {
            row,
            column,
            ..reference
        })
    })
}

/// Adjusts references after rows are inserted at `at` or removed from it.
///
/// Anchors do not protect a reference here, which matches a spreadsheet: inserting a row above
/// `$A$5` moves it to `$A$6`.
pub fn adjust_for_rows(formula: &str, at: i32, count: i32, inserted: bool) -> String {
    rewrite(formula, &|reference| {
        let mut moved = reference;
        moved.row = shift(reference.row, at, count, inserted, true)?;
        Some(moved)
    })
}

/// Adjusts references after columns are inserted or removed.
pub fn adjust_for_columns(formula: &str, at: i32, count: i32, inserted: bool) -> String {
    rewrite(formula, &|reference| {
        let mut moved = reference;
        moved.column = shift(reference.column, at, count, inserted, false)?;
        Some(moved)
    })
}

/// Rewrites a formula through the writer, or returns it untouched when it cannot be read.
fn rewrite(formula: &str, rewriter: &writer::ReferenceRewrite<'_>) -> String {
    if !formula.starts_with('=') {
        return formula.to_string();
    }

    let Ok(node) = parser::parse(formula) else {
        return formula.to_string();
    };

    writer::write(&node, Some(rewriter))
}

/// The new position of a reference, or `None` when the line it pointed at is gone.
fn shift(position: i32, at: i32, count: i32, inserted: bool, rows: bool) -> Option<i32> {
    let mut position = position;

    if inserted {
        if position >= at {
            position += count;
        }
    } else {
        // The reference sits inside the band that was removed.
        if (at..at + count).contains(&position) {
            return None;
        }

        if position >= at + count {
            position -= count;
        }
    }

    let limit = if rows { MAX_ROWS } else { MAX_COLUMNS };

    if !(0..limit).contains(&position) {
        return None;
    }

    Some(position)
}

fn on_sheet(row: i32, column: i32) -> bool {
    (0..MAX_ROWS).contains(&row) && (0..MAX_COLUMNS).contains(&column)
}
