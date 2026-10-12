//! Streaming a worksheet part: rows and cells in, raw input out.

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::XmlVersion;
use quick_xml::events::{BytesStart, Event};

use pgsheets_core::store::ImportSink;
use pgsheets_core::{CellAddress, MAX_COLUMNS, MAX_ROWS};
use pgsheets_formulas::FunctionLibrary;

use crate::import::ImportResult;
use crate::import::csv::decode;

use super::cells::{extract_function_names, resolve_cell};
use super::parts::{local_name, resolve_reference, xml_text};

/// One cell as it is being read, before it is resolved.
struct PendingCell {
    row: i32,
    column: i32,
    cell_type: Option<String>,
    style: Option<String>,
    value: String,
    formula: String,
    inline_text: Option<String>,
}

/// The element whose text is currently being collected, if any.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TextTarget {
    Value,
    Formula,
    InlineText,
}

/// Reads a worksheet part, pushing each cell into the sink.
///
/// Nothing larger than one cell is ever materialised: the reader walks `<row>` and `<c>` and hands
/// each cell straight on.
pub(super) fn parse_worksheet(
    part: &[u8],
    sink: &mut dyn ImportSink,
    shared_strings: &[String],
    number_formats: &HashMap<i32, String>,
    result: &mut ImportResult,
    import_formulas: bool,
    functions: &FunctionLibrary,
) {
    let text = decode(part);
    let mut reader = Reader::from_str(&text);

    let mut current_row = -1;
    let mut next_column = 0;
    let mut max_row = 0;
    let mut max_column = 0;
    let mut cells = 0;
    let mut pending: Option<PendingCell> = None;
    let mut collecting: Option<TextTarget> = None;
    let mut unknown_functions: Vec<String> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => match local_name(element.name()) {
                "row" => {
                    current_row = row_index(&element).unwrap_or(current_row + 1);
                    next_column = 0;
                }
                "c" => pending = Some(begin_cell(&element, current_row, &mut next_column)),
                "v" if pending.is_some() => collecting = Some(TextTarget::Value),
                "f" if pending.is_some() => collecting = Some(TextTarget::Formula),
                "t" if pending.is_some() => collecting = Some(TextTarget::InlineText),
                _ => {}
            },

            // An empty element has no children, so a cell declared that way is complete already.
            Ok(Event::Empty(element)) => {
                if local_name(element.name()) == "c" {
                    let cell = begin_cell(&element, current_row, &mut next_column);

                    collect(
                        cell,
                        shared_strings,
                        number_formats,
                        import_formulas,
                        functions,
                        sink,
                        &mut cells,
                        &mut max_row,
                        &mut max_column,
                        &mut unknown_functions,
                    );
                }
            }

            Ok(Event::Text(raw)) => {
                if let (Some(target), Some(cell)) = (collecting, pending.as_mut()) {
                    append_to_cell(cell, target, &xml_text(&raw));
                }
            }

            // An entity or character reference arrives as its own event, so the text around it would
            // otherwise be joined up with a hole where the reference was.
            Ok(Event::GeneralRef(raw)) => {
                if let (Some(target), Some(cell)) = (collecting, pending.as_mut()) {
                    append_to_cell(cell, target, &resolve_reference(&raw));
                }
            }

            Ok(Event::End(element)) => match local_name(element.name()) {
                "v" | "f" | "t" => collecting = None,
                "c" => {
                    collecting = None;

                    if let Some(cell) = pending.take() {
                        collect(
                            cell,
                            shared_strings,
                            number_formats,
                            import_formulas,
                            functions,
                            sink,
                            &mut cells,
                            &mut max_row,
                            &mut max_column,
                            &mut unknown_functions,
                        );
                    }
                }
                _ => {}
            },

            // A document type can describe entities, and an external one can make a reader fetch
            // something. Workbooks are untrusted input, so a part carrying one is abandoned rather
            // than parsed carefully.
            Ok(Event::DocType(_)) | Err(_) => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
    }

    result.rows = max_row;
    result.columns = max_column;
    result.cells_imported = cells;

    unknown_functions.sort_by_key(|name| name.to_lowercase());
    unknown_functions.dedup_by_key(|name| name.to_lowercase());

    for function in unknown_functions {
        result.add_warning(format!(
            "Unsupported function '{function}' will evaluate to #NAME?."
        ));
    }
}

