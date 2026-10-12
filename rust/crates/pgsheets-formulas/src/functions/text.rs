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

use super::helpers::{MAX_TEXT_LENGTH, optional_integer, try_integer, try_text, visit_values};
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

/// `SEARCH(find, within, [start])` — like `FIND`, but case insensitive and honouring the `*` and `?`
/// wildcards.
pub(super) fn search(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
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

    let start = match optional_integer(arguments, 2, context, 1) {
        Ok(start) => start,
        Err(error) => return error,
    };

    let units: Vec<u16> = haystack.encode_utf16().collect();
    let pattern: Vec<u16> = needle.encode_utf16().collect();

    // One past the end is allowed, so SEARCH can report that nothing follows the text.
    if start < 1 || start as usize > units.len() + 1 {
        return CellValue::error(CellError::Value);
    }

    let from = (start - 1) as usize;

    if pattern.is_empty() {
        return CellValue::number((from + 1) as f64);
    }

    for position in from..=units.len() {
        if wildcard_prefix(&units[position..], &pattern) {
            return CellValue::number((position + 1) as f64);
        }
    }

    CellValue::error(CellError::Value)
}

/// True when a wildcard pattern matches a prefix of the text.
///
/// `SEARCH` reports where the pattern starts rather than whether it covers the whole string, so this
/// is the same two-pointer walk the criteria matcher uses but with a laxer success test: the pattern
/// has to be consumed, the text does not.
fn wildcard_prefix(text: &[u16], pattern: &[u16]) -> bool {
    const STAR: u16 = b'*' as u16;

    let mut text_index = 0;
    let mut pattern_index = 0;
    let mut star: Option<usize> = None;
    let mut star_text = 0;

    while text_index < text.len() {
        if pattern_index == pattern.len() {
            // The pattern matched; the text left over is simply not part of it.
            return true;
        }

        if let Some(next) = wildcard_token(pattern, pattern_index, text[text_index]) {
            pattern_index = next;
            text_index += 1;
            continue;
        }

        if pattern.get(pattern_index) == Some(&STAR) {
            star = Some(pattern_index);
            star_text = text_index;
            pattern_index += 1;
            continue;
        }

        let Some(star) = star else {
            return false;
        };

        star_text += 1;
        text_index = star_text;
        pattern_index = star + 1;
    }

    // Trailing stars match nothing at all.
    while pattern.get(pattern_index) == Some(&STAR) {
        pattern_index += 1;
    }

    pattern_index == pattern.len()
}

/// Consumes the pattern token at `position` if it matches a code unit, returning where the pattern
/// continues. Stars are the caller's business, because they match a variable number of units.
fn wildcard_token(pattern: &[u16], position: usize, unit: u16) -> Option<usize> {
    const TILDE: u16 = b'~' as u16;
    const STAR: u16 = b'*' as u16;
    const QUESTION: u16 = b'?' as u16;

    let token = *pattern.get(position)?;

    if token == TILDE && position + 1 < pattern.len() {
        return same_unit(unit, pattern[position + 1]).then_some(position + 2);
    }

    if token == STAR {
        return None;
    }

    if token == QUESTION || same_unit(unit, token) {
        return Some(position + 1);
    }

    None
}

/// Case-insensitive comparison of two UTF-16 code units.
fn same_unit(left: u16, right: u16) -> bool {
    if left == right {
        return true;
    }

    match (char::from_u32(u32::from(left)), char::from_u32(u32::from(right))) {
        (Some(left), Some(right)) => left.to_uppercase().eq(right.to_uppercase()),
        _ => false,
    }
}

