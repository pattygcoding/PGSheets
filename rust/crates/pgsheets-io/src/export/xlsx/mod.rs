//! Writes the SpreadsheetML (xlsx) format: a zip archive of XML parts.
//!
//! Six parts are emitted and nothing else. Cells use inline strings, so no shared string table is
//! required, and a formula's result is written only when it is a value: an error is left uncached so
//! the reading application recalculates rather than inheriting this engine's `#NAME?` for a function
//! it does not implement.

mod worksheet;

use std::collections::HashMap;

use pgsheets_core::ExportSource;

use super::{ExportError, ExportOptions, ExportResult, SpreadsheetExporter};
use crate::ooxml::{
    CONTENT_TYPES_NAMESPACE, PACKAGE_RELATIONSHIP_NAMESPACE, RELATIONSHIP_NAMESPACE, sanitize_sheet_name,
};
use crate::xml::XmlWriter;
use crate::zip::{PackageWriter, WriteSeek};

use worksheet::write_worksheet;

const CONTENT_TYPES_PART: &str = "[Content_Types].xml";
const ROOT_RELATIONSHIPS_PART: &str = "_rels/.rels";
const WORKBOOK_PART: &str = "xl/workbook.xml";
const WORKBOOK_RELATIONSHIPS_PART: &str = "xl/_rels/workbook.xml.rels";
const STYLES_PART: &str = "xl/styles.xml";
const WORKSHEET_PART: &str = "xl/worksheets/sheet1.xml";

const OFFICE_DOCUMENT_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
const WORKSHEET_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";
const STYLES_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";

/// The first number format id the specification leaves to a workbook.
///
/// Everything below it is fixed by the specification, so custom codes are numbered from here up.
const FIRST_CUSTOM_FORMAT_ID: i32 = 164;

/// The distinct number formats the cells being written use, in the order they are first seen.
///
/// Style index 0 is always General, which is what a cell with no format refers to.
#[derive(Default)]
pub(crate) struct NumberFormatTable {
    indices: HashMap<String, i32>,
    codes: Vec<String>,
}

impl NumberFormatTable {
    /// The style index for a format code; 0, the default entry, when there is none.
    pub(crate) fn index_of(&mut self, code: Option<&str>) -> i32 {
        let Some(code) = code else {
            return 0;
        };

        if let Some(index) = self.indices.get(code) {
            return *index;
        }

        let index = self.codes.len() as i32 + 1;
        self.codes.push(code.to_string());
        self.indices.insert(code.to_string(), index);

        index
    }

    /// The distinct codes, in index order.
    pub(crate) fn codes(&self) -> &[String] {
        &self.codes
    }

    /// The style index a code already has; 0 — the default entry — when it has none.
    ///
    /// The formats were collected before either part was written, so the worksheet only ever looks an
    /// index up.
    pub(crate) fn index_of_known(&self, code: Option<&str>) -> i32 {
        match code {
            Some(code) => self.indices.get(code).copied().unwrap_or(0),
            None => 0,
        }
    }

    /// The `numFmtId` written for the format at a style index.
    pub(crate) fn number_format_id_at(&self, style_index: i32) -> i32 {
        FIRST_CUSTOM_FORMAT_ID + style_index - 1
    }
}

/// Writes a workbook.
#[derive(Default)]
pub struct XlsxWriter;

impl XlsxWriter {
    /// Creates the writer.
    pub fn new() -> Self {
        Self
    }
}

