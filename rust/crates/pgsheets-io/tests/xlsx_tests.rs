//! The SpreadsheetML reader and writer, including the round trip between them and the package's own
//! structure.

use std::io::Cursor;

use pgsheets_core::{CellStyle, CellValue, HorizontalAlign};
use pgsheets_io::export::ExportOptions;
use pgsheets_io::export::XlsxWriter;
use pgsheets_io::import::XlsxImporter;
use pgsheets_io::zip::{Archive, PackageWriter};
use pgsheets_io::{SheetRef, SpreadsheetExporter, SpreadsheetImporter};
use pgsheets_testkit::{MemorySource, RecordingSink};

/// A source with one of everything the writer has to represent.
fn sample() -> MemorySource {
    let mut source = MemorySource::new();

    source.set("A1", "text");
    source.set("A2", "42");
    source.set("A3", "0.155");
    source.set("A4", "TRUE");
    source.set("A5", "=A2*2");
    source.set("A6", " leading and trailing ");
    source.set("A7", "<&>\"quoted\"");
    source.set_value("A8", CellValue::number(45356.0), Some("yyyy-mm-dd"));
    source.set("B1", "second column");

    source
}

/// Writes a source into a package held in memory.
fn write(source: &MemorySource, options: &ExportOptions) -> Vec<u8> {
    let mut buffer = Cursor::new(Vec::new());

    XlsxWriter::new()
        .export(&mut buffer, "book.xlsx", source, options)
        .expect("the workbook should write");

    buffer.into_inner()
}

/// Reads a package held in memory.
fn read(bytes: Vec<u8>) -> (RecordingSink, pgsheets_io::ImportResult) {
    let mut sink = RecordingSink::new();
    let result = XlsxImporter::new()
        .import(&mut Cursor::new(bytes), &mut sink)
        .expect("the workbook should read");

    (sink, result)
}

#[test]
fn a_workbook_round_trips_through_the_writer_and_the_reader() {
    let (sink, result) = read(write(&sample(), &ExportOptions::new()));

    assert_eq!(sink.get("A1"), "text");
    assert_eq!(sink.get("A2"), "42");
    assert_eq!(sink.get("A3"), "0.155");
    assert_eq!(sink.get("A4"), "TRUE");
    assert_eq!(sink.get("A6"), " leading and trailing ");
    assert_eq!(sink.get("A7"), "<&>\"quoted\"");
    assert_eq!(sink.get("B1"), "second column");

    // The formula survives as a formula, which is what a save and reload has to preserve.
    assert_eq!(sink.get("A5"), "=A2*2");

    assert_eq!(result.sheet_name.as_deref(), Some("Sheet1"));
    assert_eq!(result.worksheet_count, 1);
    assert_eq!(result.rows, 8);
    assert_eq!(result.columns, 2);
}

#[test]
fn a_number_format_survives_the_round_trip() {
    let (sink, _) = read(write(&sample(), &ExportOptions::new()));

    // The format is what makes the serial a date again, so it has to come back with the value.
    assert_eq!(sink.get("A8"), "45356");
    assert_eq!(sink.get_format("A8"), Some("yyyy-mm-dd".to_string()));
    assert_eq!(sink.get_format("A1"), None);
}

#[test]
fn the_package_holds_the_parts_a_workbook_needs() {
    let bytes = write(&sample(), &ExportOptions::new());
    let mut archive = Archive::open(Cursor::new(bytes)).expect("the package should open");

    for part in [
        "[Content_Types].xml",
        "_rels/.rels",
        "xl/workbook.xml",
        "xl/_rels/workbook.xml.rels",
        "xl/styles.xml",
        "xl/worksheets/sheet1.xml",
    ] {
        assert!(archive.has(part), "{part} is missing from the package");
    }

    let content_types = archive
        .read_text("[Content_Types].xml")
        .expect("the part should read");
    assert!(content_types.contains("spreadsheetml.sheet.main+xml"));
    assert!(content_types.contains("/xl/worksheets/sheet1.xml"));

    let workbook = archive
        .read_text("xl/workbook.xml")
        .expect("the part should read");
    assert!(workbook.contains("name=\"Sheet1\""));
    assert!(workbook.contains("r:id=\"rId1\""));
}

