//! What an import reports back.

use crate::capped_warnings;

/// Outcome of an import: the used extent, plus any non-fatal problems encountered.
#[derive(Debug)]
pub struct ImportResult {
    /// Number of rows that contained data, counted from one.
    pub rows: i32,

    /// Number of columns that contained data.
    pub columns: i32,

    /// Human-readable warnings: unsupported functions, truncated ranges, and so on.
    pub warnings: Vec<String>,

    /// Name of the sheet or table that was imported, when the format has one.
    pub sheet_name: Option<String>,

    /// How many non-empty cells were written to the sink.
    pub cells_imported: i32,

    /// How many worksheets the source document held.
    ///
    /// Saving writes a single worksheet, so a caller that sees more than one here must not write back
    /// over the original file: it would destroy every sheet that was never loaded.
    pub worksheet_count: i32,
}

impl Default for ImportResult {
    fn default() -> Self {
        Self::new()
    }
}

impl ImportResult {
    /// An empty result for a document that holds a single worksheet, which is the common case.
    pub fn new() -> Self {
        Self {
            rows: 0,
            columns: 0,
            warnings: Vec::new(),
            sheet_name: None,
            cells_imported: 0,
            worksheet_count: 1,
        }
    }

    /// Records a problem, up to the cap that keeps a pathological file from exhausting memory.
    pub fn add_warning(&mut self, message: impl Into<String>) {
        capped_warnings::add_warning(&mut self.warnings, message);
    }
}