/// `REPLACE(text, start, length, new)` — swaps a run of characters, counted from 1, for new text.
pub(super) fn replace(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() != 4 {
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

    let count = match try_integer(arguments, 2, context) {
        Ok(count) => count,
        Err(error) => return error,
    };

    let new_text = match try_text(arguments, 3, context) {
        Ok(text) => text,
        Err(error) => return error,
    };

    if start < 1 || count < 0 {
        return CellValue::error(CellError::Value);
    }

    let length = utf16_len(&text);
    let from = (start - 1) as usize;

    let mut result = String::new();
    result.push_str(&utf16_slice(&text, 0, from.min(length)));
    result.push_str(&new_text);

    let after = from.saturating_add(count as usize);
    let tail = after.min(length);
    result.push_str(&utf16_slice(&text, tail, length - tail));

    CellValue::text(&result)
}

/// Which side of a delimiter `TEXTBEFORE` or `TEXTAFTER` keeps.
#[derive(Clone, Copy)]
enum Side {
    Before,
    After,
}

/// `TEXTBEFORE(text, delimiter, [instance], [match_mode], [match_end], [if_not_found])`.
pub(super) fn text_before(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    split_on_delimiter(arguments, context, Side::Before)
}

/// `TEXTAFTER(text, delimiter, [instance], [match_mode], [match_end], [if_not_found])`.
pub(super) fn text_after(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    split_on_delimiter(arguments, context, Side::After)
}

/// The shared body of `TEXTBEFORE` and `TEXTAFTER`.
///
/// `instance` is the 1-based occurrence, counting from the start, or from the end when negative.
/// `match_mode` `1` ignores case, and `match_end` `1` treats the end of the text as a delimiter so a
/// missing delimiter falls back to the whole string rather than to an error.
fn split_on_delimiter(arguments: &[AstNode], context: &mut dyn FormulaContext, side: Side) -> CellValue {
    if arguments.len() < 2 || arguments.len() > 6 {
        return CellValue::error(CellError::Value);
    }

    let text = match try_text(arguments, 0, context) {
        Ok(text) => text,
        Err(error) => return error,
    };

    let delimiter = match try_text(arguments, 1, context) {
        Ok(text) => text,
        Err(error) => return error,
    };

    if delimiter.is_empty() {
        // Every position is a delimiter, so there is no single answer.
        return CellValue::error(CellError::Value);
    }

    let instance = match optional_integer(arguments, 2, context, 1) {
        Ok(instance) => instance,
        Err(error) => return error,
    };

    if instance == 0 {
        return CellValue::error(CellError::Value);
    }

    let match_mode = match optional_integer(arguments, 3, context, 0) {
        Ok(mode) => mode,
        Err(error) => return error,
    };

    let match_end = match optional_integer(arguments, 4, context, 0) {
        Ok(mode) => mode,
        Err(error) => return error,
    };

    let chars: Vec<char> = text.chars().collect();
    let delimiter_chars: Vec<char> = delimiter.chars().collect();
    let ignore_case = match_mode == 1;

    // Non-overlapping occurrences, left to right, which is how a spreadsheet counts them.
    let mut positions = Vec::new();
    let mut index = 0;

    while index + delimiter_chars.len() <= chars.len() {
        if matches_at(&chars, index, &delimiter_chars, ignore_case) {
            positions.push(index);
            index += delimiter_chars.len();
        } else {
            index += 1;
        }
    }

    let chosen = if instance > 0 {
        positions.get((instance - 1) as usize).copied()
    } else {
        let from_end = (-instance) as usize;

        if positions.len() >= from_end {
            positions.get(positions.len() - from_end).copied()
        } else {
            None
        }
    };

    let (start, after_delimiter) = match chosen {
        Some(start) => (start, start + delimiter_chars.len()),
        None if match_end == 1 => (chars.len(), chars.len()),
        None => {
            return match arguments.get(5) {
                Some(fallback) => fallback.evaluate(context),
                None => CellValue::error(CellError::NotAvailable),
            };
        }
    };

    let result: String = match side {
        Side::Before => chars[..start].iter().collect(),
        Side::After => chars[after_delimiter..].iter().collect(),
    };

    CellValue::text(&result)
}

/// True when every character of `delimiter` matches the text at `start`, optionally ignoring case.
fn matches_at(text: &[char], start: usize, delimiter: &[char], ignore_case: bool) -> bool {
    for (offset, expected) in delimiter.iter().enumerate() {
        let actual = text[start + offset];

        let equal = if ignore_case {
            actual.to_lowercase().eq(expected.to_lowercase())
        } else {
            actual == *expected
        };

        if !equal {
            return false;
        }
    }

    true
}

/// `TEXT(value, format)` — the value rendered with a number-format code.
///
/// The code is the same language a cell's own number format uses, so a code the interpreter cannot
/// represent falls back to the plain rendering rather than to an error. A value that is already text
/// passes straight through, as it does in a spreadsheet.
pub(super) fn text_format(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() != 2 {
        return CellValue::error(CellError::Value);
    }

    let value = arguments[0].evaluate(context);

    if value.is_error() {
        return value;
    }

    let format = match try_text(arguments, 1, context) {
        Ok(text) => text,
        Err(error) => return error,
    };

    match value {
        CellValue::Number(number) => {
            CellValue::text(&pgsheets_core::number_format::apply(number, Some(&format)))
        }
        other => CellValue::shared_text(other.as_text()),
    }
}