#[test]
fn the_style_table_is_sized_to_the_formats_actually_used() {
    let mut source = MemorySource::new();
    source.set("A1", "1");
    source.set_formatted("A2", "0.5", "0%");
    source.set_formatted("A3", "0.5", "0.0%");
    source.set_formatted("A4", "0.5", "0%");

    let bytes = write(&source, &ExportOptions::new());
    let mut archive = Archive::open(Cursor::new(bytes)).expect("the package should open");
    let styles = archive.read_text("xl/styles.xml").expect("the part should read");

    // Two distinct codes were used, however many cells carried them.
    assert!(styles.contains("<numFmts count=\"2\">"), "{styles}");
    // Entry 0 is General, so the cell formats are those two plus it.
    assert!(styles.contains("<cellXfs count=\"3\">"), "{styles}");
    // 164 is the first custom id the specification leaves to a workbook.
    assert!(styles.contains("numFmtId=\"164\""), "{styles}");
    assert!(styles.contains("numFmtId=\"165\""), "{styles}");
    // Excel insists on at least two fills and a plain cell style.
    assert!(styles.contains("<fills count=\"2\">"), "{styles}");
    assert!(styles.contains("<cellStyleXfs count=\"1\">"), "{styles}");
}

#[test]
fn a_formula_result_that_is_an_error_is_not_cached() {
    let mut source = MemorySource::new();
    source.set_formula("A1", "=NOPE()", CellValue::error(pgsheets_core::CellError::Name));

    let bytes = write(&source, &ExportOptions::new());
    let mut archive = Archive::open(Cursor::new(bytes)).expect("the package should open");
    let sheet = archive
        .read_text("xl/worksheets/sheet1.xml")
        .expect("the part should read");

    assert!(sheet.contains("<f>NOPE()</f>"), "{sheet}");
    // The reading application recalculates rather than inheriting this engine's answer.
    assert!(!sheet.contains("#NAME?"), "{sheet}");

    // And it still reads back as the formula that was written.
    let (sink, _) = read(write(&source, &ExportOptions::new()));
    assert_eq!(sink.get("A1"), "=NOPE()");
}

#[test]
fn a_range_export_writes_only_the_selection() {
    let mut source = MemorySource::new();
    source.set("A1", "outside");
    source.set("B2", "in");
    source.set("D4", "outside");

    let options = ExportOptions::new().within(MemorySource::range("B2", "C3"));
    let bytes = write(&source, &options);
    let mut archive = Archive::open(Cursor::new(bytes)).expect("the package should open");
    let sheet = archive
        .read_text("xl/worksheets/sheet1.xml")
        .expect("the part should read");

    assert!(sheet.contains(">in<"), "{sheet}");
    assert!(!sheet.contains("outside"), "{sheet}");
}

#[test]
fn an_invalid_sheet_name_is_sanitised_and_reported() {
    let source = MemorySource::new();
    let options = ExportOptions::named("a/name:with[bad]chars");

    let bytes = write(&source, &options);
    let mut archive = Archive::open(Cursor::new(bytes)).expect("the package should open");
    let workbook = archive
        .read_text("xl/workbook.xml")
        .expect("the part should read");

    assert!(workbook.contains("name=\"anamewithbadchars\""), "{workbook}");

    let mut buffer = Cursor::new(Vec::new());
    let written = XlsxWriter::new()
        .export(&mut buffer, "book.xlsx", &source, &options)
        .expect("the workbook should write");

    assert!(
        written
            .warnings
            .iter()
            .any(|warning| warning.contains("not valid in xlsx")),
        "{:?}",
        written.warnings
    );
}

#[test]
fn a_part_with_a_document_type_is_refused() {
    // A document type can describe entities, and an external one can make a reader fetch something.
    // The reader abandons a part that carries one rather than parsing it carefully.
    let mut buffer = Cursor::new(Vec::new());
    let mut package = PackageWriter::create(&mut buffer);

    let hostile = concat!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>",
        "<!DOCTYPE worksheet [<!ENTITY xxe SYSTEM \"file:///etc/passwd\">]>",
        "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">",
        "<sheetData><row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t>&xxe;</t></is></c></row></sheetData>",
        "</worksheet>"
    );

    package
        .write_part("xl/workbook.xml", &workbook_part("Sheet1"))
        .expect("the part should write");
    package
        .write_part("xl/worksheets/sheet1.xml", hostile)
        .expect("the part should write");
    package.finish().expect("the package should close");

    let (sink, result) = read(buffer.into_inner());

    // Nothing from the hostile part reached the sink, and the cell count says so.
    assert_eq!(sink.get("A1"), "");
    assert_eq!(result.cells_imported, 0);
}

#[test]
fn a_style_survives_the_round_trip() {
    let mut source = MemorySource::new();

    source.set_styled(
        "C1",
        "loud",
        CellStyle::DEFAULT
            .with_bold(true)
            .with_italic(true)
            .with_underline(true)
            .with_align(HorizontalAlign::Right)
            .with_text_color(Some([0xFF, 0x00, 0x00]))
            .with_wrap(true),
    );

    let (sink, _) = read(write(&source, &ExportOptions::new()));

    let style = sink.get_style("C1").expect("the style should come back");

    assert!(style.bold);
    assert!(style.italic);
    assert!(style.underline);
    assert!(style.wrap);
    assert_eq!(style.align, HorizontalAlign::Right);
    assert_eq!(style.text_color, Some([0xFF, 0x00, 0x00]));

    // The value the style belonged to is still there.
    assert_eq!(sink.get("C1"), "loud");
}

