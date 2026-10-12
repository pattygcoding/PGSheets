//! Type-testing functions.
//!
//! These deliberately do *not* propagate an error argument: reporting on a value is the whole point,
//! so `ISERROR(#N/A)` has to answer `TRUE` rather than return `#N/A`.

use pgsheets_core::{CellError, CellValue};

use super::helpers::try_number;
use crate::ast::{AstNode, FormulaContext};

pub(super) fn is_blank(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    predicate(arguments, context, CellValue::is_empty)
}

pub(super) fn is_number(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    predicate(arguments, context, |value| matches!(value, CellValue::Number(_)))
}

pub(super) fn is_text(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    predicate(arguments, context, |value| matches!(value, CellValue::Text(_)))
}

pub(super) fn is_logical(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    predicate(arguments, context, |value| matches!(value, CellValue::Boolean(_)))
}

pub(super) fn is_error(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    predicate(arguments, context, CellValue::is_error)
}

pub(super) fn is_na(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    predicate(arguments, context, |value| {
        value.error_code() == Some(CellError::NotAvailable)
    })
}

pub(super) fn is_even(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    parity(arguments, context, true)
}

pub(super) fn is_odd(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    parity(arguments, context, false)
}

/// Applies a one-argument test, which is what every function in this file does.
fn predicate(
    arguments: &[AstNode],
    context: &mut dyn FormulaContext,
    test: fn(&CellValue) -> bool,
) -> CellValue {
    if arguments.len() != 1 {
        return CellValue::error(CellError::Value);
    }

    CellValue::boolean(test(&arguments[0].evaluate(context)))
}

fn parity(arguments: &[AstNode], context: &mut dyn FormulaContext, even: bool) -> CellValue {
    if arguments.len() != 1 {
        return CellValue::error(CellError::Value);
    }

    let number = match try_number(arguments, 0, context) {
        Ok(number) => number,
        Err(error) => return error,
    };

    CellValue::boolean(is_even_integer(number) == even)
}

/// True for a whole number that is a multiple of two.
///
/// .NET's `double.IsEvenInteger` answers false for anything with a fraction, which is what this
/// reproduces rather than truncating first.
fn is_even_integer(number: f64) -> bool {
    number.is_finite() && number.fract() == 0.0 && (number / 2.0).fract() == 0.0
}
