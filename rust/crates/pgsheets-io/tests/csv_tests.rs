//! The delimited-text reader and writer, including the round trip between them.

use std::io::Cursor;

use pgsheets_io::export::ExportOptions;
use pgsheets_io::export::csv::CsvExporter;
use pgsheets_io::import::csv::{CsvImporter, parse};
use pgsheets_io::{SpreadsheetExporter, SpreadsheetImporter};
use pgsheets_testkit::{MemorySource, RecordingSink};

/// Reads text through the importer, as a caller would.
fn read(text: &str) -> RecordingSink {
    let mut sink = RecordingSink::new();
    let mut reader = Cursor::new(text.as_bytes().to_vec());

    CsvImporter::new()
        .import(&mut reader, &mut sink)
        .expect("the text should read");

    sink
}

/// Writes a source as text, returning what was written and the result.
fn write(
    source: &MemorySource,
    file_name: &str,
    options: &ExportOptions,
) -> (String, pgsheets_io::ExportResult) {
    let mut buffer = Cursor::new(Vec::new());
    let result = CsvExporter::new()
        .export(&mut buffer, file_name, source, options)
        .expect("the text should write");

    let bytes = buffer.into_inner();
    // The byte-order mark is checked separately; the text is what matters here.
    let text = String::from_utf8(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes).to_vec()).unwrap();

    (text, result)
}

#[test]
fn parses_a_plain_grid() {
    let sink = read("a,b,c\r\n1,2,3\r\n");

    assert_eq!(sink.get("A1"), "a");
    assert_eq!(sink.get("B1"), "b");
    assert_eq!(sink.get("C1"), "c");
    assert_eq!(sink.get("A2"), "1");
    assert_eq!(sink.get("C2"), "3");
}

#[test]
fn counts_rows_columns_and_cells() {
    let mut sink = RecordingSink::new();
    let result = parse("a,b\r\n1,2\r\n", &mut sink);

    assert_eq!(result.rows, 2);
    assert_eq!(result.columns, 2);
    assert_eq!(result.cells_imported, 4);
}

#[test]
fn quoted_fields_keep_their_delimiters_and_newlines() {
    let sink = read("\"a,b\",\"c\nd\",\"e\"\"f\"\r\n");

    assert_eq!(sink.get("A1"), "a,b");
    assert_eq!(sink.get("B1"), "c\nd");
    assert_eq!(sink.get("C1"), "e\"f");
}

#[test]
fn the_delimiter_is_detected_from_the_first_line() {
    assert_eq!(read("a;b;c\r\n1;2;3\r\n").get("B1"), "b");
    assert_eq!(read("a\tb\tc\r\n").get("B1"), "b");
    assert_eq!(read("a|b|c\r\n").get("B1"), "b");
}

#[test]
fn a_byte_order_mark_is_not_part_of_the_first_field() {
    assert_eq!(read("\u{feff}a,b\r\n").get("A1"), "a");
}

#[test]
fn a_utf16_file_is_decoded() {
    // A UTF-16LE byte-order mark followed by "a,b" — what an editor's "Unicode" option writes.
    let bytes: Vec<u8> = [0xff, 0xfe]
        .iter()
        .copied()
        .chain("a,b".encode_utf16().flat_map(u16::to_le_bytes))
        .collect();

    let mut sink = RecordingSink::new();
    CsvImporter::new()
        .import(&mut Cursor::new(bytes), &mut sink)
        .expect("a UTF-16 file should read");

    assert_eq!(sink.get("A1"), "a");
    assert_eq!(sink.get("B1"), "b");
}

#[test]
fn a_trailing_record_without_a_newline_is_kept() {
    let sink = read("a,b\r\nc,d");

    assert_eq!(sink.get("A2"), "c");
    assert_eq!(sink.get("B2"), "d");
}

#[test]
fn empty_input_reads_as_nothing() {
    let mut sink = RecordingSink::new();
    let result = parse("", &mut sink);

    assert_eq!(result.rows, 0);
    assert_eq!(result.columns, 0);
    assert!(sink.is_empty());
}

#[test]
fn the_importer_claims_the_extensions_it_reads() {
    let importer = CsvImporter::new();

    assert!(importer.can_import("a.csv"));
    assert!(importer.can_import("a.CSV"));
    assert!(importer.can_import("a.tsv"));
    assert!(!importer.can_import("a.xlsx"));
    assert!(!importer.can_import("noextension"));
}