#[test]
fn a_cell_with_no_style_comes_back_with_none() {
    let mut source = MemorySource::new();
    source.set("A1", "plain");

    let (sink, _) = read(write(&source, &ExportOptions::new()));

    assert_eq!(
        sink.get_style("A1"),
        None,
        "nothing was styled, so nothing comes back"
    );
}

#[test]
fn the_style_table_records_a_font_and_an_alignment() {
    let mut source = MemorySource::new();
    source.set("A1", "plain");
    source.set_styled(
        "B1",
        "loud",
        CellStyle::DEFAULT
            .with_bold(true)
            .with_align(HorizontalAlign::Center),
    );

    let bytes = write(&source, &ExportOptions::new());
    let mut archive = Archive::open(Cursor::new(bytes)).expect("the package should open");
    let styles = archive.read_text("xl/styles.xml").expect("the part should read");

    // The default face plus the bold one, and the plain record plus the styled one.
    assert!(styles.contains("<fonts count=\"2\">"), "{styles}");
    assert!(
        styles.contains("<b></b>"),
        "the bold font should be written: {styles}"
    );
    assert!(styles.contains("<cellXfs count=\"2\">"), "{styles}");
    assert!(styles.contains("applyFont=\"1\""), "{styles}");
    assert!(styles.contains("horizontal=\"center\""), "{styles}");
}

#[test]
fn a_workbook_of_several_sheets_round_trips() {
    let mut first = MemorySource::new();
    first.set("A1", "one");

    let mut second = MemorySource::new();
    second.set("A1", "two");
    second.set("B2", "42");

    let sheets = [
        SheetRef {
            name: "Alpha",
            source: &first,
        },
        SheetRef {
            name: "Beta",
            source: &second,
        },
    ];

    let mut buffer = Cursor::new(Vec::new());

    let result = XlsxWriter::new()
        .export_workbook(&mut buffer, "book.xlsx", &sheets, &ExportOptions::new())
        .expect("the workbook should write");

    assert_eq!(result.cells_written, 3, "every sheet's cells are counted");

    // The package holds a part for each sheet, and the workbook names both.
    let bytes = buffer.into_inner();
    let mut archive = Archive::open(Cursor::new(bytes.clone())).expect("the package should open");

    assert!(archive.has("xl/worksheets/sheet1.xml"));
    assert!(archive.has("xl/worksheets/sheet2.xml"));

    let workbook = archive
        .read_text("xl/workbook.xml")
        .expect("the part should read");
    assert!(workbook.contains("name=\"Alpha\""), "{workbook}");
    assert!(workbook.contains("name=\"Beta\""), "{workbook}");

    // And both sheets read back, in order.
    let mut sink = RecordingSink::new();
    let result = XlsxImporter::new()
        .import(&mut Cursor::new(bytes), &mut sink)
        .expect("the workbook should read");

    assert_eq!(sink.sheets, vec!["Alpha".to_string(), "Beta".to_string()]);
    assert_eq!(result.sheet_names, vec!["Alpha", "Beta"]);
    assert_eq!(result.worksheet_count, 2);
    assert_eq!(result.sheet_name.as_deref(), Some("Alpha"));
    assert_eq!(result.cells_imported, 3);
}

#[test]
fn a_fill_survives_the_round_trip() {
    let mut source = MemorySource::new();

    source.set_styled(
        "A1",
        "shaded",
        CellStyle::DEFAULT.with_fill(Some([0x00, 0x70, 0xC0])),
    );

    let bytes = write(&source, &ExportOptions::new());
    let (sink, _) = read(bytes.clone());

    let style = sink.get_style("A1").expect("the style should come back");
    assert_eq!(style.fill, Some([0x00, 0x70, 0xC0]));

    // The package carries the fill as a solid pattern after the two entries the specification reserves.
    let mut archive = Archive::open(Cursor::new(bytes)).expect("the package should open");
    let styles = archive.read_text("xl/styles.xml").expect("the part should read");

    assert!(styles.contains("<fills count=\"3\">"), "{styles}");
    assert!(styles.contains("patternType=\"solid\""), "{styles}");
    assert!(styles.contains("rgb=\"FF0070C0\""), "{styles}");
    assert!(styles.contains("applyFill=\"1\""), "{styles}");
}

/// A minimal workbook part, for hand-built packages.
fn workbook_part(sheet_name: &str) -> String {
    format!(
        concat!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>",
            "<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" ",
            "xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">",
            "<sheets><sheet name=\"{}\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>"
        ),
        sheet_name
    )
}
