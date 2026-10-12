//! What an export reports back, and the settings it runs with.

use pgsheets_core::CellRange;

use crate::capped_warnings;

/// Outcome of an export: the extent written, plus any non-fatal problems encountered.
#[derive(Debug)]
pub struct ExportResult {
    /// Number of rows in the written extent, counted from one.
    pub rows: i32,

    /// Number of columns in the written extent.
    pub columns: i32,

    /// How many non-empty cells were written.
    pub cells_written: i32,

    /// Human-readable warnings: lossy conversions, renamed sheets, dropped content.
    pub warnings: Vec<String>,
}

impl Default for ExportResult {
    fn default() -> Self {
        Self::new()
    }
}

impl ExportResult {
    /// An empty result.
    pub fn new() -> Self {
        Self {
            rows: 0,
            columns: 0,
            cells_written: 0,
            warnings: Vec::new(),
        }
    }

    /// Records a problem, up to the cap that keeps a pathological file from exhausting memory.
    pub fn add_warning(&mut self, message: impl Into<String>) {
        capped_warnings::add_warning(&mut self.warnings, message);
    }
}

/// Caller-supplied settings for a single export run.
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Worksheet name to write. Sanitised by the writer.
    pub sheet_name: String,

    /// When false — the default — each cell's *raw input* is written, so formulas survive a
    /// save/load round trip. When true the computed *display* value is written instead, matching what
    /// the grid shows on screen.
    pub export_values: bool,

    /// Restricts the export to a rectangle. `None` (the default) writes the whole used extent.
    ///
    /// A range keeps its full shape including empty cells, because the caller is exporting a selected
    /// block and it has to line up with that selection.
    pub range: Option<CellRange>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            sheet_name: "Sheet1".to_string(),
            export_values: false,
            range: None,
        }
    }
}

impl ExportOptions {
    /// Options for a whole-sheet export of raw input.
    pub fn new() -> Self {
        Self::default()
    }

    /// Options for a file name whose sheet name should follow the file's.
    pub fn named(sheet_name: impl Into<String>) -> Self {
        Self {
            sheet_name: sheet_name.into(),
            ..Self::default()
        }
    }

    /// Restricts the export to a rectangle.
    pub fn within(mut self, range: CellRange) -> Self {
        self.range = Some(range);
        self
    }

    /// Writes computed values rather than raw input.
    pub fn with_values(mut self) -> Self {
        self.export_values = true;
        self
    }
}
