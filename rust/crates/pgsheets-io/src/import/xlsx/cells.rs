//! Converting a worksheet's cells into the text the model stores.

use pgsheets_core::CellAddress;

use super::styles::CellFormat;

/// Converts one worksheet cell into the raw input the model will store, together with the formatting
/// that gives that text its meaning and its look.
///
/// A date stays the serial the file holds: the format is what turns it back into a date, exactly as it
/// does in a spreadsheet, which is also what makes it behave like the number it is.
#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_cell(
    cell_type: Option<&str>,
    value: Option<&str>,
    formula: Option<&str>,
    inline_text: Option<&str>,
    style_attribute: Option<&str>,
    shared_strings: &[String],
    number_formats: &std::collections::HashMap<i32, CellFormat>,
    import_formulas: bool,
) -> (String, CellFormat) {
    let formatting = read_format(style_attribute, number_formats);
    let formula = formula.filter(|text| !text.is_empty());

    if import_formulas && let Some(formula) = formula {
        return (format!("={formula}"), formatting);
    }

    let raw = match cell_type {
        Some("s") => value
            .and_then(|value| value.parse::<usize>().ok())
            .and_then(|index| shared_strings.get(index).cloned())
            .unwrap_or_default(),

        Some("inlineStr") => inline_text.or(value).unwrap_or_default().to_string(),

        Some("b") => {
            if value == Some("1") {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }

        // Numeric cells — "n" or no type at all — and the cached values of formulas arrive in <v>.
        _ => value.unwrap_or_default().to_string(),
    };

    (raw, formatting)
}

/// The formatting a cell refers to with its `s` attribute, or the plain default when it has none.
fn read_format(
    style_attribute: Option<&str>,
    number_formats: &std::collections::HashMap<i32, CellFormat>,
) -> CellFormat {
    let Some(index) = style_attribute.and_then(|value| value.parse::<i32>().ok()) else {
        return CellFormat::default();
    };

    number_formats.get(&index).cloned().unwrap_or_default()
}

/// The function names a formula calls, so unsupported ones can be reported up front rather than
/// discovered as `#NAME?` later.
///
/// A name is only a call when `(` follows it, which is what keeps a reference such as `A1` out.
pub(super) fn extract_function_names(formula: &str) -> Vec<String> {
    let chars: Vec<char> = formula.chars().collect();
    let mut names = Vec::new();
    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index];

        if !ch.is_alphabetic() && ch != '_' {
            index += 1;
            continue;
        }

        let start = index;

        while index < chars.len() && (chars[index].is_alphanumeric() || matches!(chars[index], '_' | '.')) {
            index += 1;
        }

        let token: String = chars[start..index].iter().collect();

        if index >= chars.len() || chars[index] != '(' {
            continue;
        }

        // A cell reference is never followed by '('; guard against A1-style tokens anyway.
        if CellAddress::try_parse_a1(&token).is_some() {
            continue;
        }

        names.push(normalize_function_name(&token));
    }

    names
}

/// Strips the prefix Excel gives a function added after the original specification, such as
/// `_xlfn.CONCAT`.
fn normalize_function_name(token: &str) -> String {
    for prefix in ["_xlfn.", "_xlws."] {
        if token.len() > prefix.len() && token[..prefix.len()].eq_ignore_ascii_case(prefix) {
            return token[prefix.len()..].to_string();
        }
    }

    token.to_string()
}
