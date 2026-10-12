//! Writing a spreadsheet to a file.
//!
//! A writer reads from an [`ExportSource`] and never touches the UI, which keeps both formats
//! unit-testable without a window.

pub mod csv;
mod result;
mod xlsx;

use pgsheets_core::ExportSource;

use crate::zip::WriteSeek;

pub use csv::CsvExporter;
pub use result::{ExportOptions, ExportResult};
pub use xlsx::XlsxWriter;

use super::import::extension_of;

/// Why an export could not be written.
#[derive(Debug)]
pub enum ExportError {
    /// The caller asked for a format no writer claims.
    Unsupported(String),

    /// The file could not be written.
    Io(String),
}

impl core::fmt::Display for ExportError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unsupported(name) => write!(formatter, "no writer is registered for '{name}'"),
            Self::Io(message) => write!(formatter, "the file could not be written: {message}"),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<std::io::Error> for ExportError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

/// A file format writer.
///
/// Unlike an importer the file name is passed in, because a writer cannot sniff the format it has been
/// asked to produce: `.csv` and `.tsv` share one implementation and differ only by the extension the
/// user chose.
pub trait SpreadsheetExporter {
    /// Lower-case extensions this exporter claims, e.g. `".csv"`.
    fn extensions(&self) -> &'static [&'static str];

    /// True when this exporter claims a file name.
    fn can_export(&self, file_name: &str) -> bool {
        let Some(extension) = extension_of(file_name) else {
            return false;
        };

        self.extensions().iter().any(|claimed| *claimed == extension)
    }

    /// Writes the document.
    fn export(
        &self,
        writer: &mut dyn WriteSeek,
        file_name: &str,
        source: &dyn ExportSource,
        options: &ExportOptions,
    ) -> Result<ExportResult, ExportError>;
}

/// Chooses a writer for a file name.
///
/// Registering another format is one line here, which grows the save story without the window knowing
/// about it.
pub struct SheetExporter {
    writers: Vec<Box<dyn SpreadsheetExporter>>,
}

impl Default for SheetExporter {
    fn default() -> Self {
        Self::standard()
    }
}

impl SheetExporter {
    /// The formats this application writes.
    pub fn standard() -> Self {
        Self {
            writers: vec![Box::new(XlsxWriter::new()), Box::new(CsvExporter::new())],
        }
    }

    /// Format used when the user has not chosen one yet.
    pub const DEFAULT_EXTENSION: &'static str = ".xlsx";

    /// The writer that claims a file name, or `None`.
    pub fn resolve(&self, file_name: &str) -> Option<&dyn SpreadsheetExporter> {
        self.writers
            .iter()
            .find(|writer| writer.can_export(file_name))
            .map(AsRef::as_ref)
    }

    /// True when some writer claims a file name.
    pub fn can_export(&self, file_name: &str) -> bool {
        self.resolve(file_name).is_some()
    }

    /// Writes a document, or `None` when no writer claims the file name.
    pub fn export(
        &self,
        file_name: &str,
        writer: &mut dyn WriteSeek,
        source: &dyn ExportSource,
        options: &ExportOptions,
    ) -> Option<Result<ExportResult, ExportError>> {
        self.resolve(file_name)
            .map(|exporter| exporter.export(writer, file_name, source, options))
    }

    /// File-picker patterns for every registered writer.
    pub fn patterns(&self) -> Vec<String> {
        let mut patterns: Vec<String> = self
            .writers
            .iter()
            .flat_map(|writer| writer.extensions())
            .map(|extension| format!("*{extension}"))
            .collect();

        patterns.sort();
        patterns.dedup();
        patterns
    }

    /// Whether a document loaded from a file may be written back over it.
    ///
    /// Saving writes a single worksheet and truncates the target, so a workbook that held more than
    /// one worksheet has to go through Save As instead: writing in place would destroy every sheet
    /// that was never loaded. This is the same rule that already sends an `.xlsm` save to Save As,
    /// because a VBA project cannot be produced — a format this app cannot fully write is never
    /// overwritten in place.
    pub fn can_save_in_place(loaded_worksheet_count: i32) -> bool {
        loaded_worksheet_count <= 1
    }

    /// What a single-worksheet save leaves behind, or `None` when nothing is lost.
    ///
    /// The caller surfaces this as a warning, so the loss is stated rather than discovered later.
    pub fn describe_lost_worksheets(loaded_worksheet_count: i32, written_sheet: &str) -> Option<String> {
        if loaded_worksheet_count <= 1 {
            return None;
        }

        Some(format!(
            "only the '{written_sheet}' worksheet was written; the other {} worksheet(s) in the file \
             that was opened are not preserved.",
            loaded_worksheet_count - 1
        ))
    }
}
