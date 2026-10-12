//! Shared SpreadsheetML (OOXML) details: namespaces, and how arbitrary text and a sheet name are made
//! safe to write.
//!
//! The reader and the writer have to agree exactly on these, so both use this module rather than
//! repeating the values and risking drift.

use std::borrow::Cow;

/// Namespace of the workbook, worksheet and styles parts.
pub const MAIN_NAMESPACE: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// Namespace of the `r:id` attributes that link a workbook to its parts.
pub const RELATIONSHIP_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// Namespace of `[Content_Types].xml`.
pub const CONTENT_TYPES_NAMESPACE: &str = "http://schemas.openxmlformats.org/package/2006/content-types";

/// Namespace of every `.rels` part.
pub const PACKAGE_RELATIONSHIP_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/package/2006/relationships";

/// The `xml:space` attribute that keeps leading and trailing spaces in a string.
pub const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

/// Longest worksheet name a workbook may contain.
pub const MAX_SHEET_NAME_LENGTH: usize = 31;

/// Removes the characters XML 1.0 cannot represent.
///
/// Cell content is arbitrary user data, and one illegal character is enough to make the whole part
/// unreadable, so text is always filtered before it is written. The C# original also had to cope with
/// unpaired UTF-16 surrogates; a Rust `&str` is always valid Unicode, so no such character can reach
/// here. Returns the same string when nothing needs changing.
pub fn sanitize_text(text: &str) -> Cow<'_, str> {
    let mut keep = 0;

    for (index, ch) in text.char_indices() {
        if is_legal_xml_char(ch) {
            keep = index + ch.len_utf8();
            continue;
        }

        // Something has to go, so build the cleaned text and copy the rest across.
        let mut cleaned = String::with_capacity(text.len());
        cleaned.push_str(&text[..index]);
        cleaned.extend(sanitize_rest(&text[keep..]));

        return Cow::Owned(cleaned);
    }

    Cow::Borrowed(text)
}

fn sanitize_rest(text: &str) -> impl Iterator<Item = char> + '_ {
    text.chars().filter(|ch| is_legal_xml_char(*ch))
}

/// True for the characters XML 1.0 allows, which is what the original kept.
fn is_legal_xml_char(ch: char) -> bool {
    matches!(ch, '\t' | '\n' | '\r')
        || ('\u{20}'..='\u{d7ff}').contains(&ch)
        || ('\u{e000}'..='\u{fffd}').contains(&ch)
        || ch >= '\u{10000}'
}

/// Reduces an arbitrary string to a legal worksheet name.
///
/// At most 31 characters, none of `[]:*?/\`, and no characters XML cannot carry. Falls back to
/// `Sheet1` when nothing usable is left.
pub fn sanitize_sheet_name(name: Option<&str>) -> String {
    let Some(name) = name else {
        return "Sheet1".to_string();
    };

    if name.trim().is_empty() {
        return "Sheet1".to_string();
    }

    let mut cleaned = String::with_capacity(name.len().min(MAX_SHEET_NAME_LENGTH));

    for ch in name.chars() {
        if matches!(ch, '[' | ']' | ':' | '*' | '?' | '/' | '\\') || !is_legal_xml_char(ch) {
            continue;
        }

        cleaned.push(ch);

        if cleaned.chars().count() == MAX_SHEET_NAME_LENGTH {
            break;
        }
    }

    let cleaned = cleaned.trim();

    if cleaned.is_empty() {
        "Sheet1".to_string()
    } else {
        cleaned.to_string()
    }
}
