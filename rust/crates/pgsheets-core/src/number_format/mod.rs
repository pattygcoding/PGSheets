//! Renders a value the way a workbook's number format says it should: the `0.00`, `#,##0`, `0%`,
//! `$#,##0.00` and `yyyy-mm-dd` codes a cell carries.
//!
//! The format language is large, so this module interprets the parts a workbook actually uses and
//! refuses the rest: a code containing anything it cannot represent falls back to the plain
//! rendering, so a format can never make a cell harder to read than no format at all. Sections are
//! separated by `;` (positive; negative; zero; text) and the one that applies is chosen from the
//! sign of the value.

mod dates;
mod numbers;

use crate::numeric::general_number;
use numbers::Pattern;

/// The code an unformatted cell has, and the name of the plain rendering.
pub const GENERAL: &str = "General";

/// Formats a value with a format code, falling back to the plain rendering when there is no format
/// or the format cannot be represented.
pub fn apply(value: f64, format: Option<&str>) -> String {
    try_apply(value, format).unwrap_or_else(|| general_number(value))
}

/// The rendering of a number with no format applied.
pub fn apply_general(value: f64) -> String {
    general_number(value)
}

/// True when the code means "no formatting", so the plain rendering applies.
pub fn is_general(format: Option<&str>) -> bool {
    match format {
        None => true,
        Some(text) => text.trim().is_empty() || text.trim().eq_ignore_ascii_case(GENERAL),
    }
}

/// `None` means "no format applies", which the caller turns into the plain rendering.
fn try_apply(value: f64, format: Option<&str>) -> Option<String> {
    let format = format?;

    if is_general(Some(format)) {
        return None;
    }

    let sections = split_sections(format);

    if sections.is_empty() || sections.len() > 4 {
        return None;
    }

    let index = choose_section(&sections, value);

    if sections[index].trim().is_empty() {
        // An empty section is how a workbook hides a value: "0.00;" shows nothing negative.
        return Some(String::new());
    }

    Pattern::parse(&sections[index])?.render(value, sections.len())
}

/// Splits a code on `;`, ignoring separators inside quotes or brackets.
fn split_sections(format: &str) -> Vec<String> {
    let mut sections = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut in_brackets = false;

    for ch in format.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                current.push(ch);
            }
            '[' if !in_quotes => {
                in_brackets = true;
                current.push(ch);
            }
            ']' if !in_quotes => {
                in_brackets = false;
                current.push(ch);
            }
            ';' if !in_quotes && !in_brackets => sections.push(std::mem::take(&mut current)),
            _ => current.push(ch),
        }
    }

    sections.push(current);
    sections
}

/// The index of the section that applies to a value: one section covers everything, two split the
/// negative from the rest, and three or four add a separate section for zero.
fn choose_section(sections: &[String], value: f64) -> usize {
    match sections.len() {
        1 => 0,
        2 => {
            if value < 0.0 {
                1
            } else {
                0
            }
        }
        _ => {
            if value > 0.0 {
                0
            } else if value < 0.0 {
                1
            } else {
                2
            }
        }
    }
}

/// True when a section is a date pattern.
///
/// Letters inside quotes or brackets are literal text or a directive, so `0.00 "days"` and
/// `[Red]0.00` are both numbers.
fn is_date_pattern(section: &str) -> bool {
    let chars: Vec<char> = section.chars().collect();
    let mut in_quotes = false;
    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index];
        index += 1;

        match ch {
            '"' => in_quotes = !in_quotes,
            _ if in_quotes => {}
            // An escaped character, a reserved width or a fill: skip what follows.
            '\\' | '_' | '*' => index += 1,
            '[' => {
                // Skip to the closing bracket, or to the end when there is none.
                while index < chars.len() && chars[index] != ']' {
                    index += 1;
                }

                index += 1;
            }
            'y' | 'Y' | 'm' | 'M' | 'd' | 'D' | 'h' | 'H' | 's' | 'S' => return true,
            _ => {}
        }
    }

    false
}
