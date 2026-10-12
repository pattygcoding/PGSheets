//! Interprets raw, non-formula cell input (`"1200"`, `"North"`, `"TRUE"`) as a typed value.
//!
//! Kept in one place so the engine, the grid and the file readers all agree on the rules.

use crate::numeric::parse_number;
use crate::value::CellValue;

/// Interprets raw input as a typed value.
///
/// A value that looks like a number becomes one, `TRUE`/`FALSE` (in any case) becomes a boolean, and
/// everything else stays text — including anything starting with `=`, which is a formula that never
/// reached the parser. Surfacing it as text rather than guessing is deliberate.
pub fn parse_literal(raw: &str) -> CellValue {
    if raw.is_empty() {
        return CellValue::Empty;
    }

    if raw.starts_with('=') {
        return CellValue::text(raw);
    }

    if let Some(number) = parse_number(raw) {
        return CellValue::number(number);
    }

    if raw.eq_ignore_ascii_case("TRUE") {
        return CellValue::boolean(true);
    }

    if raw.eq_ignore_ascii_case("FALSE") {
        return CellValue::boolean(false);
    }

    CellValue::text(raw)
}
