//! Text functions.
//!
//! A numeric argument is coerced through its display form, so `LEN(1234)` is 4 and `LEFT(1234,2)` is
//! `"12"`, which is what a spreadsheet does.
//!
//! **Everything here counts UTF-16 code units, not characters.** That is the unit a spreadsheet counts
//! in — `LEN("😀")` is 2 — and it is the unit .NET's `string` indexes by, so a port that counted
//! Rust's `chars()` instead would disagree with the original on any text outside the basic plane. The
//! conversions are explicit for that reason.

use pgsheets_core::{CellError, CellValue};

use super::helpers::{MAX_TEXT_LENGTH, try_integer, try_text, visit_values};
use crate::ast::{AstNode, FormulaContext};

/// The length of text in UTF-16 code units.
fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// A slice of text by UTF-16 code-unit offset and length.
fn utf16_slice(text: &str, from: usize, length: usize) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    let start = from.min(units.len());
    let end = (from + length).min(units.len());

    String::from_utf16_lossy(&units[start..end])
}

pub(super) fn len(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() != 1 {
        return CellValue::error(CellError::Value);
    }

    match try_text(arguments, 0, context) {
        Ok(text) => CellValue::number(utf16_len(&text) as f64),
        Err(error) => error,
    }
}

pub(super) fn upper(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    transform(arguments, context, str::to_uppercase)
}

pub(super) fn lower(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    transform(arguments, context, str::to_lowercase)
}

pub(super) fn proper(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    transform(arguments, context, to_proper_case)
}

pub(super) fn trim(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    transform(arguments, context, collapse_spaces)
}

/// Applies a one-argument text transformation.
fn transform(
    arguments: &[AstNode],
    context: &mut dyn FormulaContext,
    operation: fn(&str) -> String,
) -> CellValue {
    if arguments.len() != 1 {
        return CellValue::error(CellError::Value);
    }

    match try_text(arguments, 0, context) {
        Ok(text) => CellValue::text(&operation(&text)),
        Err(error) => error,
    }
}

/// Capitalises the first letter of each word and lowers the rest.
fn to_proper_case(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut start_of_word = true;

    for ch in text.chars() {
        if ch.is_alphabetic() {
            // Rust's case mapping is the full Unicode mapping rather than .NET's simple one, which
            // differs only for a handful of ligatures: PROPER of "ß" is "SS" here and "ß" there.
            if start_of_word {
                result.extend(ch.to_uppercase());
            } else {
                result.extend(ch.to_lowercase());
            }

            start_of_word = false;
            continue;
        }

        result.push(ch);
        start_of_word = true;
    }

    result
}

/// `TRIM` removes the ends and collapses runs of spaces inside to one, which is more than trimming
/// whitespace does and is what a spreadsheet offers.
fn collapse_spaces(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut pending_space = false;

    for ch in text.chars() {
        if ch == ' ' {
            // Only worth remembering once something has been written.
            pending_space = !result.is_empty();
            continue;
        }

        if pending_space {
            result.push(' ');
            pending_space = false;
        }

        result.push(ch);
    }

    result
}

pub(super) fn left(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    take_characters(arguments, context, false)
}

pub(super) fn right(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    take_characters(arguments, context, true)
}

/// `LEFT` and `RIGHT` differ only in which end they take from.
fn take_characters(arguments: &[AstNode], context: &mut dyn FormulaContext, from_end: bool) -> CellValue {
    if arguments.is_empty() || arguments.len() > 2 {
        return CellValue::error(CellError::Value);
    }

    let text = match try_text(arguments, 0, context) {
        Ok(text) => text,
        Err(error) => return error,
    };

    let count = if arguments.len() == 2 {
        match try_integer(arguments, 1, context) {
            Ok(count) => count,
            Err(error) => return error,
        }
    } else {
        1
    };

    if count < 0 {
        return CellValue::error(CellError::Value);
    }

    let length = utf16_len(&text);
    let count = count as usize;

    if count >= length {
        return CellValue::text(&text);
    }

    let from = if from_end { length - count } else { 0 };
    CellValue::text(&utf16_slice(&text, from, count))
}

