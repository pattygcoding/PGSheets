//! Writing the worksheet part: one `<row>` per populated row, and one `<c>` per cell.

use pgsheets_core::numeric::write_round_trip;
use pgsheets_core::{CellAddress, CellValue, ExportSource, MAX_COLUMNS, MAX_ROWS, parse_literal};

use super::NumberFormatTable;
use crate::export::{ExportOptions, ExportResult};
use crate::ooxml::sanitize_text;
use crate::xml::XmlWriter;

/// Streams the worksheet.
///
/// Only populated cells are visited, grouped into one `<row>` each, so the part is written in a single
/// pass without building a document in memory.
pub(super) fn write_worksheet(
    source: &dyn ExportSource,
    options: &ExportOptions,
    number_formats: &NumberFormatTable,
    result: &mut ExportResult,
) -> String {
    let mut cells: Vec<CellAddress> = source
        .populated_cells()
        .into_iter()
        .filter(|address| {
            !options
                .range
                .is_some_and(|range| !range.contains(address.row, address.column))
        })
        .filter(|address| (0..MAX_ROWS).contains(&address.row) && (0..MAX_COLUMNS).contains(&address.column))
        .collect();

    cells.sort_unstable();

    let mut xml = XmlWriter::new();
    xml.declaration();
    xml.start("worksheet");
    xml.start("sheetData");

    let mut max_row = -1;
    let mut max_column = -1;
    let mut written = 0;
    let mut index = 0;

    while index < cells.len() {
        let row = cells[index].row;

        xml.start("row");
        xml.attribute("r", &(row + 1).to_string());

        while index < cells.len() && cells[index].row == row {
            let column = cells[index].column;

            write_cell(
                &mut xml,
                row,
                column,
                source,
                number_formats.index_of_known(source.number_format(row, column)),
            );

            written += 1;
            max_column = max_column.max(column);
            index += 1;
        }

        xml.end();
        max_row = row;
    }

    xml.end();
    xml.end();

    result.rows = max_row + 1;
    result.columns = max_column + 1;
    result.cells_written = written;

    xml.into_string()
}

/// Writes one cell, choosing the representation its value calls for.
fn write_cell(xml: &mut XmlWriter, row: i32, column: i32, source: &dyn ExportSource, style_index: i32) {
    let raw = source.raw_input(row, column);
    let reference = CellAddress::to_a1(row, column);

    // A leading '=' is what makes a cell a formula; writing it as text would turn every formula into a
    // string, so it is written as <f> without the sign.
    if let Some(formula) = raw.strip_prefix('=') {
        write_formula_cell(xml, &reference, formula, &source.value(row, column), style_index);
        return;
    }

    let mut value = source.value(row, column);

    if value.is_empty() {
        // A populated cell always evaluates to something; fall back to the literal rules.
        value = parse_literal(raw);
    }

    match value {
        // "R" round-trips the double exactly, so a save/load cycle does not drift.
        CellValue::Number(number) => {
            write_value_cell(xml, &reference, None, &write_round_trip(number), style_index);
        }

        CellValue::Boolean(flag) => {
            write_value_cell(
                xml,
                &reference,
                Some("b"),
                if flag { "1" } else { "0" },
                style_index,
            );
        }

        CellValue::Error(error) => {
            write_value_cell(xml, &reference, Some("e"), error.text(), style_index);
        }

        // Text is written as the raw input, which is what preserves it verbatim.
        _ => write_inline_string_cell(xml, &reference, raw, style_index),
    }
}

/// Writes a formula, with its result cached when that result is a value.
fn write_formula_cell(
    xml: &mut XmlWriter,
    reference: &str,
    formula: &str,
    value: &CellValue,
    style_index: i32,
) {
    xml.start("c");
    xml.attribute("r", reference);
    write_style(xml, style_index);

    // Without the type flag a string result would be read back as a number.
    match value {
        CellValue::Text(_) => xml.attribute("t", "str"),
        CellValue::Boolean(_) => xml.attribute("t", "b"),
        _ => {}
    }

    xml.start("f");
    xml.text(&sanitize_text(formula));
    xml.end();

    // Errors are not cached: the reading application recalculates rather than inheriting this engine's
    // #NAME? results for functions it does not implement.
    if !value.is_error() {
        write_value_element(xml, &value_text(value));
    }

    xml.end();
}

/// Writes a cell whose value stands on its own, with an optional type flag.
fn write_value_cell(
    xml: &mut XmlWriter,
    reference: &str,
    cell_type: Option<&str>,
    text: &str,
    style_index: i32,
) {
    xml.start("c");
    xml.attribute("r", reference);

    if let Some(cell_type) = cell_type {
        xml.attribute("t", cell_type);
    }

    write_style(xml, style_index);
    write_value_element(xml, text);
    xml.end();
}

/// Writes a cell whose text is stored inline, so no shared string table is needed.
fn write_inline_string_cell(xml: &mut XmlWriter, reference: &str, text: &str, style_index: i32) {
    xml.start("c");
    xml.attribute("r", reference);
    xml.attribute("t", "inlineStr");
    write_style(xml, style_index);

    xml.start("is");
    xml.start("t");

    // Without this, leading and trailing spaces are silently dropped, and text that is entirely
    // whitespace is discarded altogether.
    if text.trim().len() != text.len() {
        xml.qualified_attribute("xml:space", "preserve");
    }

    xml.text(&sanitize_text(text));
    xml.end();
    xml.end();
    xml.end();
}

fn write_value_element(xml: &mut XmlWriter, text: &str) {
    xml.start("v");
    xml.text(&sanitize_text(text));
    xml.end();
}

/// Writes the style index that ties a cell to its number format.
///
/// Index 0 is the default entry, so leaving the attribute off says the same thing while keeping the
/// part smaller.
fn write_style(xml: &mut XmlWriter, style_index: i32) {
    if style_index > 0 {
        xml.attribute("s", &style_index.to_string());
    }
}

/// How a value is written into `<v>`: a number round-trips, a boolean is 1 or 0, and anything else is
/// its display form.
fn value_text(value: &CellValue) -> String {
    match value {
        CellValue::Number(number) => write_round_trip(*number),
        CellValue::Boolean(flag) => if *flag { "1" } else { "0" }.to_string(),
        other => other.to_display_string(),
    }
}