impl SpreadsheetExporter for XlsxWriter {
    fn extensions(&self) -> &'static [&'static str] {
        &[".xlsx"]
    }

    fn export(
        &self,
        writer: &mut dyn WriteSeek,
        _file_name: &str,
        source: &dyn ExportSource,
        options: &ExportOptions,
    ) -> Result<ExportResult, ExportError> {
        let mut result = ExportResult::new();
        let sheet_name = sanitize_sheet_name(Some(&options.sheet_name));

        // The style table has to know every format before either part is written, so the formats are
        // collected in one pass over the cell keys first.
        let number_formats = collect_number_formats(source, options);
        let worksheet = write_worksheet(source, options, &number_formats, &mut result);

        let mut package = PackageWriter::create(writer);

        package.write_part(CONTENT_TYPES_PART, &write_content_types())?;
        package.write_part(ROOT_RELATIONSHIPS_PART, &write_root_relationships())?;
        package.write_part(WORKBOOK_PART, &write_workbook(&sheet_name))?;
        package.write_part(WORKBOOK_RELATIONSHIPS_PART, &write_workbook_relationships())?;
        package.write_part(STYLES_PART, &write_styles(&number_formats))?;
        package.write_part(WORKSHEET_PART, &worksheet)?;
        package.finish()?;

        if options.sheet_name != sheet_name {
            result.add_warning(format!(
                "Worksheet name '{}' is not valid in xlsx; written as '{sheet_name}'.",
                options.sheet_name
            ));
        }

        Ok(result)
    }
}

/// `[Content_Types].xml`: what each part of the package is.
fn write_content_types() -> String {
    let mut xml = XmlWriter::new();
    xml.declaration();
    xml.start_in("Types", CONTENT_TYPES_NAMESPACE);

    write_default_type(
        &mut xml,
        "rels",
        "application/vnd.openxmlformats-package.relationships+xml",
    );
    write_default_type(&mut xml, "xml", "application/xml");

    write_override_type(
        &mut xml,
        "/xl/workbook.xml",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml",
    );
    write_override_type(
        &mut xml,
        "/xl/worksheets/sheet1.xml",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml",
    );
    write_override_type(
        &mut xml,
        "/xl/styles.xml",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml",
    );

    xml.end();
    xml.into_string()
}

/// `_rels/.rels`: the package's relationship to its one document part.
fn write_root_relationships() -> String {
    let mut xml = XmlWriter::new();
    xml.declaration();
    xml.start_in("Relationships", PACKAGE_RELATIONSHIP_NAMESPACE);
    write_relationship(&mut xml, "rId1", OFFICE_DOCUMENT_RELATIONSHIP, "xl/workbook.xml");
    xml.end();
    xml.into_string()
}

/// `xl/workbook.xml`: the one worksheet's name and id.
fn write_workbook(sheet_name: &str) -> String {
    let mut xml = XmlWriter::new();
    xml.declaration();
    xml.start("workbook");

    // The r: prefix has to be declared for the sheet's r:id attribute to resolve.
    xml.namespace_attribute("r", RELATIONSHIP_NAMESPACE);

    xml.start("sheets");
    xml.start("sheet");
    xml.attribute("name", sheet_name);
    xml.attribute("sheetId", "1");
    xml.qualified_attribute("r:id", "rId1");
    xml.end();
    xml.end();

    xml.end();
    xml.into_string()
}

/// `xl/_rels/workbook.xml.rels`: the workbook's relationship to the worksheet and the styles.
fn write_workbook_relationships() -> String {
    let mut xml = XmlWriter::new();
    xml.declaration();
    xml.start_in("Relationships", PACKAGE_RELATIONSHIP_NAMESPACE);
    write_relationship(&mut xml, "rId1", WORKSHEET_RELATIONSHIP, "worksheets/sheet1.xml");
    write_relationship(&mut xml, "rId2", STYLES_RELATIONSHIP, "styles.xml");
    xml.end();
    xml.into_string()
}

/// Collects the number formats the cells being written use, so the style table can name them and the
/// worksheet can refer to them by index.
fn collect_number_formats(source: &dyn ExportSource, options: &ExportOptions) -> NumberFormatTable {
    let mut table = NumberFormatTable::default();

    for address in source.populated_cells() {
        if options
            .range
            .is_some_and(|range| !range.contains(address.row, address.column))
        {
            continue;
        }

        table.index_of(source.number_format(address.row, address.column));
    }

    table
}

