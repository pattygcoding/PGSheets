//! Statistical functions.
//!
//! They share one rule: text and blanks in a range are skipped rather than counted as zero, and an
//! error anywhere in the range propagates.

use pgsheets_core::{CellError, CellValue};

use super::helpers::{collect_numbers, try_integer, try_number, visit_values};
use crate::ast::{AstNode, FormulaContext};

pub(super) fn median(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    let numbers = match collect_numbers(arguments, context) {
        Ok(numbers) => numbers,
        Err(error) => return error,
    };

    if numbers.is_empty() {
        return CellValue::error(CellError::Number);
    }

    let mut numbers = numbers;
    sort_ascending(&mut numbers);

    let middle = numbers.len() / 2;

    if numbers.len() % 2 == 1 {
        CellValue::number(numbers[middle])
    } else {
        CellValue::number((numbers[middle - 1] + numbers[middle]) / 2.0)
    }
}

/// `MODE` is the most frequent value; the smallest wins a tie, as in a spreadsheet.
pub(super) fn mode(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    let numbers = match collect_numbers(arguments, context) {
        Ok(numbers) => numbers,
        Err(error) => return error,
    };

    if numbers.is_empty() {
        return CellValue::error(CellError::NotAvailable);
    }

    let mut numbers = numbers;
    sort_ascending(&mut numbers);

    let mut most_frequent = f64::NAN;
    let mut highest_count = 1;
    let mut index = 0;

    while index < numbers.len() {
        let mut run = 1;

        while index + run < numbers.len() && numbers[index + run] == numbers[index] {
            run += 1;
        }

        // Strictly greater keeps the first, lowest, value when two run lengths tie.
        if run > highest_count {
            highest_count = run;
            most_frequent = numbers[index];
        }

        index += run;
    }

    if highest_count > 1 {
        CellValue::number(most_frequent)
    } else {
        CellValue::error(CellError::NotAvailable)
    }
}

pub(super) fn large(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    nth_extreme(arguments, context, true)
}

pub(super) fn small(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    nth_extreme(arguments, context, false)
}

/// `LARGE` and `SMALL` take the k-th value counting in from one end of the sorted numbers.
fn nth_extreme(arguments: &[AstNode], context: &mut dyn FormulaContext, kth_largest: bool) -> CellValue {
    if arguments.len() != 2 {
        return CellValue::error(CellError::Value);
    }

    let numbers = match collect_numbers(&arguments[..1], context) {
        Ok(numbers) => numbers,
        Err(error) => return error,
    };

    let k = match try_integer(arguments, 1, context) {
        Ok(k) => k,
        Err(error) => return error,
    };

    if numbers.is_empty() || k < 1 || k as usize > numbers.len() {
        return CellValue::error(CellError::Number);
    }

    let mut numbers = numbers;
    sort_ascending(&mut numbers);

    let index = if kth_largest {
        numbers.len() - k as usize
    } else {
        k as usize - 1
    };

    CellValue::number(numbers[index])
}

/// Sorting a list that cannot contain a NaN: every value came from a number cell.
fn sort_ascending(numbers: &mut [f64]) {
    numbers.sort_by(|left, right| left.partial_cmp(right).unwrap_or(core::cmp::Ordering::Less));
}

/// `RANK(number, range, [order])` — order 0 or omitted ranks the largest value first.
pub(super) fn rank(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() < 2 || arguments.len() > 3 {
        return CellValue::error(CellError::Value);
    }

    let number = match try_number(arguments, 0, context) {
        Ok(number) => number,
        Err(error) => return error,
    };

    let numbers = match collect_numbers(&arguments[1..2], context) {
        Ok(numbers) => numbers,
        Err(error) => return error,
    };

    let ascending = if arguments.len() == 3 {
        match try_number(arguments, 2, context) {
            Ok(order) => order != 0.0,
            Err(error) => return error,
        }
    } else {
        false
    };

    if !numbers.contains(&number) {
        return CellValue::error(CellError::NotAvailable);
    }

    // Counting strictly better values gives tied entries the same rank.
    let ahead = numbers
        .iter()
        .filter(|candidate| {
            if ascending {
                **candidate < number
            } else {
                **candidate > number
            }
        })
        .count();

    CellValue::number((ahead + 1) as f64)
}

pub(super) fn st_dev(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    match sample_variance(arguments, context) {
        Ok(variance) => CellValue::number(variance.sqrt()),
        Err(error) => error,
    }
}

pub(super) fn variance(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    match sample_variance(arguments, context) {
        Ok(variance) => CellValue::number(variance),
        Err(error) => error,
    }
}

/// The sample variance, and with it `STDEV`.
///
/// Two passes rather than the sum-of-squares shortcut, which loses most of its precision on values
/// that are large and close together.
fn sample_variance(arguments: &[AstNode], context: &mut dyn FormulaContext) -> Result<f64, CellValue> {
    let numbers = collect_numbers(arguments, context)?;

    if numbers.len() < 2 {
        return Err(CellValue::error(CellError::DivideByZero));
    }

    let mean = numbers.iter().sum::<f64>() / numbers.len() as f64;
    let sum_of_squares: f64 = numbers.iter().map(|number| (number - mean).powi(2)).sum();

    Ok(sum_of_squares / (numbers.len() - 1) as f64)
}

/// The number of empty cells across the range.
pub(super) fn count_blank(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() != 1 {
        return CellValue::error(CellError::Value);
    }

    let mut count = 0.0;

    visit_values(&arguments[0], context, &mut |value| {
        if value.is_empty() {
            count += 1.0;
        }

        true
    });

    CellValue::number(count)
}

/// `SUMPRODUCT` multiplies its arguments cell by cell and adds the products.
///
/// Every argument must cover the same number of cells, and a non-numeric entry counts as zero.
pub(super) fn sum_product(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.is_empty() {
        return CellValue::error(CellError::Value);
    }

    let mut arrays: Vec<Vec<f64>> = Vec::new();

    for argument in arguments {
        let mut values = Vec::new();
        let mut error = None;

        visit_values(argument, context, &mut |value| {
            if value.is_error() {
                error = Some(value);
                return false;
            }

            values.push(match value {
                CellValue::Number(number) => number,
                _ => 0.0,
            });

            true
        });

        if let Some(failure) = error {
            return failure;
        }

        arrays.push(values);
    }

    let length = arrays[0].len();

    if arrays.iter().any(|array| array.len() != length) {
        // Mismatched shapes would otherwise pair up the wrong cells and quietly return a number that
        // means nothing.
        return CellValue::error(CellError::Value);
    }

    let mut total = 0.0;

    for index in 0..length {
        let mut product = 1.0;

        for array in &arrays {
            product *= array[index];
        }

        total += product;
    }

    CellValue::number(total)
}
