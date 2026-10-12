//! A rectangular block of cells, always normalised so its corners are ordered.

use crate::address::CellAddress;

/// A rectangular block of cells. Always normalised so `row1 <= row2` and `column1 <= column2`; a
/// single cell has equal corners.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellRange {
    /// Top row of the block.
    pub row1: i32,

    /// Left column of the block.
    pub column1: i32,

    /// Bottom row of the block.
    pub row2: i32,

    /// Right column of the block.
    pub column2: i32,
}

impl CellRange {
    /// Creates a normalised range from any two corners, given in any order.
    pub fn new(row1: i32, column1: i32, row2: i32, column2: i32) -> Self {
        Self {
            row1: row1.min(row2),
            row2: row1.max(row2),
            column1: column1.min(column2),
            column2: column1.max(column2),
        }
    }

    /// Creates a single-cell range.
    pub const fn single(row: i32, column: i32) -> Self {
        Self {
            row1: row,
            column1: column,
            row2: row,
            column2: column,
        }
    }

    /// Number of rows covered.
    pub fn row_count(self) -> i32 {
        self.row2 - self.row1 + 1
    }

    /// Number of columns covered.
    pub fn column_count(self) -> i32 {
        self.column2 - self.column1 + 1
    }

    /// Number of cells covered.
    pub fn cell_count(self) -> i32 {
        self.row_count() * self.column_count()
    }

    /// True when the range covers exactly one cell.
    pub fn is_single_cell(self) -> bool {
        self.row1 == self.row2 && self.column1 == self.column2
    }

    /// True when the coordinate falls inside the range.
    pub fn contains(self, row: i32, column: i32) -> bool {
        row >= self.row1 && row <= self.row2 && column >= self.column1 && column <= self.column2
    }

    /// Shifts the whole range by a delta, for tiling a paste across a larger selection.
    pub fn offset(self, row_delta: i32, column_delta: i32) -> Self {
        Self {
            row1: self.row1 + row_delta,
            column1: self.column1 + column_delta,
            row2: self.row2 + row_delta,
            column2: self.column2 + column_delta,
        }
    }

    /// Every cell in the range, row by row.
    pub fn iter(self) -> impl Iterator<Item = CellAddress> {
        (self.row1..=self.row2).flat_map(move |row| {
            (self.column1..=self.column2).map(move |column| CellAddress::new(row, column))
        })
    }
}

impl core::fmt::Display for CellRange {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_single_cell() {
            formatter.write_str(&CellAddress::to_a1(self.row1, self.column1))
        } else {
            write!(
                formatter,
                "{}:{}",
                CellAddress::to_a1(self.row1, self.column1),
                CellAddress::to_a1(self.row2, self.column2)
            )
        }
    }
}
