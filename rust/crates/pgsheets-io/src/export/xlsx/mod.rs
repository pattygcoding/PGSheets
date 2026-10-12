//! Writes the SpreadsheetML (xlsx) format: a zip archive of XML parts.
//!
//! Six parts are emitted and nothing else. Cells use inline strings, so no shared string table is
//! required, and a formula's result is written only when it is a value: an error is left uncached so
//! the reading application recalculates rather than inheriting this engine's `#NAME?` for a function
//! it does not implement.

mod worksheet;

use std::collections::HashMap;

use pgsheets_core::{CellStyle, ExportSource, HorizontalAlign};

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

/// The distinct number formats, fonts and alignments the cells being written use, in the order they are
/// first seen.
///
/// Style index 0 is always the plain entry — General, the default font, no alignment — which is what a
/// cell with no formatting refers to. Everything after it is a distinct combination, so two cells that
/// differ only in bold share a font and differ only in their `xf`.
#[derive(Default)]
pub(crate) struct StyleTable {
    /// The distinct number format codes, numbered from 0; the `numFmtId` written is offset by the first
    /// custom id.
    codes: Vec<String>,
    code_indices: HashMap<String, i32>,

    /// The distinct fonts, numbered from 0; index 0 is the default font.
    fonts: Vec<CellStyle>,
    font_indices: HashMap<CellStyle, i32>,

    /// The distinct style records, numbered from 0; index 0 is the plain entry.
    xfs: Vec<StyleKey>,
    xf_indices: HashMap<StyleKey, i32>,
}

/// One style record: a cell's number format, font and alignment together, which is exactly what an
/// `<xf>` holds.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct StyleKey {
    /// Index into [`StyleTable::fonts`].
    font: i32,

    /// A custom number format id, or 0 for General.
    num_fmt_id: i32,

    /// Horizontal alignment.
    align: HorizontalAlign,

    /// Whether the text wraps.
    wrap: bool,
}

impl StyleTable {
    /// A table seeded with the plain style at index 0, which every cell with no formatting refers to.
    pub(crate) fn new() -> Self {
        let mut table = Self::default();

        // Font 0 is the default font and xf 0 is the plain entry, so a cell written without an `s`
        // attribute means "no formatting".
        table.fonts.push(CellStyle::DEFAULT);
        table.font_indices.insert(CellStyle::DEFAULT, 0);

        let plain = StyleKey {
            font: 0,
            num_fmt_id: 0,
            align: HorizontalAlign::General,
            wrap: false,
        };

        table.xfs.push(plain.clone());
        table.xf_indices.insert(plain, 0);

        table
    }

    /// Notes a cell's number format and style, returning nothing; the worksheet looks the index up
    /// afterwards with [`StyleTable::index_of`].
    pub(crate) fn insert(&mut self, number_format: Option<&str>, style: Option<CellStyle>) {
        let style = style.unwrap_or_default();

        let num_fmt_id = match number_format {
            Some(code) => self.intern_code(code),
            None => 0,
        };

        let font = self.intern_font(style);

        let key = StyleKey {
            font,
            num_fmt_id,
            align: style.align,
            wrap: style.wrap,
        };

        if let std::collections::hash_map::Entry::Vacant(entry) = self.xf_indices.entry(key.clone()) {
            entry.insert(self.xfs.len() as i32);
            self.xfs.push(key);
        }
    }

    /// The style index for a cell's number format and style; 0, the plain entry, when it has neither.
    pub(crate) fn index_of(&self, number_format: Option<&str>, style: Option<CellStyle>) -> i32 {
        let style = style.unwrap_or_default();

        let num_fmt_id = match number_format {
            Some(code) => match self.code_indices.get(code) {
                Some(index) => self.number_format_id_at(*index),
                None => 0,
            },
            None => 0,
        };

        let font = self.font_indices.get(&font_key(style)).copied().unwrap_or(0);

        let key = StyleKey {
            font,
            num_fmt_id,
            align: style.align,
            wrap: style.wrap,
        };

        self.xf_indices.get(&key).copied().unwrap_or(0)
    }

    /// The distinct number format codes, in index order.
    pub(crate) fn codes(&self) -> &[String] {
        &self.codes
    }

    /// The distinct fonts, in index order.
    pub(crate) fn fonts(&self) -> &[CellStyle] {
        &self.fonts
    }

    /// The distinct style records, in index order.
    pub(crate) fn styles(&self) -> &[StyleKey] {
        &self.xfs
    }

    /// The `numFmtId` a cell with this style record refers to.
    pub(crate) fn number_format_id_of(&self, key: &StyleKey) -> i32 {
        key.num_fmt_id
    }

    /// The `numFmtId` written for the code at an index, counting from 0.
    pub(crate) fn number_format_id_at(&self, code_index: i32) -> i32 {
        FIRST_CUSTOM_FORMAT_ID + code_index
    }

    fn intern_code(&mut self, code: &str) -> i32 {
        if let Some(index) = self.code_indices.get(code) {
            return self.number_format_id_at(*index);
        }

        let index = self.codes.len() as i32;
        self.codes.push(code.to_string());
        self.code_indices.insert(code.to_string(), index);

        self.number_format_id_at(index)
    }

    fn intern_font(&mut self, style: CellStyle) -> i32 {
        // Only the font-carrying flags matter here; alignment and the number format live on the `xf`.
        let font_key = font_key(style);

        if let Some(index) = self.font_indices.get(&font_key) {
            return *index;
        }

        let index = self.fonts.len() as i32;
        self.fonts.push(font_key);
        self.font_indices.insert(font_key, index);

        index
    }
}

