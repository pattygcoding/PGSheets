//! The style table, as far as a number format, a font and an alignment go.

use std::collections::HashMap;

use pgsheets_core::{CellStyle, HorizontalAlign};
use quick_xml::Reader;
use quick_xml::XmlVersion;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;

use crate::zip::{Archive, ReadSeek};

/// What a cell's style index resolves to: the number format that gives its value meaning and the
/// styling that goes with it.
#[derive(Clone, Debug, Default)]
pub struct CellFormat {
    /// The number format code, or `None` for General.
    pub number_format: Option<String>,

    /// The styling; [`CellStyle::DEFAULT`] when the record carries none this reader understands.
    pub style: CellStyle,
}

/// One `<xf>`: the font, fill and number format it refers to, and any alignment it carries.
#[derive(Clone, Copy, Debug, Default)]
struct XfRecord {
    num_fmt_id: Option<i32>,
    font_id: i32,
    fill_id: i32,
    align: HorizontalAlign,
    wrap: bool,
}

/// The section of the part that is currently open, so a bare element can be attributed to it.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Section {
    #[default]
    Other,
    NumFmts,
    Fonts,
    Fills,
    CellXfs,
}

/// Everything one walk of the style part collects, including the record currently open.
#[derive(Default)]
struct Collected {
    custom: HashMap<i32, String>,
    fonts: Vec<CellStyle>,
    fills: Vec<Option<[u8; 3]>>,
    xfs: Vec<XfRecord>,

    /// The `<font>` currently open, or `None`.
    font: Option<CellStyle>,

    /// The `<fill>` currently open: the outer `Some` while it is open, the inner value the colour found
    /// inside it, which is `None` for a pattern that paints nothing.
    fill: Option<Option<[u8; 3]>>,

    /// The `<xf>` currently open, or `None`.
    xf: Option<XfRecord>,

    section: Section,
}

/// The number format ids the SpreadsheetML specification fixes.
///
/// A workbook normally writes its own codes into `numFmts` instead, but a minimal file refers to
/// these ids.
pub fn built_in_number_formats() -> HashMap<i32, &'static str> {
    HashMap::from([
        (1, "0"),
        (2, "0.00"),
        (3, "#,##0"),
        (4, "#,##0.00"),
        (9, "0%"),
        (10, "0.00%"),
        (11, "0.00E+00"),
        (12, "# ?/?"),
        (13, "# ??/??"),
        (14, "mm-dd-yy"),
        (15, "d-mmm-yy"),
        (16, "d-mmm"),
        (17, "mmm-yy"),
        (18, "h:mm AM/PM"),
        (19, "h:mm:ss AM/PM"),
        (20, "h:mm"),
        (21, "h:mm:ss"),
        (22, "m/d/yy h:mm"),
        (37, "#,##0 ;(#,##0)"),
        (38, "#,##0 ;[Red](#,##0)"),
        (39, "#,##0.00;(#,##0.00)"),
        (40, "#,##0.00;[Red](#,##0.00)"),
        (45, "mm:ss"),
        (46, "[h]:mm:ss"),
        (47, "mmss.0"),
        (48, "##0.0E+0"),
        (49, "@"),
    ])
}

/// Reads the formatting of every cell style, keyed by the index a cell refers to with its `s`
/// attribute.
///
/// Three sections matter and they are read in one walk: `numFmts` holds the codes the workbook
/// defined, `fonts` holds the text attributes a font id stands for, and `cellXfs` is what maps a cell
/// to one of each, together with any alignment. Everything else in the part is ignored.
pub fn read_cell_formats<R: ReadSeek>(archive: &mut Archive<R>) -> HashMap<i32, CellFormat> {
    let Some(text) = archive.read_text("xl/styles.xml") else {
        return HashMap::new();
    };

    let collected = read_styles(&text);

    resolve(
        collected.custom,
        &collected.fonts,
        &collected.fills,
        collected.xfs,
    )
}

/// Walks the style part once, collecting custom codes, fonts, fills and cell records in order.
fn read_styles(text: &str) -> Collected {
    let mut reader = Reader::from_str(text);
    let mut collected = Collected::default();

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) | Ok(Event::Empty(element)) => {
                open_element(&mut collected, element.name(), &element);
            }

            Ok(Event::End(element)) => {
                close_element(&mut collected, element.name().local_name().into_inner())
            }

            // A document type is a security hazard, not a document: a part that carries one is refused
            // outright rather than parsed carefully.
            Ok(Event::DocType(_)) => return collected,
            Ok(Event::Eof) | Err(_) => return collected,
            _ => {}
        }
    }
}

