//! Lookup functions.
//!
//! A failed match is `#N/A` — which is what makes `IFERROR` and `IFNA` worth wrapping around a lookup
//! — and an index outside the table is `#REF!`.

use pgsheets_core::{CellError, CellValue};

use super::helpers::{compare_values, optional_integer, try_integer};
use super::wildcards::wildcard_match;
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

/// A range read as a single row or column, so the lookup functions share one walk.
///
/// `XLOOKUP` and `XMATCH` are one-dimensional by definition — a position only exists along a single
/// row or column — so a range that is both taller and wider than one cell has no answer.
#[derive(Clone, Copy)]
struct Vector {
    row1: i32,
    column1: i32,
    vertical: bool,
    length: i32,
}

impl Vector {
    /// Reads a range as a vector, or `None` when it is a block rather than a line.
    fn of(node: &AstNode) -> Option<Self> {
        let AstNode::Range(range) = node else {
            return None;
        };

        let vertical = range.row_count() > 1;

        if vertical && range.column_count() > 1 {
            return None;
        }

        Some(Self {
            row1: range.row1(),
            column1: range.column1(),
            vertical,
            length: if vertical {
                range.row_count()
            } else {
                range.column_count()
            },
        })
    }

    /// The value at an offset along the vector.
    fn value(&self, offset: i32, context: &mut dyn FormulaContext) -> CellValue {
        let (row, column) = if self.vertical {
            (self.row1 + offset, self.column1)
        } else {
            (self.row1, self.column1 + offset)
        };

        context.cell_value(row, column)
    }
}

/// Why a key could not be positioned.
enum SeekOutcome {
    /// The arguments were wrong; the value is the error to answer with.
    Error(CellValue),

    /// The key was not found and there is no fallback.
    NotFound,
}

/// `LOOKUP(value, lookup_vector, [result_vector])` — the approximate lookup that assumes its vector is
/// sorted ascending and returns the matching entry, or the last one at or below the key.
pub(super) fn look_up(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() < 2 || arguments.len() > 3 {
        return CellValue::error(CellError::Value);
    }

    let Some(lookup) = Vector::of(&arguments[1]) else {
        return CellValue::error(CellError::Value);
    };

    // Without a result vector the value is read from the lookup vector itself.
    let result = if arguments.len() == 3 {
        match Vector::of(&arguments[2]) {
            Some(vector) if vector.length == lookup.length => vector,
            _ => return CellValue::error(CellError::Value),
        }
    } else {
        lookup
    };

    let key = arguments[0].evaluate(context);

    if key.is_error() {
        return key;
    }

    let mut found: Option<i32> = None;

    for offset in 0..lookup.length {
        let probe = lookup.value(offset, context);

        if probe.is_error() {
            return probe;
        }

        let comparison = compare_values(&probe, &key);

        if comparison.is_eq() {
            return result.value(offset, context);
        }

        if comparison.is_lt() {
            // Below the key, so a candidate; the table is assumed sorted, so an entry above it ends
            // the search.
            found = Some(offset);
        } else {
            break;
        }
    }

    match found {
        Some(offset) => result.value(offset, context),
        None => CellValue::error(CellError::NotAvailable),
    }
}

/// `XLOOKUP(key, lookup_array, return_array, [if_not_found], [match_mode], [search_mode])`.
pub(super) fn x_look_up(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() < 3 || arguments.len() > 6 {
        return CellValue::error(CellError::Value);
    }

    let match_mode = match optional_integer(arguments, 4, context, 0) {
        Ok(mode) => mode,
        Err(error) => return error,
    };

    let search_mode = match optional_integer(arguments, 5, context, 1) {
        Ok(mode) => mode,
        Err(error) => return error,
    };

    let (Some(lookup), Some(returned)) = (Vector::of(&arguments[1]), Vector::of(&arguments[2])) else {
        return CellValue::error(CellError::Value);
    };

    if lookup.length != returned.length {
        return CellValue::error(CellError::Value);
    }

    match seek(&arguments[0], &lookup, context, match_mode, search_mode) {
        Ok(position) => returned.value(position, context),
        Err(SeekOutcome::Error(error)) => error,
        Err(SeekOutcome::NotFound) => match arguments.get(3) {
            Some(fallback) => fallback.evaluate(context),
            None => CellValue::error(CellError::NotAvailable),
        },
    }
}