/// The part of a style a font is made of: the text attributes, without the alignment or the number
/// format, which belong on the `xf` rather than on the `<font>`.
fn font_key(style: CellStyle) -> CellStyle {
    CellStyle {
        align: HorizontalAlign::General,
        wrap: false,
        ..style
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

        // The style table has to know every format, font and alignment before either part is written,
        // so they are collected in one pass over the cell keys first.
        let styles = collect_styles(source, options);
        let worksheet = write_worksheet(source, options, &styles, &mut result);

        let mut package = PackageWriter::create(writer);

        package.write_part(CONTENT_TYPES_PART, &write_content_types())?;
        package.write_part(ROOT_RELATIONSHIPS_PART, &write_root_relationships())?;
        package.write_part(WORKBOOK_PART, &write_workbook(&sheet_name))?;
        package.write_part(WORKBOOK_RELATIONSHIPS_PART, &write_workbook_relationships())?;
        package.write_part(STYLES_PART, &write_styles(&styles))?;
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

/// Collects the number formats, fonts and alignments the cells being written use, so the style table
/// can name them and the worksheet can refer to them by index.
fn collect_styles(source: &dyn ExportSource, options: &ExportOptions) -> StyleTable {
    let mut table = StyleTable::new();

    for address in source.populated_cells() {
        if options
            .range
            .is_some_and(|range| !range.contains(address.row, address.column))
        {
            continue;
        }

        table.insert(
            source.number_format(address.row, address.column),
            source.cell_style(address.row, address.column),
        );
    }

    table
}

/// `xl/styles.xml`: the number formats, fonts and alignments every cell in the sheet refers to.
///
/// Excel is strict about this part — every `count` must match the number of children, there must be at
/// least two fills, and `cellStyleXfs` must hold a plain entry — and getting any of that wrong makes
/// Excel offer to repair the file.
fn write_styles(styles: &StyleTable) -> String {
    let mut xml = XmlWriter::new();
    xml.declaration();
    xml.start("styleSheet");

    // The specification fixes the order of these children, and numFmts has to come first: a file with
    // them out of order is one Excel offers to repair.
    if !styles.codes().is_empty() {
        xml.start("numFmts");
        xml.attribute("count", &styles.codes().len().to_string());

        for (index, code) in styles.codes().iter().enumerate() {
            xml.start("numFmt");
            xml.attribute("numFmtId", &styles.number_format_id_at(index as i32).to_string());
            xml.attribute("formatCode", code);
            xml.end();
        }

        xml.end();
    }

    xml.start("fonts");
    xml.attribute("count", &styles.fonts().len().to_string());

    for font in styles.fonts() {
        write_font(&mut xml, *font);
    }

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
    write_format(&mut xml, "0", 0, false, false, HorizontalAlign::General, false);
    xml.end();

    xml.start("cellXfs");
    xml.attribute("count", &styles.styles().len().to_string());

    for key in styles.styles() {
        write_format(
            &mut xml,
            &styles.number_format_id_of(key).to_string(),
            key.font,
            true,
            key.num_fmt_id != 0,
            key.align,
            key.wrap,
        );
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

/// Writes one `<font>`: the text attributes a style carries, on top of the default face.
fn write_font(xml: &mut XmlWriter, font: CellStyle) {
    xml.start("font");

    if font.bold {
        xml.start("b");
        xml.end();
    }

    if font.italic {
        xml.start("i");
        xml.end();
    }

    if font.underline {
        xml.start("u");
        xml.end();
    }

    if let Some([red, green, blue]) = font.text_color {
        xml.start("color");
        xml.attribute("rgb", &format!("FF{red:02X}{green:02X}{blue:02X}"));
        xml.end();
    }

    write_valued(xml, "sz", "11");
    write_valued(xml, "name", "Calibri");
    xml.end();
}

/// Writes one `<xf>`, the record a cell's `s` attribute points at.
///
/// The `xfId` is only present on the `cellXfs` entries: `cellStyleXfs` holds the named cell styles and
/// must not name one.
#[allow(clippy::too_many_arguments)]
fn write_format(
    xml: &mut XmlWriter,
    num_fmt_id: &str,
    font_id: i32,
    include_style_reference: bool,
    apply_number_format: bool,
    align: HorizontalAlign,
    wrap: bool,
) {
    xml.start("xf");
    xml.attribute("numFmtId", num_fmt_id);
    xml.attribute("fontId", &font_id.to_string());
    xml.attribute("fillId", "0");
    xml.attribute("borderId", "0");

    if include_style_reference {
        xml.attribute("xfId", "0");
    }

    if font_id != 0 {
        xml.attribute("applyFont", "1");
    }

    if apply_number_format {
        xml.attribute("applyNumberFormat", "1");
    }

    let horizontal = horizontal_name(align);

    if horizontal.is_some() || wrap {
        xml.attribute("applyAlignment", "1");
        xml.start("alignment");

        if let Some(horizontal) = horizontal {
            xml.attribute("horizontal", horizontal);
        }

        if wrap {
            xml.attribute("wrapText", "1");
        }

        xml.end();
    }

    xml.end();
}

/// The value Excel gives a horizontal alignment, or `None` for General — which is left off rather than
/// spelled out, so a cell with no alignment renders as one that never mentioned it.
fn horizontal_name(align: HorizontalAlign) -> Option<&'static str> {
    match align {
        HorizontalAlign::General => None,
        HorizontalAlign::Left => Some("left"),
        HorizontalAlign::Center => Some("center"),
        HorizontalAlign::Right => Some("right"),
    }
}
