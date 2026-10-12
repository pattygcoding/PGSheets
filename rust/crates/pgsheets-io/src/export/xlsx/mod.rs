//! Writes the SpreadsheetML (xlsx) format: a zip archive of XML parts.
//!
//! Six parts are emitted and nothing else. Cells use inline strings, so no shared string table is
//! required, and a formula's result is written only when it is a value: an error is left uncached so
//! the reading application recalculates rather than inheriting this engine's `#NAME?` for a function
//! it does not implement.

mod worksheet;

use std::collections::HashMap;

use pgsheets_core::{CellStyle, ExportSource, HorizontalAlign};

use super::{ExportError, ExportOptions, ExportResult, SheetRef, SpreadsheetExporter};
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

/// The content type of a worksheet part.
const WORKSHEET_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";

/// The part a worksheet lives in, numbered from one: the first sheet is `sheet1.xml`.
fn worksheet_part(index: usize) -> String {
    format!("xl/worksheets/sheet{}.xml", index + 1)
}

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

    /// The distinct background fills, numbered from 0; indices 0 and 1 are the two the specification
    /// reserves, so the first real fill is index 2.
    fills: Vec<Option<[u8; 3]>>,
    fill_indices: HashMap<Option<[u8; 3]>, i32>,

    /// The distinct style records, numbered from 0; index 0 is the plain entry.
    xfs: Vec<StyleKey>,
    xf_indices: HashMap<StyleKey, i32>,
}

