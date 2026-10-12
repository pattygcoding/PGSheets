//! The three narrow contracts that keep the model from leaking: what the engine may touch, what an
//! importer may write, and what an exporter may read.

use crate::address::CellAddress;
use crate::style::CellStyle;
use crate::value::CellValue;

/// The minimal view of cell storage the formula engine needs.
///
/// Keeping this separate from the model lets the engine stay free of UI and file concerns, and makes
/// it trivially testable with an in-memory fake.
pub trait CellStore {
    /// Raw, unparsed input for a cell; empty when the cell is not populated.
    fn raw_input(&self, row: i32, column: i32) -> &str;

    /// The last computed value for a cell; [`CellValue::Empty`] when unknown.
    fn value(&self, row: i32, column: i32) -> CellValue;

    /// Stores an evaluated result.
    ///
    /// Implementations must ignore cells that do not exist, so that removing a cell is not undone by
    /// the recalculation that follows it.
    fn set_computed(&mut self, row: i32, column: i32, value: CellValue);

    /// Every populated cell, used for a full rebuild after a bulk import.
    fn populated_cells(&self) -> Vec<CellAddress>;
}

/// Sink used by importers, so they can push data into the model without depending on the UI or on the
/// model's concrete type.
pub trait ImportSink {
    /// Stores the raw input of a cell.
    fn set_raw_input(&mut self, row: i32, column: i32, raw_input: &str);

    /// Stores the number format that applies to a cell, as read from the file alongside its value.
    ///
    /// A separate call because the format arrives with the style rather than with the content, and
    /// most sinks have no use for it.
    fn set_number_format(&mut self, row: i32, column: i32, number_format: Option<&str>);

    /// Stores the styling that applies to a cell, as read from the file alongside its value.
    ///
    /// Like [`ImportSink::set_number_format`], a separate call because the style arrives with the
    /// cell's formatting rather than with its content, and most sinks have no use for it.
    fn set_cell_style(&mut self, row: i32, column: i32, style: Option<&CellStyle>);
}

/// The minimal read-only view of cell storage that exporters need.
///
/// Kept separate from [`CellStore`] so a writer cannot write back into the workbook, and so exporters
/// stay unit-testable with an in-memory fake.
pub trait ExportSource {
    /// Every populated cell, in no particular order.
    fn populated_cells(&self) -> Vec<CellAddress>;

    /// Raw, unparsed input for a cell; empty when the cell is not populated.
    fn raw_input(&self, row: i32, column: i32) -> &str;

    /// The last computed value for a cell.
    fn value(&self, row: i32, column: i32) -> CellValue;

    /// The number format code of a cell, or `None` when it has none.
    ///
    /// A format changes only what a cell shows, so a writer needs it to keep the grid's appearance
    /// rather than just its values.
    fn number_format(&self, row: i32, column: i32) -> Option<&str>;

    /// The styling of a cell, or `None` when it has none.
    ///
    /// Like the number format, a style changes only what a cell looks like, so a writer needs it to
    /// keep the grid's appearance rather than just its values.
    fn cell_style(&self, row: i32, column: i32) -> Option<CellStyle>;
}