/// `MID(text, start, length)` — start is 1-based and must be at least 1.
pub(super) fn mid(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() != 3 {
        return CellValue::error(CellError::Value);
    }

    let text = match try_text(arguments, 0, context) {
        Ok(text) => text,
        Err(error) => return error,
    };

    let start = match try_integer(arguments, 1, context) {
        Ok(start) => start,
        Err(error) => return error,
    };

    let length = match try_integer(arguments, 2, context) {
        Ok(length) => length,
        Err(error) => return error,
    };

    if start < 1 || length < 0 {
        return CellValue::error(CellError::Value);
    }

    let from = (start - 1) as usize;

    if from >= utf16_len(&text) {
        // Asking past the end is empty text, not an error.
        return CellValue::text("");
    }

    CellValue::text(&utf16_slice(&text, from, length as usize))
}

/// `TEXTJOIN(delimiter, ignoreEmpty, value, ...)` — the values may be ranges.
pub(super) fn text_join(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() < 3 {
        return CellValue::error(CellError::Value);
    }

    let delimiter = match try_text(arguments, 0, context) {
        Ok(text) => text,
        Err(error) => return error,
    };

    let ignore_empty_value = arguments[1].evaluate(context);

    if ignore_empty_value.is_error() {
        return ignore_empty_value;
    }

    let ignore_empty = ignore_empty_value.as_boolean();
    let mut joined = String::new();
    let mut appended = false;
    let mut error = None;

    for argument in &arguments[2..] {
        visit_values(argument, context, &mut |value| {
            if value.is_error() {
                error = Some(value);
                return false;
            }

            if ignore_empty && value.is_empty() {
                return true;
            }

            if appended {
                joined.push_str(&delimiter);
            }

            joined.push_str(&value.as_text());
            appended = true;
            true
        });

        if let Some(failure) = error {
            return failure;
        }
    }

    CellValue::text(&joined)
}

pub(super) fn rept(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() != 2 {
        return CellValue::error(CellError::Value);
    }

    let text = match try_text(arguments, 0, context) {
        Ok(text) => text,
        Err(error) => return error,
    };

    let count = match try_integer(arguments, 1, context) {
        Ok(count) => count,
        Err(error) => return error,
    };

    if count < 0 {
        return CellValue::error(CellError::Value);
    }

    // One cell can only hold so much, so an absurd count is refused rather than allocated.
    if utf16_len(&text) * count as usize > MAX_TEXT_LENGTH {
        return CellValue::error(CellError::Value);
    }

    CellValue::text(&text.repeat(count as usize))
}

/// `SUBSTITUTE(text, old, new, [instance])` replaces all, or only the nth occurrence.
pub(super) fn substitute(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() < 3 || arguments.len() > 4 {
        return CellValue::error(CellError::Value);
    }

    let text = match try_text(arguments, 0, context) {
        Ok(text) => text,
        Err(error) => return error,
    };

    let old_text = match try_text(arguments, 1, context) {
        Ok(text) => text,
        Err(error) => return error,
    };

    let new_text = match try_text(arguments, 2, context) {
        Ok(text) => text,
        Err(error) => return error,
    };

    if old_text.is_empty() {
        // Nothing to look for, so the text passes through unchanged.
        return CellValue::text(&text);
    }

    if arguments.len() == 3 {
        return CellValue::text(&text.replace(&*old_text, &new_text));
    }

    let instance = match try_integer(arguments, 3, context) {
        Ok(instance) => instance,
        Err(error) => return error,
    };

    if instance < 1 {
        return CellValue::error(CellError::Value);
    }

    CellValue::text(&replace_occurrence(
        &text,
        &old_text,
        &new_text,
        instance as usize,
    ))
}

