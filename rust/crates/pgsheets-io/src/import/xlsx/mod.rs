//! Reads the SpreadsheetML (xlsx/xlsm) format: a zip archive of XML parts.
//!
//! The first worksheet, the shared string table and just enough of the style table to recognise a
//! number format are read; everything else in the package is ignored.

mod cells;
mod parts;
mod styles;
mod worksheet;

use pgsheets_formulas::FunctionLibrary;

use super::{ImportError, ImportResult, SpreadsheetImporter};
use crate::zip::{Archive, ReadSeek};

/// Reads a workbook.
pub struct XlsxImporter {
    /// When true — the default — formulas are imported as formulas and re-evaluated by the local
    /// engine. When false the cached result stored in the file is imported as a literal instead.
    pub import_formulas: bool,

    /// The function set a formula's calls are checked against, so an unsupported function is reported
    /// as a warning rather than discovered later as `#NAME?`.
    functions: FunctionLibrary,
}

impl Default for XlsxImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl XlsxImporter {
    /// Reads formulas as formulas, which is what a user opening a workbook expects.
    pub fn new() -> Self {
        Self {
            import_formulas: true,
            functions: FunctionLibrary::standard(),
        }
    }

    /// Reads the cached results instead, for a host that wants the file's own answers.
    pub fn with_cached_values() -> Self {
        Self {
            import_formulas: false,
            functions: FunctionLibrary::standard(),
        }
    }

    /// Checks a formula's calls against a library of the caller's choosing, so a host that extends the
    /// language does not see its own functions reported as unsupported.
    pub fn with_functions(mut self, functions: FunctionLibrary) -> Self {
        self.functions = functions;
        self
    }
}

impl SpreadsheetImporter for XlsxImporter {
    fn extensions(&self) -> &'static [&'static str] {
        &[".xlsx", ".xlsm"]
    }

    fn import(
        &self,
        reader: &mut dyn ReadSeek,
        sink: &mut dyn pgsheets_core::store::ImportSink,
    ) -> Result<ImportResult, ImportError> {
        let mut archive = Archive::open(reader).map_err(|error| ImportError::Container(error.to_string()))?;

        let mut result = ImportResult::new();

        let sheets = parts::read_workbook_sheets(&mut archive, &mut result);

        let shared_strings = parts::read_shared_strings(&mut archive);
        let number_formats = styles::read_cell_formats(&mut archive);

        let mut imported = 0;

        for sheet in &sheets {
            if !sheet.is_worksheet {
                // A chartsheet or a macro sheet has no cells to read, and writing one back as a
                // worksheet would change what the file is, so it is reported rather than faked.
                result.dropped_sheets += 1;
                result.add_warning(format!(
                    "The '{}' sheet is not a worksheet and was not imported.",
                    sheet.name
                ));

                continue;
            }

            // The path the workbook gave can be absolute inside the package or relative to the workbook
            // part, and it has already been resolved by whoever read the relationships.
            let Some(worksheet) = archive.read(&sheet.path) else {
                result.add_warning(format!(
                    "Worksheet part '{}' is missing from the archive.",
                    sheet.path
                ));

                continue;
            };

            sink.begin_sheet(&sheet.name);

            worksheet::parse_worksheet(
                &worksheet,
                sink,
                &shared_strings,
                &number_formats,
                &mut result,
                self.import_formulas,
                &self.functions,
            );

            sink.end_sheet();

            if result.sheet_name.is_none() {
                result.sheet_name = Some(sheet.name.clone());
            }

            result.sheet_names.push(sheet.name.clone());
            imported += 1;
        }

        result.worksheet_count = imported;

        if imported == 0 && result.dropped_sheets == 0 {
            result.add_warning("The workbook does not contain any worksheets.");
        }

        Ok(result)
    }
}
