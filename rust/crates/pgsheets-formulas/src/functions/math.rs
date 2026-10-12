//! Numeric functions, and the lazy `IF` that lives beside them.

use pgsheets_core::{CellError, CellValue};

use super::helpers::{round_away_from_zero, try_integer, try_number, visit_values};
use crate::ast::{AstNode, FormulaContext};

/// Lazy conditional: only the selected branch is evaluated, or a guarded division would still raise.
pub(super) fn if_function(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() < 2 {
        return CellValue::error(CellError::Value);
    }

    let condition = arguments[0].evaluate(context);

    if condition.is_error() {
        return condition;
    }

    if condition.as_boolean() {
        return arguments[1].evaluate(context);
    }

    match arguments.get(2) {
        Some(branch) => branch.evaluate(context),
        None => CellValue::boolean(false),
    }
}

/// Applies a one-argument numeric function, which is most of this file.
fn unary_math(
    arguments: &[AstNode],
    context: &mut dyn FormulaContext,
    operation: fn(f64) -> f64,
) -> CellValue {
    if arguments.len() != 1 {
        return CellValue::error(CellError::Value);
    }

    let value = arguments[0].evaluate(context);

    if value.is_error() {
        return value;
    }

    let Some(number) = value.as_number() else {
        return CellValue::error(CellError::Value);
    };

    // A NaN result is a domain error, which a spreadsheet reports as #NUM!.
    match operation(number) {
        result if result.is_nan() => CellValue::error(CellError::Number),
        result => CellValue::number(result),
    }
}

pub(super) fn abs(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    unary_math(arguments, context, f64::abs)
}

pub(super) fn sign(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    unary_math(arguments, context, f64::signum)
}

pub(super) fn sqrt(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    unary_math(arguments, context, |value| {
        if value < 0.0 { f64::NAN } else { value.sqrt() }
    })
}

pub(super) fn exp(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    unary_math(arguments, context, f64::exp)
}

pub(super) fn ln(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    unary_math(arguments, context, |value| {
        if value <= 0.0 { f64::NAN } else { value.ln() }
    })
}

pub(super) fn log10(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    unary_math(arguments, context, |value| {
        if value <= 0.0 { f64::NAN } else { value.log10() }
    })
}

/// Rounds down to the nearest whole number, towards negative infinity rather than towards zero.
pub(super) fn int(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    unary_math(arguments, context, f64::floor)
}

pub(super) fn pi(arguments: &[AstNode], _context: &mut dyn FormulaContext) -> CellValue {
    if arguments.is_empty() {
        CellValue::number(core::f64::consts::PI)
    } else {
        CellValue::error(CellError::Value)
    }
}

pub(super) fn power(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() != 2 {
        return CellValue::error(CellError::Value);
    }

    let base = match try_number(arguments, 0, context) {
        Ok(number) => number,
        Err(error) => return error,
    };

    let exponent = match try_number(arguments, 1, context) {
        Ok(number) => number,
        Err(error) => return error,
    };

    CellValue::number(base.powf(exponent))
}

/// `LOG(number, [base])` — base 10 unless one is given.
pub(super) fn log(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.is_empty() || arguments.len() > 2 {
        return CellValue::error(CellError::Value);
    }

    let number = match try_number(arguments, 0, context) {
        Ok(number) => number,
        Err(error) => return error,
    };

    let log_base = if arguments.len() == 2 {
        match try_number(arguments, 1, context) {
            Ok(number) => number,
            Err(error) => return error,
        }
    } else {
        10.0
    };

    if number <= 0.0 || log_base <= 0.0 || log_base == 1.0 {
        return CellValue::error(CellError::Number);
    }

    CellValue::number(number.log(log_base))
}

