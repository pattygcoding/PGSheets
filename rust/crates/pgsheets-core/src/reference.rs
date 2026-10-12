//! A single cell reference as it appears in a formula, including its `$` anchors.

use crate::address::CellAddress;

/// A cell reference with the anchors that decide how a copy treats it.
///
/// The anchors are what make a reference behave correctly when a formula is copied: a relative part
/// shifts with the copy, an anchored one stays put. Inserting or deleting rows shifts every
/// reference regardless of its anchors, which is also how a spreadsheet behaves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellReference {
    /// Zero-based row of the reference.
    pub row: i32,

    /// Zero-based column of the reference.
    pub column: i32,

    /// True when the row was written as `$7`.
    pub absolute_row: bool,

    /// True when the column was written as `$B`.
    pub absolute_column: bool,
}

impl CellReference {
    /// An unanchored reference, which is what a plain `A1` means.
    pub const fn relative(row: i32, column: i32) -> Self {
        Self {
            row,
            column,
            absolute_row: false,
            absolute_column: false,
        }
    }

    /// Formats as `B7`, `$B$7`, `B$7` or `$B7`.
    ///
    /// # Panics
    ///
    /// Panics for a negative row or column, which has no A1 form.
    pub fn to_a1(self) -> String {
        let mut text = String::with_capacity(8);

        if self.absolute_column {
            text.push('$');
        }

        text.push_str(&CellAddress::column_name(self.column));

        if self.absolute_row {
            text.push('$');
        }

        text.push_str(&(self.row + 1).to_string());
        text
    }
}

impl core::fmt::Display for CellReference {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(&self.to_a1())
    }
}