/// `XMATCH(key, lookup_array, [match_mode], [search_mode])` — the 1-based position, or `#N/A`.
pub(super) fn x_match(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() < 2 || arguments.len() > 4 {
        return CellValue::error(CellError::Value);
    }

    let match_mode = match optional_integer(arguments, 2, context, 0) {
        Ok(mode) => mode,
        Err(error) => return error,
    };

    let search_mode = match optional_integer(arguments, 3, context, 1) {
        Ok(mode) => mode,
        Err(error) => return error,
    };

    let Some(lookup) = Vector::of(&arguments[1]) else {
        return CellValue::error(CellError::Value);
    };

    match seek(&arguments[0], &lookup, context, match_mode, search_mode) {
        Ok(position) => CellValue::number(f64::from(position + 1)),
        Err(SeekOutcome::Error(error)) => error,
        Err(SeekOutcome::NotFound) => CellValue::error(CellError::NotAvailable),
    }
}

/// The offset of a key within a vector, honouring the match and search modes.
///
/// `match_mode` is `0` for an exact match, `-1` for an exact match or the next smaller value, `1` for
/// an exact match or the next larger value, and `2` to read the key as a wildcard pattern.
/// `search_mode` scans forwards when positive and backwards when negative.
fn seek(
    key_node: &AstNode,
    lookup: &Vector,
    context: &mut dyn FormulaContext,
    match_mode: i32,
    search_mode: i32,
) -> Result<i32, SeekOutcome> {
    let key = key_node.evaluate(context);

    if key.is_error() {
        return Err(SeekOutcome::Error(key));
    }

    let reverse = search_mode < 0;
    let mut best: Option<i32> = None;

    for step in 0..lookup.length {
        let offset = if reverse { lookup.length - 1 - step } else { step };

        let probe = lookup.value(offset, context);

        if probe.is_error() {
            return Err(SeekOutcome::Error(probe));
        }

        if match_mode == 2 {
            if wildcard_match(&probe.as_text(), &key.as_text()) {
                return Ok(offset);
            }

            continue;
        }

        let comparison = compare_values(&probe, &key);

        if comparison.is_eq() {
            return Ok(offset);
        }

        // Without an exact match the answer is the nearest value on the requested side; comparing the
        // candidates by value keeps that true whichever way the scan runs.
        let qualifies = if match_mode == -1 {
            comparison.is_lt()
        } else if match_mode == 1 {
            comparison.is_gt()
        } else {
            false
        };

        if qualifies {
            best = Some(match best {
                None => offset,
                Some(current) => {
                    let current_value = lookup.value(current, context);
                    let better = if match_mode == -1 {
                        compare_values(&probe, &current_value).is_gt()
                    } else {
                        compare_values(&probe, &current_value).is_lt()
                    };

                    if better { offset } else { current }
                }
            });
        }
    }

    best.ok_or(SeekOutcome::NotFound)
}

/// `ROWS(array)` — how many rows a range, or a single cell, covers.
pub(super) fn rows(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    dimensions(arguments, context, true)
}

/// `COLUMNS(array)` — how many columns a range, or a single cell, covers.
pub(super) fn columns(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    dimensions(arguments, context, false)
}

fn dimensions(arguments: &[AstNode], context: &mut dyn FormulaContext, rows: bool) -> CellValue {
    if arguments.len() != 1 {
        return CellValue::error(CellError::Value);
    }

    match &arguments[0] {
        AstNode::Range(range) => {
            let count = if rows {
                range.row_count()
            } else {
                range.column_count()
            };

            CellValue::number(f64::from(count))
        }
        AstNode::Cell(_) => CellValue::number(1.0),
        other => {
            // A scalar is not an array, but an error passed in is still worth reporting.
            let value = other.evaluate(context);

            if value.is_error() {
                value
            } else {
                CellValue::error(CellError::Value)
            }
        }
    }
}
