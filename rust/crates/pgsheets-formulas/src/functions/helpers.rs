//! Shared argument handling for the built-in functions.
//!
//! Every function reports a bad argument as `#VALUE!` and propagates an error it is given, so those
//! rules live here rather than being repeated in eighty implementations.

use std::cmp::Ordering;
use std::rc::Rc;

use pgsheets_core::{CellError, CellValue};

use crate::ast::{AstNode, FormulaContext};

/// Excel's limit on the characters one cell can hold, used to refuse an absurd `REPT`.
pub(crate) const MAX_TEXT_LENGTH: usize = 32_767;

/// Every value an argument contributes: a range contributes each of its cells, anything else exactly
/// one value.
///
/// `visit` returns false to stop early, which is how a function returns the first error it meets.
pub(crate) fn visit_values(
    node: &AstNode,
    context: &mut dyn FormulaContext,
    visit: &mut dyn FnMut(CellValue) -> bool,
) {
    match node {
        AstNode::Range(range) => {
            for address in range.cells() {
                if !visit(context.cell_value(address.row, address.column)) {
                    return;
                }
            }
        }
        other => {
            visit(other.evaluate(context));
        }
    }
}

/// Evaluates an argument, requiring a number.
///
/// `Err` is the error to return: `#VALUE!` for a missing or non-numeric argument, or the argument's
/// own error, which propagates.
pub(crate) fn try_number(
    arguments: &[AstNode],
    index: usize,
    context: &mut dyn FormulaContext,
) -> Result<f64, CellValue> {
    let Some(argument) = arguments.get(index) else {
        return Err(CellValue::error(CellError::Value));
    };

    let value = argument.evaluate(context);

    if value.is_error() {
        return Err(value);
    }

    value
        .as_number()
        .ok_or_else(|| CellValue::error(CellError::Value))
}

/// Evaluates an argument and rounds it to an integer, as a count or index argument.
pub(crate) fn try_integer(
    arguments: &[AstNode],
    index: usize,
    context: &mut dyn FormulaContext,
) -> Result<i32, CellValue> {
    let number = try_number(arguments, index, context)?;

    // The original rounds half away from zero; Rust's `round` does the same.
    Ok(number.round().clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32)
}

/// Evaluates an optional integer argument, returning `default` when it was not supplied.
///
/// The lookup modes and the text-splitting options all have a documented default, so reading them
/// through here keeps every function from repeating the ``if arguments.len() > index`` dance.
pub(crate) fn optional_integer(
    arguments: &[AstNode],
    index: usize,
    context: &mut dyn FormulaContext,
    default: i32,
) -> Result<i32, CellValue> {
    if index < arguments.len() {
        try_integer(arguments, index, context)
    } else {
        Ok(default)
    }
}

/// Evaluates an argument and returns its text, using the display form of a number.
pub(crate) fn try_text(
    arguments: &[AstNode],
    index: usize,
    context: &mut dyn FormulaContext,
) -> Result<Rc<str>, CellValue> {
    let Some(argument) = arguments.get(index) else {
        return Err(CellValue::error(CellError::Value));
    };

    let value = argument.evaluate(context);

    if value.is_error() {
        return Err(value);
    }

    Ok(value.as_text())
}

/// Gathers every numeric value the arguments contribute.
///
/// Text and blanks are skipped, which is how an aggregate treats a range; an error anywhere is
/// returned instead.
pub(crate) fn collect_numbers(
    arguments: &[AstNode],
    context: &mut dyn FormulaContext,
) -> Result<Vec<f64>, CellValue> {
    let mut numbers = Vec::new();
    let mut error = None;

    for argument in arguments {
        visit_values(argument, context, &mut |value| {
            if value.is_error() {
                error = Some(value);
                return false;
            }

            if let CellValue::Number(number) = value {
                numbers.push(number);
            }

            true
        });

        if let Some(failure) = error {
            return Err(failure);
        }
    }

    Ok(numbers)
}

/// Compares text the way `StringComparison.OrdinalIgnoreCase` does, without folding both sides into
/// new strings: a `COUNTIF` over a large range does this once per cell.
pub fn compare_ignore_case(left: &str, right: &str) -> Ordering {
    let mut left = left.chars().flat_map(char::to_lowercase);
    let mut right = right.chars().flat_map(char::to_lowercase);

    loop {
        match (left.next(), right.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(left), Some(right)) => match left.cmp(&right) {
                Ordering::Equal => {}
                other => return other,
            },
        }
    }
}

/// Compares two values the way a lookup does: numbers numerically, anything else as text, and text
/// comparisons ignoring case. An empty cell coerces to zero only when both sides are numbers.
pub(crate) fn compare_values(left: &CellValue, right: &CellValue) -> Ordering {
    let left_number = if left.is_empty() { None } else { left.as_number() };
    let right_number = if right.is_empty() { None } else { right.as_number() };

    match (left_number, right_number) {
        (Some(left), Some(right)) => left.partial_cmp(&right).unwrap_or(Ordering::Less),
        _ => compare_ignore_case(&left.to_display_string(), &right.to_display_string()),
    }
}

/// Rounds to a number of decimal places, half away from zero.
///
/// Scaling by a power of ten turns "round to n places" into "round to a whole number", and the
/// half-away-from-zero rule is applied to that whole number — which is what .NET's
/// `Math.Round(value, digits, MidpointRounding.AwayFromZero)` does.
pub(crate) fn round_away_from_zero(value: f64, digits: i32) -> f64 {
    let factor = 10f64.powi(digits.clamp(-15, 15));
    let scaled = value * factor;

    let rounded = if scaled >= 0.0 {
        (scaled + 0.5).floor()
    } else {
        (scaled - 0.5).ceil()
    };

    rounded / factor
}
