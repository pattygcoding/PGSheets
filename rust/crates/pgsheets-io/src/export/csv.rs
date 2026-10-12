//! Writes delimited text (RFC 4180).
//!
//! Quoting, escaping and line endings are chosen to round-trip back through the CSV reader.

use pgsheets_core::{CellValue, ExportSource};

use super::{ExportError, ExportOptions, ExportResult, SpreadsheetExporter};
use crate::zip::WriteSeek;

/// Line ending used between records. CRLF is what spreadsheet applications expect.
const RECORD_SEPARATOR: &str = "\r\n";

/// The byte-order mark that keeps non-ASCII text readable in an application that is not UTF-8 native.
const BYTE_ORDER_MARK: [u8; 3] = [0xef, 0xbb, 0xbf];

/// Writes delimited text.
#[derive(Default)]
pub struct CsvExporter;

impl CsvExporter {
    /// Creates the writer.
    pub fn new() -> Self {
        Self
    }
}

impl SpreadsheetExporter for CsvExporter {
    fn extensions(&self) -> &'static [&'static str] {
        &[".csv", ".tsv", ".txt"]
    }

    fn export(
        &self,
        writer: &mut dyn WriteSeek,
        file_name: &str,
        source: &dyn ExportSource,
        options: &ExportOptions,
    ) -> Result<ExportResult, ExportError> {
        let mut result = ExportResult::new();
        let text = build(source, options, delimiter_for(file_name), &mut result);

        if text.is_empty() {
            return Ok(result);
        }

        writer.write_all(&BYTE_ORDER_MARK)?;
        writer.write_all(text.as_bytes())?;

        Ok(result)
    }
}

/// Tab for `.tsv`, comma for everything else.
pub fn delimiter_for(file_name: &str) -> char {
    if crate::import::extension_of(file_name).as_deref() == Some(".tsv") {
        '\t'
    } else {
        ','
    }
}

/// Renders a region as delimited text, which is also what the clipboard carries.
pub fn build(
    source: &dyn ExportSource,
    options: &ExportOptions,
    delimiter: char,
    result: &mut ExportResult,
) -> String {
    let (first_row, first_column, last_row, last_column, cells) = extent(source, options);

    result.rows = last_row - first_row + 1;
    result.columns = last_column - first_column + 1;
    result.cells_written = cells;

    if last_row < first_row || last_column < first_column {
        return String::new();
    }

    if options.export_values {
        report_lossy_values(source, result);
    }

    let mut text = String::new();

    for row in first_row..=last_row {
        for column in first_column..=last_column {
            if column > first_column {
                text.push(delimiter);
            }

            append_field(&mut text, &cell_text(source, row, column, options), delimiter);
        }

        text.push_str(RECORD_SEPARATOR);
    }

    text
}

fn extent(source: &dyn ExportSource, options: &ExportOptions) -> (i32, i32, i32, i32, i32) {
    let mut cells = 0;

    match options.range {
        // A range keeps its rectangular shape even where cells are empty, so a copied or saved block
        // lines up with what was selected.
        Some(selected) => {
            for address in source.populated_cells() {
                if selected.contains(address.row, address.column)
                    && !source.raw_input(address.row, address.column).is_empty()
                {
                    cells += 1;
                }
            }

            (
                selected.row1,
                selected.column1,
                selected.row2,
                selected.column2,
                cells,
            )
        }
        None => {
            let mut last_row = -1;
            let mut last_column = -1;

            for address in source.populated_cells() {
                last_row = last_row.max(address.row);
                last_column = last_column.max(address.column);

                if !source.raw_input(address.row, address.column).is_empty() {
                    cells += 1;
                }
            }

            (0, 0, last_row, last_column, cells)
        }
    }
}

fn cell_text(source: &dyn ExportSource, row: i32, column: i32, options: &ExportOptions) -> String {
    let raw = source.raw_input(row, column);

    if !options.export_values || raw.is_empty() {
        return raw.to_string();
    }

    source.value(row, column).to_display_string()
}

/// Quotes a field when it contains the delimiter, a quote or a line break, and also when it has
/// leading or trailing whitespace that an unquoted round trip would silently trim.
fn append_field(out: &mut String, text: &str, delimiter: char) {
    let needs_quotes = text.contains(delimiter)
        || text.contains('"')
        || text.contains('\n')
        || text.contains('\r')
        || text.starts_with(char::is_whitespace)
        || text.ends_with(char::is_whitespace);

    if !needs_quotes {
        out.push_str(text);
        return;
    }

    out.push('"');

    for ch in text.chars() {
        if ch == '"' {
            out.push('"');
        }

        out.push(ch);
    }

    out.push('"');
}

/// In value mode a text result that looks numeric — or starts with `=` — is read back as a number or
/// as a formula. Reporting that is the honest option; silently changing the data is not.
fn report_lossy_values(source: &dyn ExportSource, result: &mut ExportResult) {
    let mut numeric_looking = 0;
    let mut formula_looking = 0;

    for address in source.populated_cells() {
        let CellValue::Text(text) = source.value(address.row, address.column) else {
            continue;
        };

        if text.is_empty() {
            continue;
        }

        if text.starts_with('=') {
            formula_looking += 1;
        } else if CellValue::shared_text(text).as_number().is_some() {
            numeric_looking += 1;
        }
    }

    if numeric_looking > 0 {
        result.add_warning(format!(
            "{numeric_looking} text cell(s) look like numbers and will be read back as numbers."
        ));
    }

    if formula_looking > 0 {
        result.add_warning(format!(
            "{formula_looking} text cell(s) start with '=' and will be read back as formulas."
        ));
    }
}