/// `xl/styles.xml`: enough of a style table for the number formats to mean something.
///
/// Excel is strict about this part — every `count` must match the number of children, there must be at
/// least two fills, and `cellStyleXfs` must hold a plain entry — and getting any of that wrong makes
/// Excel offer to repair the file.
fn write_styles(number_formats: &NumberFormatTable) -> String {
    let mut xml = XmlWriter::new();
    xml.declaration();
    xml.start("styleSheet");

    // The specification fixes the order of these children, and numFmts has to come first: a file with
    // them out of order is one Excel offers to repair.
    if !number_formats.codes().is_empty() {
        xml.start("numFmts");
        xml.attribute("count", &number_formats.codes().len().to_string());

        for (index, code) in number_formats.codes().iter().enumerate() {
            let style_index = index as i32 + 1;

            xml.start("numFmt");
            xml.attribute(
                "numFmtId",
                &number_formats.number_format_id_at(style_index).to_string(),
            );
            xml.attribute("formatCode", code);
            xml.end();
        }

        xml.end();
    }

    xml.start("fonts");
    xml.attribute("count", "1");
    xml.start("font");
    write_valued(&mut xml, "sz", "11");
    write_valued(&mut xml, "name", "Calibri");
    xml.end();
    xml.end();

    xml.start("fills");
    xml.attribute("count", "2");
    write_pattern_fill(&mut xml, "none");
    write_pattern_fill(&mut xml, "gray125");
    xml.end();

    xml.start("borders");
    xml.attribute("count", "1");
    xml.start("border");

    for side in ["left", "right", "top", "bottom", "diagonal"] {
        xml.start(side);
        xml.end();
    }

    xml.end();
    xml.end();

    xml.start("cellStyleXfs");
    xml.attribute("count", "1");
    write_format(&mut xml, "0", false, false);
    xml.end();

    xml.start("cellXfs");
    xml.attribute("count", &(number_formats.codes().len() + 1).to_string());

    // Entry 0 is General, which is what a cell with no number format refers to.
    write_format(&mut xml, "0", true, false);

    for style_index in 1..=number_formats.codes().len() as i32 {
        let id = number_formats.number_format_id_at(style_index).to_string();
        write_format(&mut xml, &id, true, true);
    }

    xml.end();

    xml.start("cellStyles");
    xml.attribute("count", "1");
    xml.start("cellStyle");
    xml.attribute("name", "Normal");
    xml.attribute("xfId", "0");
    xml.attribute("builtinId", "0");
    xml.end();
    xml.end();

    xml.end();
    xml.into_string()
}

fn write_default_type(xml: &mut XmlWriter, extension: &str, content_type: &str) {
    xml.start_in("Default", CONTENT_TYPES_NAMESPACE);
    xml.attribute("Extension", extension);
    xml.attribute("ContentType", content_type);
    xml.end();
}

fn write_override_type(xml: &mut XmlWriter, part_name: &str, content_type: &str) {
    xml.start_in("Override", CONTENT_TYPES_NAMESPACE);
    xml.attribute("PartName", part_name);
    xml.attribute("ContentType", content_type);
    xml.end();
}

fn write_relationship(xml: &mut XmlWriter, id: &str, relationship_type: &str, target: &str) {
    xml.start_in("Relationship", PACKAGE_RELATIONSHIP_NAMESPACE);
    xml.attribute("Id", id);
    xml.attribute("Type", relationship_type);
    xml.attribute("Target", target);
    xml.end();
}

fn write_valued(xml: &mut XmlWriter, element: &str, value: &str) {
    xml.start(element);
    xml.attribute("val", value);
    xml.end();
}

fn write_pattern_fill(xml: &mut XmlWriter, pattern_type: &str) {
    xml.start("fill");
    xml.start("patternFill");
    xml.attribute("patternType", pattern_type);
    xml.end();
    xml.end();
}

fn write_format(
    xml: &mut XmlWriter,
    num_fmt_id: &str,
    include_style_reference: bool,
    apply_number_format: bool,
) {
    xml.start("xf");
    xml.attribute("numFmtId", num_fmt_id);
    xml.attribute("fontId", "0");
    xml.attribute("fillId", "0");
    xml.attribute("borderId", "0");

    if include_style_reference {
        xml.attribute("xfId", "0");
    }

    if apply_number_format {
        xml.attribute("applyNumberFormat", "1");
    }

    xml.end();
}
