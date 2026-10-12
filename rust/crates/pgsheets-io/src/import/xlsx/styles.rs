//! The style table, as far as a number format goes.

use std::collections::HashMap;

use crate::zip::{Archive, ReadSeek};

use super::parts::for_each_element;

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

/// Reads the number format of every cell style, keyed by the index a cell refers to with its `s`
/// attribute.
///
/// Both halves of the style table matter: `numFmts` holds the codes the workbook defined, and
/// `cellXfs` is what maps a cell to one of them or to a built-in id.
pub fn read_number_formats<R: ReadSeek>(archive: &mut Archive<R>) -> HashMap<i32, String> {
    let mut formats = HashMap::new();

    let Some(text) = archive.read_text("xl/styles.xml") else {
        return formats;
    };

    let mut custom: HashMap<i32, String> = HashMap::new();
    let mut index = 0;

    read_styles(&text, &mut custom, &mut formats, &mut index);

    formats
}

/// Walks the style part once, collecting the workbook's own codes and the cell formats in order.
fn read_styles(
    text: &str,
    custom: &mut HashMap<i32, String>,
    formats: &mut HashMap<i32, String>,
    index: &mut i32,
) {
    // Only direct children of the root count, which is why the containing element is tracked rather
    // than every element being treated alike.
    let mut section = String::new();
    let built_in = built_in_number_formats();

    for_each_element(text, |name, attributes| {
        match name {
            "numFmts" | "cellXfs" => {
                section = name.to_string();
                return;
            }
            _ => {}
        }

        match (section.as_str(), name) {
            ("numFmts", "numFmt") => {
                let id = attribute(attributes, "numFmtId").and_then(|value| value.parse().ok());
                let code = attribute(attributes, "formatCode");

                if let (Some(id), Some(code)) = (id, code) {
                    custom.insert(id, code);
                }
            }

            ("cellXfs", "xf") => {
                let id = attribute(attributes, "numFmtId").and_then(|value| value.parse::<i32>().ok());

                if let Some(id) = id {
                    // A code the workbook defined wins; otherwise a built-in id, and an id this reader
                    // does not know is left alone rather than guessed at, so the cell keeps the plain
                    // rendering instead of being given a wrong one.
                    if let Some(code) = custom.get(&id) {
                        formats.insert(*index, code.clone());
                    } else if let Some(code) = built_in.get(&id) {
                        formats.insert(*index, (*code).to_string());
                    }
                }

                *index += 1;
            }

            _ => {}
        }
    });
}

/// Reads one attribute by name.
fn attribute(attributes: &[(String, String)], name: &str) -> Option<String> {
    attributes
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.clone())
}