/// Adds a piece of an element's text to the part of the cell it belongs to.
///
/// Text arrives in pieces — one event per run between references — so this is called several times for
/// a cell whose text contains an escape.
fn append_to_cell(cell: &mut PendingCell, target: TextTarget, text: &str) {
    match target {
        TextTarget::Value => cell.value.push_str(text),
        TextTarget::Formula => cell.formula.push_str(text),
        TextTarget::InlineText => cell.inline_text.get_or_insert_with(String::new).push_str(text),
    }
}

/// Sets up the cell a `<c>` element describes.
fn begin_cell(element: &BytesStart<'_>, current_row: i32, next_column: &mut i32) -> PendingCell {
    let reference = attribute(element, "r");
    let cell_type = attribute(element, "t");
    let style = attribute(element, "s");

    // A cell that names its own reference is trusted over the running position, which is how a file
    // that skips empty cells is read correctly.
    let (row, column) = reference
        .as_deref()
        .and_then(CellAddress::try_parse_a1)
        .unwrap_or((current_row, *next_column));

    *next_column = column + 1;

    PendingCell {
        row,
        column,
        cell_type,
        style,
        value: String::new(),
        formula: String::new(),
        inline_text: None,
    }
}

/// Resolves a completed cell and, when it lands on the sheet, hands it to the sink.
#[allow(clippy::too_many_arguments)]
fn collect(
    cell: PendingCell,
    shared_strings: &[String],
    number_formats: &HashMap<i32, String>,
    import_formulas: bool,
    functions: &FunctionLibrary,
    sink: &mut dyn ImportSink,
    cells: &mut i32,
    max_row: &mut i32,
    max_column: &mut i32,
    unknown_functions: &mut Vec<String>,
) {
    let (raw, format) = resolve_cell(
        cell.cell_type.as_deref(),
        non_empty(&cell.value),
        non_empty(&cell.formula),
        cell.inline_text.as_deref(),
        cell.style.as_deref(),
        shared_strings,
        number_formats,
        import_formulas,
    );

    if import_formulas && !cell.formula.is_empty() {
        for function in extract_function_names(&cell.formula) {
            if !functions.is_known(&function) {
                unknown_functions.push(function);
            }
        }
    }

    if cell.row < 0 || cell.column < 0 || cell.row >= MAX_ROWS || cell.column >= MAX_COLUMNS {
        return;
    }

    sink.set_raw_input(cell.row, cell.column, &raw);

    if let Some(format) = format.filter(|format| !format.is_empty()) {
        // The format is what makes a serial read as a date, so it travels with the value.
        sink.set_number_format(cell.row, cell.column, Some(&format));
    }

    if !raw.is_empty() {
        *cells += 1;
    }

    *max_row = (*max_row).max(cell.row + 1);
    *max_column = (*max_column).max(cell.column + 1);
}

/// The row index a `<row r="..">` stands for, counted from zero.
fn row_index(element: &BytesStart<'_>) -> Option<i32> {
    attribute(element, "r")?.parse::<i32>().ok().map(|row| row - 1)
}

/// An attribute by qualified name, normalised as XML requires.
fn attribute(element: &BytesStart<'_>, name: &str) -> Option<String> {
    element
        .attributes()
        .flatten()
        .find(|attribute| attribute.key.as_ref() == name)
        .and_then(|attribute| {
            attribute
                .normalized_value(XmlVersion::Implicit1_0)
                .ok()
                .map(|value| value.into_owned())
        })
}

/// `None` for an empty string, so a cell with no `<v>` is treated as absent rather than blank.
fn non_empty(text: &str) -> Option<&str> {
    if text.is_empty() { None } else { Some(text) }
}
