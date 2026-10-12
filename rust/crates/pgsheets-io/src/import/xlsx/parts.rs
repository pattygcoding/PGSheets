//! The workbook, relationship and shared-string parts.

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::XmlVersion;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;

use crate::zip::{Archive, ReadSeek};

use super::super::ImportResult;

/// One worksheet, as the workbook names it and as the package stores it.
pub struct SheetPart {
    /// The name shown on a tab.
    pub name: String,

    /// The path of the part inside the package.
    pub path: String,

    /// Whether this sheet is a worksheet. A chartsheet or a macro sheet is not, and cannot be
    /// represented as cells, so the importer skips it and the caller must not save over the file.
    pub is_worksheet: bool,
}

/// A relationship target together with the kind it claims to be.
pub struct Relationship {
    /// The resolved path of the part inside the package.
    pub target: String,

    /// The relationship type URI, which is what tells a worksheet from a chart sheet.
    pub kind: String,
}

/// The suffix every worksheet relationship type ends with.
const WORKSHEET_RELATIONSHIP_SUFFIX: &str = "/worksheet";

/// Reads the workbook part, resolving each sheet to the path its relationship points at.
pub fn read_workbook_sheets<R: ReadSeek>(
    archive: &mut Archive<R>,
    result: &mut ImportResult,
) -> Vec<SheetPart> {
    let relationships = read_relationships(archive, "xl/_rels/workbook.xml.rels", "xl/");
    let mut sheets = Vec::new();

    let Some(text) = archive.read_text("xl/workbook.xml") else {
        result.add_warning("xl/workbook.xml is missing from the archive.");
        return sheets;
    };

    for_each_element(&text, |name, attributes| {
        if name != "sheet" {
            return;
        }

        let sheet_name = attribute(attributes, "name").unwrap_or_else(|| "Sheet1".to_string());

        let relationship = attribute(attributes, "r:id").and_then(|id| relationships.get(&id));

        let path = relationship
            .map(|relationship| relationship.target.clone())
            .unwrap_or_else(|| "xl/worksheets/sheet1.xml".to_string());

        // A sheet with no relationship to check is assumed to be a worksheet, which is what every
        // workbook written here and by Excel has.
        let is_worksheet = relationship
            .map(|relationship| relationship.kind.ends_with(WORKSHEET_RELATIONSHIP_SUFFIX))
            .unwrap_or(true);

        sheets.push(SheetPart {
            name: sheet_name,
            path,
            is_worksheet,
        });
    });

    sheets
}

/// Reads a `.rels` part into an id-to-relationship map.
pub fn read_relationships<R: ReadSeek>(
    archive: &mut Archive<R>,
    part_path: &str,
    base_directory: &str,
) -> HashMap<String, Relationship> {
    let mut map = HashMap::new();

    let Some(text) = archive.read_text(part_path) else {
        return map;
    };

    for_each_element(&text, |name, attributes| {
        if name != "Relationship" {
            return;
        }

        let (Some(id), Some(target)) = (attribute(attributes, "Id"), attribute(attributes, "Target")) else {
            return;
        };

        // A target starting with '/' is absolute within the package; anything else is relative to the
        // part that owns the relationship.
        let resolved = match target.strip_prefix('/') {
            Some(absolute) => absolute.to_string(),
            None => format!("{base_directory}{target}"),
        };

        let kind = attribute(attributes, "Type").unwrap_or_default();

        map.insert(
            id,
            Relationship {
                target: resolved,
                kind,
            },
        );
    });

    map
}

/// Reads the shared string table.
///
/// Every `<t>` run inside a `<si>` is concatenated, so rich text is flattened to the text it carries.
pub fn read_shared_strings<R: ReadSeek>(archive: &mut Archive<R>) -> Vec<String> {
    let Some(text) = archive.read_text("xl/sharedStrings.xml") else {
        return Vec::new();
    };

    let mut strings = Vec::new();
    let mut reader = Reader::from_str(&text);
    let mut inside_string = false;
    let mut inside_text = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let name = local_name(element.name());

                match name {
                    "si" => {
                        inside_string = true;
                        strings.push(String::new());
                    }
                    "t" if inside_string => inside_text = true,
                    _ => {}
                }
            }

            Ok(Event::End(element)) => match local_name(element.name()) {
                "si" => inside_string = false,
                "t" => inside_text = false,
                _ => {}
            },

            Ok(Event::Text(raw)) if inside_text => {
                if let Some(current) = strings.last_mut() {
                    current.push_str(&xml_text(&raw));
                }
            }

            // An entity or character reference arrives as its own event, so it has to be resolved and
            // appended rather than left out of the string.
            Ok(Event::GeneralRef(raw)) if inside_text => {
                if let Some(current) = strings.last_mut() {
                    current.push_str(&resolve_reference(&raw));
                }
            }

            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    strings
}

/// The value of an attribute, by its qualified name (`r:id` is not `id`).
fn attribute(attributes: &[(String, String)], name: &str) -> Option<String> {
    attributes
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.clone())
}

/// The local name of an element, without its prefix.
pub(crate) fn local_name(name: QName<'_>) -> &str {
    // The name borrows the part it was parsed from, not the element that points at it, which is what
    // lets a caller keep it while the element goes out of scope.
    name.local_name().into_inner()
}

/// The text of an element, with its entities resolved.
pub(crate) fn xml_text(raw: &quick_xml::events::BytesText<'_>) -> String {
    raw.xml_content(XmlVersion::Implicit1_0).into_owned()
}

/// Resolves an entity or character reference to the text it stands for.
///
/// A reference arrives as its own event carrying the name between the delimiters, so `&lt;` turns up as
/// `lt` and has to be turned back into `<`. A name this reader does not know is left as it was rather
/// than silently dropped — a document type is refused outright, so an unknown entity means a damaged
/// part rather than a document with a custom entity in it.
pub(crate) fn resolve_reference(raw: &quick_xml::events::BytesRef<'_>) -> String {
    let name: &str = raw.as_ref();

    match name {
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "amp" => "&".to_string(),
        "quot" => "\"".to_string(),
        "apos" => "'".to_string(),
        _ => character_reference(name)
            .map(|ch| ch.to_string())
            .unwrap_or_else(|| format!("&{name};")),
    }
}

/// A character reference such as `#65` or `#x41`.
fn character_reference(name: &str) -> Option<char> {
    let number = name.strip_prefix('#')?;

    let code = match number.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => number.parse::<u32>().ok()?,
    };

    char::from_u32(code)
}

/// Reads a part, calling `visit` for every element that starts or stands alone.
pub(crate) fn for_each_element(text: &str, mut visit: impl FnMut(&str, &[(String, String)])) {
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) | Ok(Event::Empty(element)) => {
                visit(local_name(element.name()), &attributes_of(&element));
            }

            // A document type is a security hazard, not a document: it can describe entities, and an
            // external one can make a reader fetch something. Files here are untrusted input, so a
            // part that carries one is refused outright rather than parsed carefully.
            Ok(Event::DocType(_)) => return,

            Ok(Event::Eof) | Err(_) => return,
            _ => {}
        }
    }
}

/// Every attribute of an element, by qualified name, normalised as XML requires.
fn attributes_of(element: &BytesStart<'_>) -> Vec<(String, String)> {
    element
        .attributes()
        .flatten()
        .map(|attribute| {
            let key = attribute.key.as_ref().to_string();
            let value = attribute
                .normalized_value(XmlVersion::Implicit1_0)
                .unwrap_or_default()
                .into_owned();

            (key, value)
        })
        .collect()
}
