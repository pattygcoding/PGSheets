//! Logical functions.
//!
//! The conditional ones are lazy on purpose: `IFERROR`, `IFS` and `SWITCH` must not evaluate a branch
//! they are not going to return, or a guarded division would still raise.

use pgsheets_core::{CellError, CellValue};

use super::helpers::{compare_values, visit_values};
use crate::ast::{AstNode, FormulaContext};

/// How `AND`, `OR` and `XOR` fold their arguments.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LogicMode {
    And,
    Or,
    Xor,
}

/// Returns the fallback only when the first argument is an error.
pub(super) fn if_error(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() != 2 {
        return CellValue::error(CellError::Value);
    }

    let value = arguments[0].evaluate(context);

    if value.is_error() {
        arguments[1].evaluate(context)
    } else {
        value
    }
}

/// Like `IFERROR`, but only catches `#N/A` so a real mistake still shows through.
pub(super) fn if_na(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() != 2 {
        return CellValue::error(CellError::Value);
    }

    let value = arguments[0].evaluate(context);

    if value.error_code() == Some(CellError::NotAvailable) {
        arguments[1].evaluate(context)
    } else {
        value
    }
}

pub(super) fn and(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    logic(arguments, context, LogicMode::And)
}

pub(super) fn or(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    logic(arguments, context, LogicMode::Or)
}

pub(super) fn xor(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    logic(arguments, context, LogicMode::Xor)
}

/// `AND`, `OR` and `XOR` over any number of arguments.
///
/// Text and blanks inside a range are skipped rather than counted as false, which is what a
/// spreadsheet does, and an error propagates.
fn logic(arguments: &[AstNode], context: &mut dyn FormulaContext, mode: LogicMode) -> CellValue {
    let mut found = false;
    let mut accumulator = mode == LogicMode::And;
    let mut true_count = 0;
    let mut error = None;

    for argument in arguments {
        visit_values(argument, context, &mut |value| {
            if value.is_error() {
                error = Some(value);
                return false;
            }

            let truth = match value {
                CellValue::Boolean(flag) => flag,
                CellValue::Number(number) => number != 0.0,
                // Text and blanks in a range are ignored rather than treated as false.
                _ => return true,
            };

            found = true;

            if truth {
                true_count += 1;
            }

            accumulator = match mode {
                LogicMode::And => accumulator && truth,
                LogicMode::Or => accumulator || truth,
                LogicMode::Xor => accumulator,
            };

            true
        });

        if let Some(failure) = error {
            return failure;
        }
    }

    if !found {
        return CellValue::error(CellError::Value);
    }

    CellValue::boolean(match mode {
        LogicMode::Xor => true_count % 2 == 1,
        _ => accumulator,
    })
}

pub(super) fn not(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() != 1 {
        return CellValue::error(CellError::Value);
    }

    let value = arguments[0].evaluate(context);

    if value.is_error() {
        return value;
    }

    CellValue::boolean(!value.as_boolean())
}

/// Nested conditions, evaluated in order and lazily.
pub(super) fn ifs(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() < 2 || !arguments.len().is_multiple_of(2) {
        return CellValue::error(CellError::Value);
    }

    let mut index = 0;

    while index + 1 < arguments.len() {
        let condition = arguments[index].evaluate(context);

        if condition.is_error() {
            return condition;
        }

        if condition.as_boolean() {
            return arguments[index + 1].evaluate(context);
        }

        index += 2;
    }

    // Nothing matched, which a spreadsheet reports as #N/A rather than as FALSE.
    CellValue::error(CellError::NotAvailable)
}

/// `SWITCH(expression, value, result, ..., [default])`. Only the chosen result is evaluated.
pub(super) fn switch(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() < 3 {
        return CellValue::error(CellError::Value);
    }

    let subject = arguments[0].evaluate(context);

    if subject.is_error() {
        return subject;
    }

    // An odd number of trailing arguments means the last one is the default result.
    let has_default = (arguments.len() - 1) % 2 == 1;
    let limit = if has_default {
        arguments.len() - 1
    } else {
        arguments.len()
    };

    let mut index = 1;

    while index + 1 < limit {
        let candidate = arguments[index].evaluate(context);

        if candidate.is_error() {
            return candidate;
        }

        if compare_values(&subject, &candidate).is_eq() {
            return arguments[index + 1].evaluate(context);
        }

        index += 2;
    }

    if has_default {
        arguments[arguments.len() - 1].evaluate(context)
    } else {
        CellValue::error(CellError::NotAvailable)
    }
}

pub(super) fn true_value(arguments: &[AstNode], _context: &mut dyn FormulaContext) -> CellValue {
    if arguments.is_empty() {
        CellValue::boolean(true)
    } else {
        CellValue::error(CellError::Value)
    }
}

pub(super) fn false_value(arguments: &[AstNode], _context: &mut dyn FormulaContext) -> CellValue {
    if arguments.is_empty() {
        CellValue::boolean(false)
    } else {
        CellValue::error(CellError::Value)
    }
}

pub(super) fn na(arguments: &[AstNode], _context: &mut dyn FormulaContext) -> CellValue {
    if arguments.is_empty() {
        CellValue::error(CellError::NotAvailable)
    } else {
        CellValue::error(CellError::Value)
    }
}