#[test]
fn writes_a_grid_with_a_byte_order_mark() {
    let mut source = MemorySource::new();
    source.set("A1", "a").set("B1", "b").set("A2", "1");

    let mut buffer = Cursor::new(Vec::new());
    CsvExporter::new()
        .export(&mut buffer, "sheet.csv", &source, &ExportOptions::new())
        .expect("the grid should write");

    let bytes = buffer.into_inner();

    assert!(
        bytes.starts_with(&[0xef, 0xbb, 0xbf]),
        "a mark keeps non-ASCII readable"
    );
    assert_eq!(&bytes[3..], b"a,b\r\n1,\r\n");
}

#[test]
fn a_tsv_export_uses_tabs() {
    let mut source = MemorySource::new();
    source.set("A1", "a").set("B1", "b");

    let (text, _) = write(&source, "sheet.tsv", &ExportOptions::new());

    assert_eq!(text, "a\tb\r\n");
}

#[test]
fn fields_are_quoted_only_when_they_have_to_be() {
    let mut source = MemorySource::new();
    source.set("A1", "plain");
    source.set("B1", "has,comma");
    source.set("C1", "has\"quote");
    source.set("D1", "has\nnewline");
    source.set("E1", " padded ");
    source.set("F1", "");

    let (text, _) = write(&source, "sheet.csv", &ExportOptions::new());

    assert_eq!(
        text,
        "plain,\"has,comma\",\"has\"\"quote\",\"has\nnewline\",\" padded \",\r\n"
    );
}

#[test]
fn a_round_trip_through_text_preserves_what_was_written() {
    let mut source = MemorySource::new();
    source.set("A1", "plain");
    source.set("B1", "has,comma");
    source.set("C1", "has\"quote");
    source.set("D1", "has\nnewline");
    source.set("E1", " padded ");
    source.set("A2", "42");

    let (text, _) = write(&source, "sheet.csv", &ExportOptions::new());
    let sink = read(&text);

    assert_eq!(sink.get("A1"), "plain");
    assert_eq!(sink.get("B1"), "has,comma");
    assert_eq!(sink.get("C1"), "has\"quote");
    assert_eq!(sink.get("D1"), "has\nnewline");
    assert_eq!(sink.get("E1"), " padded ");
    assert_eq!(sink.get("A2"), "42");
}

#[test]
fn a_range_export_keeps_its_rectangle() {
    let mut source = MemorySource::new();
    source.set("A1", "outside");
    source.set("B2", "in");
    source.set("D4", "outside");

    let options = ExportOptions::new().within(MemorySource::range("B2", "C3"));
    let (text, result) = write(&source, "sheet.csv", &options);

    assert_eq!(result.rows, 2);
    assert_eq!(result.columns, 2);
    assert_eq!(text, "in,\r\n,\r\n");
}

#[test]
fn value_mode_warns_about_text_that_will_read_back_as_something_else() {
    let mut source = MemorySource::new();
    source.set_value("A1", pgsheets_core::CellValue::text("007"), None);
    source.set_value("A2", pgsheets_core::CellValue::text("=1+1"), None);

    let (_, result) = write(&source, "sheet.csv", &ExportOptions::new().with_values());
    let warnings = result.warnings.join("\n");

    assert!(warnings.contains("look like numbers"), "{warnings}");
    assert!(warnings.contains("start with '='"), "{warnings}");
}

#[test]
fn raw_input_is_written_by_default_and_values_only_when_asked_for() {
    let mut source = MemorySource::new();

    // A formula has both: the raw input is what a save preserves, the value is what the grid shows.
    source.set_formula("A1", "=1+1", pgsheets_core::CellValue::number(2.0));

    let (raw_mode, _) = write(&source, "sheet.csv", &ExportOptions::new());
    assert_eq!(raw_mode, "=1+1\r\n");

    let (value_mode, _) = write(&source, "sheet.csv", &ExportOptions::new().with_values());
    assert_eq!(value_mode, "2\r\n");
}

#[test]
fn a_cell_with_no_raw_input_behind_it_writes_nothing() {
    let mut source = MemorySource::new();

    // An imported date has a value and a format but no raw input, and the raw input is what is
    // written; asking for values does not invent one.
    source.set_value("A1", pgsheets_core::CellValue::number(3.5), None);

    assert_eq!(write(&source, "sheet.csv", &ExportOptions::new()).0, "\r\n");
    assert_eq!(
        write(&source, "sheet.csv", &ExportOptions::new().with_values()).0,
        "\r\n"
    );
}
