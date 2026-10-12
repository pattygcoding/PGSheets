//! Conditional aggregates.
//!
//! Working out what a criterion means is [`criteria`](super::criteria)'s job; what is left here is
//! pairing the tested range with the range that gets summed or averaged.

use pgsheets_core::{CellError, CellValue};

use super::criteria::Criteria;
use crate::ast::{AstNode, FormulaContext};

/// `COUNTIF(range, criteria)` — the criteria may be a number, text or `">=10"`.
pub(super) fn count_if(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    if arguments.len() != 2 {
        return CellValue::error(CellError::Value);
    }

    let criteria = match try_criteria(&arguments[1], context) {
        Ok(criteria) => criteria,
        Err(error) => return error,
    };

    let mut count = 0.0;

    super::helpers::visit_values(&arguments[0], context, &mut |value| {
        if criteria.matches(&value) {
            count += 1.0;
        }

        true
    });

    CellValue::number(count)
}

pub(super) fn sum_if(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    conditional_aggregate(arguments, context, false)
}

pub(super) fn average_if(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    conditional_aggregate(arguments, context, true)
}

fn try_criteria(node: &AstNode, context: &mut dyn FormulaContext) -> Result<Criteria, CellValue> {
    let value = node.evaluate(context);

    if value.is_error() {
        return Err(value);
    }

    Ok(Criteria::parse(&value))
}

/// The top-left corner of a range or single-cell argument.
fn top_left(node: &AstNode) -> Option<(i32, i32)> {
    match node {
        AstNode::Range(range) => Some((range.row1(), range.column1())),
        AstNode::Cell(reference) => Some((reference.row, reference.column)),
        _ => None,
    }
}

/// `SUMIF` and `AVERAGEIF`.
///
/// The range being added up lines up with the tested range by offset, so a sum range is anchored at
/// its own top-left corner and then walked with the tested range's shape — which is exactly how a
/// spreadsheet treats a sum range of the wrong size. Without a third argument the tested range is the
/// one that gets added up.
fn conditional_aggregate(
    arguments: &[AstNode],
    context: &mut dyn FormulaContext,
    average: bool,
) -> CellValue {
    if arguments.len() < 2 || arguments.len() > 3 {
        return CellValue::error(CellError::Value);
    }

    let AstNode::Range(tested) = &arguments[0] else {
        return CellValue::error(CellError::Value);
    };

    let criteria = match try_criteria(&arguments[1], context) {
        Ok(criteria) => criteria,
        Err(error) => return error,
    };

    let origin = if arguments.len() == 3 {
        top_left(&arguments[2])
    } else {
        Some((tested.row1(), tested.column1()))
    };

    let mut total = 0.0;
    let mut matched = 0.0;

    for row in tested.row1()..=tested.row2() {
        for column in tested.column1()..=tested.column2() {
            let Some((origin_row, origin_column)) = origin else {
                continue;
            };

            if !criteria.matches(&context.cell_value(row, column)) {
                continue;
            }

            let value = context.cell_value(
                origin_row + (row - tested.row1()),
                origin_column + (column - tested.column1()),
            );

            if value.is_error() {
                return value;
            }

            if let CellValue::Number(number) = value {
                total += number;
                matched += 1.0;
            }
        }
    }

    if !average {
        return CellValue::number(total);
    }

    if matched == 0.0 {
        CellValue::error(CellError::DivideByZero)
    } else {
        CellValue::number(total / matched)
    }
}
