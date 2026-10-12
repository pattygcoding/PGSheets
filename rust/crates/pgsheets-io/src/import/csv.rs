//! Reads delimited text (RFC 4180): quoted fields, escaped quotes (`""`) and embedded newlines, with
//! a UTF-8 BOM stripped and the delimiter detected from the first line.
//!
//! Written out rather than taken from a crate, like every other format here.

use pgsheets_core::store::ImportSink;
use pgsheets_core::{MAX_COLUMNS, MAX_ROWS};

use super::{ImportError, ImportResult, SpreadsheetImporter};
use crate::zip::ReadSeek;

/// The delimiters a first line is tested against; the one it uses most wins.
const DELIMITER_CANDIDATES: [char; 4] = [',', ';', '\t', '|'];

/// Reads delimited text.
#[derive(Default)]
pub struct CsvImporter;

impl CsvImporter {
    /// Creates the reader.
    pub fn new() -> Self {
        Self
    }
}

impl SpreadsheetImporter for CsvImporter {
    fn extensions(&self) -> &'static [&'static str] {
        &[".csv", ".tsv", ".txt"]
    }

    fn import(
        &self,
        reader: &mut dyn ReadSeek,
        sink: &mut dyn ImportSink,
    ) -> Result<ImportResult, ImportError> {
        let mut bytes = Vec::new();

        reader
            .read_to_end(&mut bytes)
            .map_err(|error| ImportError::Container(error.to_string()))?;

        Ok(parse(&decode(&bytes), sink))
    }
}

/// Parses delimited text. Exposed so a caller can parse text it already holds — the clipboard does.
pub fn parse(text: &str, sink: &mut dyn ImportSink) -> ImportResult {
    let mut result = ImportResult::new();

    if text.is_empty() {
        return result;
    }

    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let delimiter = detect_delimiter(text);

    sink.begin_sheet("Sheet1");

    let mut row = 0;
    let mut column = 0;
    let mut max_column = 0;
    let mut rows = 0;
    let mut cells = 0;
    let mut field_started = false;
    let mut in_quotes = false;
    let mut field = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                    continue;
                }

                in_quotes = false;
                continue;
            }

            field.push(ch);
            continue;
        }

        if ch == '"' && !field_started {
            in_quotes = true;
            field_started = true;
            continue;
        }

        if ch == delimiter {
            emit_field(&field, row, &mut column, &mut cells, sink);
            field.clear();
            field_started = false;
            continue;
        }

        if ch == '\r' || ch == '\n' {
            emit_field(&field, row, &mut column, &mut cells, sink);
            field.clear();
            field_started = false;

            max_column = max_column.max(column);
            column = 0;
            rows += 1;
            row += 1;

            if ch == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
            }

            continue;
        }

        field.push(ch);
        field_started = true;
    }

    // Flush a trailing record that was not terminated by a newline.
    if !field.is_empty() || column > 0 || in_quotes {
        emit_field(&field, row, &mut column, &mut cells, sink);
        max_column = max_column.max(column);
        rows += 1;
    }

    sink.end_sheet();

    result.sheet_name = Some("Sheet1".to_string());
    result.sheet_names = vec!["Sheet1".to_string()];
    result.worksheet_count = 1;
    result.rows = rows;
    result.columns = max_column;
    result.cells_imported = cells;
    result
}

fn emit_field(field: &str, row: i32, column: &mut i32, cells: &mut i32, sink: &mut dyn ImportSink) {
    if row < MAX_ROWS && *column < MAX_COLUMNS {
        sink.set_raw_input(row, *column, field);

        if !field.is_empty() {
            *cells += 1;
        }
    }

    *column += 1;
}

/// The delimiter the first line uses most.
fn detect_delimiter(text: &str) -> char {
    let end = text.find(['\r', '\n']).unwrap_or(text.len());
    let first_line = &text[..end];

    let mut best = ',';
    let mut best_count = 0;

    for candidate in DELIMITER_CANDIDATES {
        let count = first_line.chars().filter(|ch| *ch == candidate).count();

        if count > best_count {
            best_count = count;
            best = candidate;
        }
    }

    best
}

/// Decodes a text file, honouring a byte-order mark the way a `StreamReader` does.
///
/// Invalid UTF-8 becomes the replacement character rather than an error, which is what the original's
/// decoder fallback did: a spreadsheet is more useful with one odd character in it than not at all.
pub(crate) fn decode(bytes: &[u8]) -> String {
    match bytes {
        [0xef, 0xbb, 0xbf, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
        [0xff, 0xfe, rest @ ..] => decode_utf16(rest, u16::from_le_bytes),
        [0xfe, 0xff, rest @ ..] => decode_utf16(rest, u16::from_be_bytes),
        other => String::from_utf8_lossy(other).into_owned(),
    }
}

fn decode_utf16(bytes: &[u8], to_unit: fn([u8; 2]) -> u16) -> String {
    let (pairs, _) = bytes.as_chunks::<2>();
    let units: Vec<u16> = pairs.iter().map(|pair| to_unit(*pair)).collect();

    String::from_utf16_lossy(&units)
}
