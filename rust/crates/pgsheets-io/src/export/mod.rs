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

/// A named worksheet as an exporter sees it.
///
/// A document is a list of these, so a writer that can hold several sheets — xlsx — receives them all,
/// while one that cannot — delimited text — writes the single sheet the options name.
#[derive(Clone, Copy)]
pub struct SheetRef<'a> {
    /// The name shown on the tab.
    pub name: &'a str,

    /// The cells of the sheet.
    pub source: &'a dyn ExportSource,
}

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

    /// Writes a whole document: one or more named worksheets.
    ///
    /// The default writes the single sheet whose name matches [`ExportOptions::sheet_name`] — or the
    /// first, when none does — through [`SpreadsheetExporter::export`]. That is all a single-table
    /// format such as CSV can hold; a format that can carry several sheets overrides this rather than
    /// the single-sheet method.
    fn export_workbook(
        &self,
        writer: &mut dyn WriteSeek,
        file_name: &str,
        sheets: &[SheetRef<'_>],
        options: &ExportOptions,
    ) -> Result<ExportResult, ExportError> {
        let chosen = sheets
            .iter()
            .find(|sheet| sheet.name == options.sheet_name)
            .or_else(|| sheets.first());

        match chosen {
            Some(sheet) => self.export(writer, file_name, sheet.source, options),
            None => Err(ExportError::Io("the document holds no worksheets".to_string())),
        }
    }
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

    /// Writes a whole document, or `None` when no writer claims the file name.
    pub fn export_workbook(
        &self,
        file_name: &str,
        writer: &mut dyn WriteSeek,
        sheets: &[SheetRef<'_>],
        options: &ExportOptions,
    ) -> Option<Result<ExportResult, ExportError>> {
        self.resolve(file_name)
            .map(|exporter| exporter.export_workbook(writer, file_name, sheets, options))
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
    /// Every worksheet is read and every worksheet is written, so a workbook of many sheets saves in
    /// place as readily as one of a single sheet. What still sends a save to Save As is a sheet this
    /// app cannot represent — a chartsheet or a macro sheet, counted as `dropped_sheets` by the
    /// importer — because writing over the file would destroy it. The same rule sends an `.xlsm` save
    /// to Save As, since a VBA project cannot be produced.
    pub fn can_save_in_place(dropped_sheets: i32) -> bool {
        dropped_sheets <= 0
    }

    /// What a save leaves behind, or `None` when nothing is lost.
    ///
    /// The caller surfaces this as a warning, so the loss is stated rather than discovered later.
    pub fn describe_lost_sheets(dropped_sheets: i32) -> Option<String> {
        if dropped_sheets <= 0 {
            return None;
        }

        Some(format!(
            "{dropped_sheets} sheet(s) in the file that was opened are not worksheets and were not \
             preserved."
        ))
    }
}
