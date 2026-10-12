//! Collects imported cells in memory, keeping the rectangular shape of the source.
//!
//! Used as the target of a clipboard paste: a sink that discarded empty fields would lose the
//! alignment, and a pasted block has to line up with a selection.

use std::collections::HashMap;

use crate::range::CellRange;
use crate::store::ImportSink;

/// A rectangular block of raw values, including the empty ones.
#[derive(Default)]
pub struct CellBuffer {
    cells: HashMap<(i32, i32), String>,
}

impl CellBuffer {
    /// Creates an empty buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Every field of the block, including empty ones, in no particular order.
    pub fn cells(&self) -> impl Iterator<Item = (i32, i32, &str)> {
        self.cells
            .iter()
            .map(|((row, column), value)| (*row, *column, value.as_str()))
    }

    /// Number of fields, including empty ones.
    pub fn count(&self) -> usize {
        self.cells.len()
    }

    /// Number of fields that hold content.
    pub fn non_empty_count(&self) -> usize {
        self.cells.values().filter(|value| !value.is_empty()).count()
    }

    /// The value of a field; empty when the field was never set.
    pub fn get(&self, row: i32, column: i32) -> &str {
        self.cells.get(&(row, column)).map(String::as_str).unwrap_or("")
    }

    /// The smallest rectangle covering the block; `None` when the buffer is empty.
    pub fn try_get_bounds(&self) -> Option<CellRange> {
        let mut min_row = i32::MAX;
        let mut min_column = i32::MAX;
        let mut max_row = i32::MIN;
        let mut max_column = i32::MIN;

        for (row, column) in self.cells.keys() {
            min_row = min_row.min(*row);
            max_row = max_row.max(*row);
            min_column = min_column.min(*column);
            max_column = max_column.max(*column);
        }

        if min_row > max_row {
            return None;
        }

        Some(CellRange::new(min_row, min_column, max_row, max_column))
    }
}

impl ImportSink for CellBuffer {
    /// Records a field, including empty ones, so the block keeps its shape.
    fn set_raw_input(&mut self, row: i32, column: i32, raw_input: &str) {
        self.cells.insert((row, column), raw_input.to_string());
    }

    /// Ignored: a tab-separated clipboard carries values, not formats. Pasting text should not
    /// silently reformat the cells it lands on.
    fn set_number_format(&mut self, _row: i32, _column: i32, _number_format: Option<&str>) {}
}