/// Rounds to a number of decimal places, half away from zero.
pub(super) fn round(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.is_empty() || arguments.len() > 2 {
        return CellValue::error(CellError::Value);
    }

    let number = match try_number(arguments, 0, context) {
        Ok(number) => number,
        Err(error) => return error,
    };

    let digits = if arguments.len() == 2 {
        match try_integer(arguments, 1, context) {
            Ok(digits) => digits,
            Err(error) => return error,
        }
    } else {
        0
    };

    CellValue::number(round_away_from_zero(number, digits.clamp(0, 15)))
}

/// Rounds away from zero to a number of digits.
pub(super) fn round_up(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    round_directional(arguments, context, |value| {
        if value >= 0.0 { value.ceil() } else { value.floor() }
    })
}

/// Rounds towards zero to a number of digits.
pub(super) fn round_down(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    round_directional(arguments, context, f64::trunc)
}

pub(super) fn trunc(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    round_directional(arguments, context, f64::trunc)
}

/// `ROUNDUP`, `ROUNDDOWN` and `TRUNC` differ only in which way they move a value that sits between two
/// candidates, so the three share this.
fn round_directional(
    arguments: &[AstNode],
    context: &mut dyn FormulaContext,
    adjust: fn(f64) -> f64,
) -> CellValue {
    if arguments.is_empty() || arguments.len() > 2 {
        return CellValue::error(CellError::Value);
    }

    let number = match try_number(arguments, 0, context) {
        Ok(number) => number,
        Err(error) => return error,
    };

    let digits = if arguments.len() == 2 {
        match try_integer(arguments, 1, context) {
            Ok(digits) => digits,
            Err(error) => return error,
        }
    } else {
        0
    };

    // Scaling by a power of ten turns "round to n digits" into "round to a whole number".
    let factor = 10f64.powi(digits.clamp(-15, 15));

    CellValue::number(adjust(number * factor) / factor)
}

pub(super) fn ceiling(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    multiple_of(arguments, context, true)
}

pub(super) fn floor(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    multiple_of(arguments, context, false)
}

/// `CEILING` and `FLOOR` round to a multiple of a significance, 1 by default.
fn multiple_of(arguments: &[AstNode], context: &mut dyn FormulaContext, round_up: bool) -> CellValue {
    if arguments.is_empty() || arguments.len() > 2 {
        return CellValue::error(CellError::Value);
    }

    let number = match try_number(arguments, 0, context) {
        Ok(number) => number,
        Err(error) => return error,
    };

    let significance = if arguments.len() == 2 {
        match try_number(arguments, 1, context) {
            Ok(number) => number,
            Err(error) => return error,
        }
    } else {
        1.0
    };

    if significance == 0.0 {
        return CellValue::number(0.0);
    }

    // Working in multiples of the significance reduces both cases to a plain ceiling or floor. A
    // negative significance reverses the direction, which is what a spreadsheet does.
    let upward = if significance > 0.0 { round_up } else { !round_up };
    let multiple = number / significance;

    CellValue::number((if upward { multiple.ceil() } else { multiple.floor() }) * significance)
}

/// `MOD` is the spreadsheet remainder, which follows the divisor's sign rather than the language's
/// `%`: `MOD(-3, 5)` is 2, where `-3 % 5` is -3.
pub(super) fn modulo(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() != 2 {
        return CellValue::error(CellError::Value);
    }

    let number = match try_number(arguments, 0, context) {
        Ok(number) => number,
        Err(error) => return error,
    };

    let divisor = match try_number(arguments, 1, context) {
        Ok(number) => number,
        Err(error) => return error,
    };

    if divisor == 0.0 {
        return CellValue::error(CellError::DivideByZero);
    }

    CellValue::number(number - (divisor * (number / divisor).floor()))
}

/// Joins the display form of every value it is given.
pub(super) fn concat(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    let mut text = String::new();
    let mut error = None;

    for argument in arguments {
        visit_values(argument, context, &mut |value| {
            if value.is_error() {
                error = Some(value);
                return false;
            }

            text.push_str(&value.to_display_string());
            true
        });

        if let Some(failure) = error {
            return failure;
        }
    }

    CellValue::text(&text)
}
