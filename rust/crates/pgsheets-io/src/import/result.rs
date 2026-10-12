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

    /// Name of the sheet or table that was imported, when the format has one. The first worksheet,
    /// for a document that holds several.
    pub sheet_name: Option<String>,

    /// Every worksheet imported, in the order the document listed them.
    pub sheet_names: Vec<String>,

    /// How many non-empty cells were written to the sink.
    pub cells_imported: i32,

    /// How many worksheets the source document held.
    ///
    /// Every worksheet is imported, so this is the number of sheets the document is expected to write
    /// back. What blocks writing over the original is [`ImportResult::dropped_sheets`], not this.
    pub worksheet_count: i32,

    /// How many sheets the document held that are not worksheets — chartsheets, macro sheets and the
    /// like. Such a sheet cannot be represented here, so saving must not write over the file: doing so
    /// would destroy it.
    pub dropped_sheets: i32,
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
            sheet_names: Vec::new(),
            cells_imported: 0,
            worksheet_count: 0,
            dropped_sheets: 0,
        }
    }

    /// Records a problem, up to the cap that keeps a pathological file from exhausting memory.
    pub fn add_warning(&mut self, message: impl Into<String>) {
        capped_warnings::add_warning(&mut self.warnings, message);
    }
}