/// One style record: a cell's number format, font, fill and alignment together, which is exactly what
/// an `<xf>` holds.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct StyleKey {
    /// Index into [`StyleTable::fonts`].
    font: i32,

    /// Index into [`StyleTable::fills`].
    fill: i32,

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

        // The specification reserves two fills: 0 is `none` and 1 is `gray125`. Neither is a real fill,
        // and Excel expects both, so they are seeded here and a real fill is interned from index 2.
        table.fills.push(None);
        table.fills.push(None);
        table.fill_indices.insert(None, 0);

        let plain = StyleKey {
            font: 0,
            fill: 0,
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
        let fill = self.intern_fill(style.fill);

        let key = StyleKey {
            font,
            fill,
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
        let fill = self.fill_indices.get(&style.fill).copied().unwrap_or(0);

        let key = StyleKey {
            font,
            fill,
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

    /// The distinct fills, in index order; the first two are the reserved slots.
    pub(crate) fn fills(&self) -> &[Option<[u8; 3]>] {
        &self.fills
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

    fn intern_fill(&mut self, fill: Option<[u8; 3]>) -> i32 {
        if let Some(index) = self.fill_indices.get(&fill) {
            return *index;
        }

        let index = self.fills.len() as i32;
        self.fills.push(fill);
        self.fill_indices.insert(fill, index);

        index
    }
}

/// The part of a style a font is made of: the text attributes, without the alignment, the fill or the
/// number format, which belong elsewhere in the style record.
fn font_key(style: CellStyle) -> CellStyle {
    CellStyle {
        align: HorizontalAlign::General,
        fill: None,
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
        file_name: &str,
        source: &dyn ExportSource,
        options: &ExportOptions,
    ) -> Result<ExportResult, ExportError> {
        let sheets = [SheetRef {
            name: &options.sheet_name,
            source,
        }];

        self.export_workbook(writer, file_name, &sheets, options)
    }

    fn export_workbook(
        &self,
        writer: &mut dyn WriteSeek,
        _file_name: &str,
        sheets: &[SheetRef<'_>],
        options: &ExportOptions,
    ) -> Result<ExportResult, ExportError> {
        if sheets.is_empty() {
            return Err(ExportError::Io("the document holds no worksheets".to_string()));
        }

        let mut result = ExportResult::new();

        // Every name is sanitised first, so the sheet records and the parts agree on what each is
        // called, and a name that had to change is reported.
        let names: Vec<String> = sheets
            .iter()
            .map(|sheet| sanitize_sheet_name(Some(sheet.name)))
            .collect();

        for (sheet, name) in sheets.iter().zip(&names) {
            if sheet.name != name {
                result.add_warning(format!(
                    "Worksheet name '{}' is not valid in xlsx; written as '{name}'.",
                    sheet.name
                ));
            }
        }

        // The style table has to know every format, font and alignment before any part is written, so
        // it is collected in one pass over every sheet's cells first and shared between them.
        let mut styles = StyleTable::new();

        for sheet in sheets {
            collect_styles_into(&mut styles, sheet.source, options);
        }

        let mut worksheets = Vec::with_capacity(sheets.len());

        for sheet in sheets {
            let mut sheet_result = ExportResult::new();
            worksheets.push(write_worksheet(sheet.source, options, &styles, &mut sheet_result));

            result.cells_written += sheet_result.cells_written;
            result.rows = result.rows.max(sheet_result.rows);
            result.columns = result.columns.max(sheet_result.columns);
        }

        let mut package = PackageWriter::create(writer);

        package.write_part(CONTENT_TYPES_PART, &write_content_types(names.len()))?;
        package.write_part(ROOT_RELATIONSHIPS_PART, &write_root_relationships())?;
        package.write_part(WORKBOOK_PART, &write_workbook(&names))?;
        package.write_part(
            WORKBOOK_RELATIONSHIPS_PART,
            &write_workbook_relationships(names.len()),
        )?;
        package.write_part(STYLES_PART, &write_styles(&styles))?;

        for (index, worksheet) in worksheets.iter().enumerate() {
            package.write_part(&worksheet_part(index), worksheet)?;
        }

        package.finish()?;

        Ok(result)
    }
}

/// `[Content_Types].xml`: what each part of the package is.
fn write_content_types(sheet_count: usize) -> String {
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

    for index in 0..sheet_count {
        write_override_type(
            &mut xml,
            &format!("/{}", worksheet_part(index)),
            WORKSHEET_CONTENT_TYPE,
        );
    }

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

/// `xl/workbook.xml`: every worksheet's name, id and relationship.
fn write_workbook(names: &[String]) -> String {
    let mut xml = XmlWriter::new();
    xml.declaration();
    xml.start("workbook");

    // The r: prefix has to be declared for the sheet's r:id attribute to resolve.
    xml.namespace_attribute("r", RELATIONSHIP_NAMESPACE);

    xml.start("sheets");

    for (index, name) in names.iter().enumerate() {
        xml.start("sheet");
        xml.attribute("name", name);
        xml.attribute("sheetId", &(index + 1).to_string());
        xml.qualified_attribute("r:id", &format!("rId{}", index + 1));
        xml.end();
    }

    xml.end();

    xml.end();
    xml.into_string()
}

/// `xl/_rels/workbook.xml.rels`: the workbook's relationships to its worksheets and the styles.
fn write_workbook_relationships(sheet_count: usize) -> String {
    let mut xml = XmlWriter::new();
    xml.declaration();
    xml.start_in("Relationships", PACKAGE_RELATIONSHIP_NAMESPACE);

    for index in 0..sheet_count {
        write_relationship(
            &mut xml,
            &format!("rId{}", index + 1),
            WORKSHEET_RELATIONSHIP,
            &format!("worksheets/sheet{}.xml", index + 1),
        );
    }

    // The styles relationship follows the worksheets, so its id depends on how many there are.
    write_relationship(
        &mut xml,
        &format!("rId{}", sheet_count + 1),
        STYLES_RELATIONSHIP,
        "styles.xml",
    );

    xml.end();
    xml.into_string()
}

/// Collects the number formats, fonts and alignments a sheet's cells use into a shared table, so the
/// style table can name them and any worksheet can refer to them by index.
fn collect_styles_into(table: &mut StyleTable, source: &dyn ExportSource, options: &ExportOptions) {
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

    // The first two fills are the ones the specification reserves — `none` and `gray125` — and every
    // real fill follows them, which is why a cell's fill index is never 0 or 1.
    xml.start("fills");
    xml.attribute("count", &styles.fills().len().to_string());
    write_pattern_fill(&mut xml, "none");
    write_pattern_fill(&mut xml, "gray125");

    for fill in styles.fills().get(2..).unwrap_or_default() {
        write_solid_fill(&mut xml, fill.unwrap_or([0, 0, 0]));
    }

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
    write_format(&mut xml, "0", 0, 0, false, false, HorizontalAlign::General, false);
    xml.end();

    xml.start("cellXfs");
    xml.attribute("count", &styles.styles().len().to_string());

    for key in styles.styles() {
        write_format(
            &mut xml,
            &styles.number_format_id_of(key).to_string(),
            key.font,
            key.fill,
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

/// Writes one `<fill>` that paints a cell a solid colour.
fn write_solid_fill(xml: &mut XmlWriter, [red, green, blue]: [u8; 3]) {
    xml.start("fill");
    xml.start("patternFill");
    xml.attribute("patternType", "solid");

    xml.start("fgColor");
    xml.attribute("rgb", &format!("FF{red:02X}{green:02X}{blue:02X}"));
    xml.end();

    // The background colour has to be present for Excel to accept a solid fill; index 64 is
    // "automatic", which is what a spreadsheet writes for one it was not told.
    xml.start("bgColor");
    xml.attribute("indexed", "64");
    xml.end();

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
    fill_id: i32,
    include_style_reference: bool,
    apply_number_format: bool,
    align: HorizontalAlign,
    wrap: bool,
) {
    xml.start("xf");
    xml.attribute("numFmtId", num_fmt_id);
    xml.attribute("fontId", &font_id.to_string());
    xml.attribute("fillId", &fill_id.to_string());
    xml.attribute("borderId", "0");

    if include_style_reference {
        xml.attribute("xfId", "0");
    }

    if font_id != 0 {
        xml.attribute("applyFont", "1");
    }

    if fill_id != 0 {
        xml.attribute("applyFill", "1");
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