/// Replaces the nth (1-based) occurrence only, leaving the others alone.
fn replace_occurrence(text: &str, old_text: &str, new_text: &str, instance: usize) -> String {
    let mut found = None;
    let mut search_from = 0;

    for _ in 0..instance {
        match text[search_from..].find(old_text) {
            Some(offset) => {
                found = Some(search_from + offset);
                search_from += offset + old_text.len();
            }
            // Fewer occurrences than asked for: the text comes back unchanged.
            None => return text.to_string(),
        }
    }

    let Some(index) = found else {
        return text.to_string();
    };

    let mut result = String::with_capacity(text.len() + new_text.len());
    result.push_str(&text[..index]);
    result.push_str(new_text);
    result.push_str(&text[index + old_text.len()..]);
    result
}

/// `EXACT` compares two strings character for character, not case-insensitively.
pub(super) fn exact(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() != 2 {
        return CellValue::error(CellError::Value);
    }

    let left = match try_text(arguments, 0, context) {
        Ok(text) => text,
        Err(error) => return error,
    };

    let right = match try_text(arguments, 1, context) {
        Ok(text) => text,
        Err(error) => return error,
    };

    CellValue::boolean(left == right)
}

/// `VALUE` parses text into a number, refusing anything a spreadsheet would not accept.
pub(super) fn value(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() != 1 {
        return CellValue::error(CellError::Value);
    }

    let text = match try_text(arguments, 0, context) {
        Ok(text) => text,
        Err(error) => return error,
    };

    // The same coercion every numeric argument gets, so VALUE("1,234") is 1234.
    match CellValue::shared_text(text).as_number() {
        Some(number) => CellValue::number(number),
        None => CellValue::error(CellError::Value),
    }
}

/// `CHAR` turns a code in the 1..255 range into its character.
pub(super) fn character(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() != 1 {
        return CellValue::error(CellError::Value);
    }

    let code = match try_integer(arguments, 0, context) {
        Ok(code) => code,
        Err(error) => return error,
    };

    if !(1..=255).contains(&code) {
        return CellValue::error(CellError::Value);
    }

    match char::from_u32(code as u32) {
        Some(ch) => CellValue::text(&ch.to_string()),
        None => CellValue::error(CellError::Value),
    }
}

/// `CODE` is the code of the first character, in UTF-16 code units.
pub(super) fn code(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() != 1 {
        return CellValue::error(CellError::Value);
    }

    let text = match try_text(arguments, 0, context) {
        Ok(text) => text,
        Err(error) => return error,
    };

    // A lone surrogate has no `char`, which is why this reads the code unit rather than a character.
    match text.encode_utf16().next() {
        Some(unit) => CellValue::number(f64::from(unit)),
        None => CellValue::error(CellError::Value),
    }
}

/// `FIND` is case sensitive and 1-based, and reports a miss as `#VALUE!`.
pub(super) fn find(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() < 2 || arguments.len() > 3 {
        return CellValue::error(CellError::Value);
    }

    let needle = match try_text(arguments, 0, context) {
        Ok(text) => text,
        Err(error) => return error,
    };

    let haystack = match try_text(arguments, 1, context) {
        Ok(text) => text,
        Err(error) => return error,
    };

    let start = if arguments.len() == 3 {
        match try_integer(arguments, 2, context) {
            Ok(start) => start,
            Err(error) => return error,
        }
    } else {
        1
    };

    let length = utf16_len(&haystack);

    // One past the end is allowed, so FIND can report that nothing follows the text.
    if start < 1 || start as usize > length + 1 {
        return CellValue::error(CellError::Value);
    }

    let units: Vec<u16> = haystack.encode_utf16().collect();
    let needle_units: Vec<u16> = needle.encode_utf16().collect();
    let from = (start - 1) as usize;

    if needle_units.is_empty() {
        return CellValue::number((from + 1) as f64);
    }

    let position = units[from.min(units.len())..]
        .windows(needle_units.len())
        .position(|window| window == needle_units.as_slice());

    match position {
        Some(offset) => CellValue::number((from + offset + 1) as f64),
        None => CellValue::error(CellError::Value),
    }
}