/// Handles one opening or standalone element, updating the section and the record open within it.
fn open_element(state: &mut Collected, name: QName<'_>, element: &BytesStart<'_>) {
    match name.local_name().into_inner() {
        "numFmts" => state.section = Section::NumFmts,
        "fonts" => state.section = Section::Fonts,
        "fills" => state.section = Section::Fills,
        "cellXfs" => state.section = Section::CellXfs,
        "cellStyleXfs" => state.section = Section::Other,

        "numFmt" if state.section == Section::NumFmts => {
            let id = attribute(element, "numFmtId").and_then(|value| value.parse().ok());
            let code = attribute(element, "formatCode");

            if let (Some(id), Some(code)) = (id, code) {
                state.custom.insert(id, code);
            }
        }

        // A font with no children is the default face, which is what `<font/>` means.
        "font" if state.section == Section::Fonts => state.font = Some(CellStyle::DEFAULT),

        "b" if state.font.is_some() => state.font.as_mut().unwrap().bold = true,
        "i" if state.font.is_some() => state.font.as_mut().unwrap().italic = true,
        "u" if state.font.is_some() => state.font.as_mut().unwrap().underline = true,

        "color" if state.font.is_some() => {
            if let Some(color) = attribute(element, "rgb").and_then(|rgb| parse_rgb(&rgb)) {
                state.font.as_mut().unwrap().text_color = Some(color);
            }
        }

        // A fill is opened before its `<patternFill>`, so it starts without a colour and the `fgColor` of
        // a solid fill is what fills it in; a pattern that paints nothing leaves it `None`.
        "fill" if state.section == Section::Fills => state.fill = Some(None),

        "fgColor" if state.fill.is_some() => {
            if let Some(color) = attribute(element, "rgb").and_then(|rgb| parse_rgb(&rgb)) {
                state.fill = Some(Some(color));
            }
        }

        "xf" if state.section == Section::CellXfs => {
            state.xf = Some(XfRecord {
                num_fmt_id: attribute(element, "numFmtId").and_then(|value| value.parse().ok()),
                font_id: attribute(element, "fontId")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0),
                fill_id: attribute(element, "fillId")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0),
                align: HorizontalAlign::General,
                wrap: false,
            });
        }

        "alignment" if state.xf.is_some() => {
            let record = state.xf.as_mut().unwrap();

            if let Some(horizontal) = attribute(element, "horizontal") {
                record.align = parse_align(&horizontal);
            }

            record.wrap = attribute(element, "wrapText").is_some_and(|value| truthy(&value));
        }

        _ => {}
    }
}

/// Handles a closing element, finishing whichever record it ends.
fn close_element(state: &mut Collected, name: &str) {
    match name {
        "font" => {
            if let Some(font) = state.font.take() {
                state.fonts.push(font);
            }
        }

        "fill" => state.fills.push(state.fill.take().flatten()),

        "xf" => {
            if let Some(record) = state.xf.take() {
                state.xfs.push(record);
            }
        }

        "fonts" | "fills" | "cellXfs" | "numFmts" | "cellStyleXfs" => state.section = Section::Other,
        _ => {}
    }
}

/// Turns the raw records into per-index formatting, mapping font ids to fonts and numFmt ids to codes.
fn resolve(
    custom: HashMap<i32, String>,
    fonts: &[CellStyle],
    fills: &[Option<[u8; 3]>],
    xfs: Vec<XfRecord>,
) -> HashMap<i32, CellFormat> {
    let built_in = built_in_number_formats();
    let mut formats = HashMap::new();

    for (index, record) in xfs.into_iter().enumerate() {
        let number_format = record.num_fmt_id.and_then(|id| {
            custom
                .get(&id)
                .cloned()
                .or_else(|| built_in.get(&id).map(|code| (*code).to_string()))
        });

        let mut style = record
            .font_id
            .try_into()
            .ok()
            .and_then(|font_id: usize| fonts.get(font_id).copied())
            .unwrap_or_default();

        style.align = record.align;
        style.wrap = record.wrap;
        style.fill = fills.get(record.fill_id.max(0) as usize).copied().flatten();

        // An entry that says nothing is left out, so the worksheet treats it as formatting-free.
        if number_format.is_some() || !style.is_default() {
            formats.insert(index as i32, CellFormat { number_format, style });
        }
    }

    formats
}

/// Parses an ARGB colour such as `FFFF0000` or a bare `FF0000` into red, green and blue.
fn parse_rgb(value: &str) -> Option<[u8; 3]> {
    let hex = match value.len() {
        8 => &value[2..],
        6 => value,
        _ => return None,
    };

    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some([red, green, blue])
}

/// The horizontal alignment a value names; anything unrecognised falls back to General.
fn parse_align(value: &str) -> HorizontalAlign {
    match value {
        "left" => HorizontalAlign::Left,
        "center" => HorizontalAlign::Center,
        "right" => HorizontalAlign::Right,
        _ => HorizontalAlign::General,
    }
}

/// True for the two spellings XML accepts for a boolean attribute.
fn truthy(value: &str) -> bool {
    matches!(value, "1" | "true" | "TRUE" | "True")
}

/// Reads one attribute by name.
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
