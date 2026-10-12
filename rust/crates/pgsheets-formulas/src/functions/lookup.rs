//! Lookup functions.
//!
//! A failed match is `#N/A` — which is what makes `IFERROR` and `IFNA` worth wrapping around a lookup
//! — and an index outside the table is `#REF!`.

use pgsheets_core::{CellError, CellValue};

use super::helpers::{compare_values, try_integer};
use crate::ast::{AstNode, FormulaContext};

pub(super) fn v_look_up(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    lookup(arguments, context, true)
}

pub(super) fn h_look_up(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    lookup(arguments, context, false)
}

/// `VLOOKUP` and `HLOOKUP`.
///
/// The key is searched down the first column, or along the first row, and the result is read `index`
/// columns or rows further across.
fn lookup(arguments: &[AstNode], context: &mut dyn FormulaContext, vertical: bool) -> CellValue {
    if arguments.len() < 3 || arguments.len() > 4 {
        return CellValue::error(CellError::Value);
    }

    // The table must be a range, because its coordinates are what the result is read from.
    let AstNode::Range(range) = &arguments[1] else {
        return CellValue::error(CellError::Value);
    };

    let key = arguments[0].evaluate(context);

    if key.is_error() {
        return key;
    }

    let index = match try_integer(arguments, 2, context) {
        Ok(index) => index,
        Err(error) => return error,
    };

    let approximate = if arguments.len() == 4 {
        let range_lookup = arguments[3].evaluate(context);

        if range_lookup.is_error() {
            return range_lookup;
        }

        range_lookup.as_boolean()
    } else {
        true
    };

    let length = if vertical {
        range.row_count()
    } else {
        range.column_count()
    };

    let width = if vertical {
        range.column_count()
    } else {
        range.row_count()
    };

    if index < 1 || index > width {
        return CellValue::error(CellError::Reference);
    }

    let mut found: Option<i32> = None;

    for offset in 0..length {
        let probe = if vertical {
            context.cell_value(range.row1() + offset, range.column1())
        } else {
            context.cell_value(range.row1(), range.column1() + offset)
        };

        if probe.is_error() {
            return probe;
        }

        let comparison = compare_values(&probe, &key);

        if comparison.is_eq() {
            found = Some(offset);
            break;
        }

        if !approximate {
            // Exact matching keeps scanning; only an identical value counts.
            continue;
        }

        if comparison.is_lt() {
            // Below the key, so a candidate — the table is assumed sorted ascending.
            found = Some(offset);
        } else {
            break;
        }
    }

    let Some(found) = found else {
        return CellValue::error(CellError::NotAvailable);
    };

    if vertical {
        context.cell_value(range.row1() + found, range.column1() + index - 1)
    } else {
        context.cell_value(range.row1() + index - 1, range.column1() + found)
    }
}

/// `MATCH(value, range, [type])` — the 1-based position within a single row or column.
pub(super) fn match_value(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() < 2 || arguments.len() > 3 {
        return CellValue::error(CellError::Value);
    }

    let AstNode::Range(range) = &arguments[1] else {
        return CellValue::error(CellError::Value);
    };

    let key = arguments[0].evaluate(context);

    if key.is_error() {
        return key;
    }

    let match_type = if arguments.len() == 3 {
        match try_integer(arguments, 2, context) {
            Ok(match_type) => match_type,
            Err(error) => return error,
        }
    } else {
        1
    };

    let vertical = range.row_count() > 1;

    if vertical && range.column_count() > 1 {
        // A position only exists along one dimension, so a block has no single answer.
        return CellValue::error(CellError::NotAvailable);
    }

    let length = if vertical {
        range.row_count()
    } else {
        range.column_count()
    };

    let mut found: Option<i32> = None;

    for offset in 0..length {
        let probe = if vertical {
            context.cell_value(range.row1() + offset, range.column1())
        } else {
            context.cell_value(range.row1(), range.column1() + offset)
        };

        if probe.is_error() {
            return probe;
        }

        let comparison = compare_values(&probe, &key);

        if comparison.is_eq() {
            found = Some(offset);
            break;
        }

        if match_type == 0 {
            // Exact matching only.
            continue;
        }

        // Type 1 expects the list sorted ascending and type -1 descending; either way the answer is
        // the last value still on the correct side of the key.
        let correct_side = if match_type > 0 {
            comparison.is_lt()
        } else {
            comparison.is_gt()
        };

        if correct_side {
            found = Some(offset);
        } else {
            break;
        }
    }

    match found {
        Some(offset) => CellValue::number((offset + 1) as f64),
        None => CellValue::error(CellError::NotAvailable),
    }
}

/// `INDEX(range, row, [column])` — 1-based. A single-row range accepts one index, which is then read
/// as the column, matching a spreadsheet.
pub(super) fn index(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() < 2 || arguments.len() > 3 {
        return CellValue::error(CellError::Value);
    }

    let AstNode::Range(range) = &arguments[0] else {
        return CellValue::error(CellError::Value);
    };

    let rows = range.row_count();
    let columns = range.column_count();

    let mut row = match try_integer(arguments, 1, context) {
        Ok(row) => row,
        Err(error) => return error,
    };

    let mut column = 1;

    if arguments.len() == 3 {
        column = match try_integer(arguments, 2, context) {
            Ok(column) => column,
            Err(error) => return error,
        };
    } else if rows == 1 {
        column = row;
        row = 1;
    }

    if row < 1 || row > rows || column < 1 || column > columns {
        return CellValue::error(CellError::Reference);
    }

    context.cell_value(range.row1() + row - 1, range.column1() + column - 1)
}

/// `CHOOSE(index, value, ...)` — only the chosen value is evaluated.
pub(super) fn choose(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() < 2 {
        return CellValue::error(CellError::Value);
    }

    let index = match try_integer(arguments, 0, context) {
        Ok(index) => index,
        Err(error) => return error,
    };

    if index < 1 || index as usize >= arguments.len() {
        return CellValue::error(CellError::Value);
    }

    arguments[index as usize].evaluate(context)
}
