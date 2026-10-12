//! Conditional aggregates.
//!
//! Working out what a criterion means is [`criteria`](super::criteria)'s job; what is left here is
//! pairing the tested range with the range that gets summed or averaged.

use pgsheets_core::{CellError, CellValue};

use super::criteria::Criteria;
use crate::ast::{AstNode, CellRangeRef, FormulaContext};

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

/// Which aggregate a multi-criteria function computes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MultiAggregate {
    Count,
    Sum,
    Average,
    Max,
    Min,
}

/// `SUMIFS(sum_range, criteria_range, criteria, ...)`.
pub(super) fn sum_ifs(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    multi_aggregate(arguments, context, MultiAggregate::Sum)
}

/// `COUNTIFS(criteria_range, criteria, ...)`.
pub(super) fn count_ifs(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    multi_aggregate(arguments, context, MultiAggregate::Count)
}

/// `AVERAGEIFS(average_range, criteria_range, criteria, ...)`.
pub(super) fn average_ifs(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    multi_aggregate(arguments, context, MultiAggregate::Average)
}

/// `MAXIFS(max_range, criteria_range, criteria, ...)`.
pub(super) fn max_ifs(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    multi_aggregate(arguments, context, MultiAggregate::Max)
}

/// `MINIFS(min_range, criteria_range, criteria, ...)`.
pub(super) fn min_ifs(arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    multi_aggregate(arguments, context, MultiAggregate::Min)
}

/// The body shared by the `...IFS` family.
///
/// Every range has to cover the same cells — a mismatched pair would silently line up the wrong ones —
/// and `COUNTIFS` differs only in having no result range of its own, so it starts at the criteria pair
/// rather than after a leading range.
fn multi_aggregate(
    arguments: &[AstNode],
    context: &mut dyn FormulaContext,
    aggregate: MultiAggregate,
) -> CellValue {
    let counting = aggregate == MultiAggregate::Count;
    let pair_start = if counting { 0 } else { 1 };

    if arguments.len() < pair_start + 2 || !(arguments.len() - pair_start).is_multiple_of(2) {
        return CellValue::error(CellError::Value);
    }

    let AstNode::Range(result) = &arguments[0] else {
        return CellValue::error(CellError::Value);
    };

    let result = *result;

    let mut criteria: Vec<(CellRangeRef, Criteria)> = Vec::new();
    let mut index = pair_start;

    while index + 1 < arguments.len() {
        let AstNode::Range(range) = &arguments[index] else {
            return CellValue::error(CellError::Value);
        };

        let value = arguments[index + 1].evaluate(context);

        if value.is_error() {
            return value;
        }

        criteria.push((*range, Criteria::parse(&value)));
        index += 2;
    }

    let rows = result.row_count();
    let columns = result.column_count();

    if criteria
        .iter()
        .any(|(range, _)| range.row_count() != rows || range.column_count() != columns)
    {
        return CellValue::error(CellError::Value);
    }

    let mut count = 0.0;
    let mut total = 0.0;
    let mut extremum: Option<f64> = None;

    for row_offset in 0..rows {
        for column_offset in 0..columns {
            let matched = criteria.iter().all(|(range, test)| {
                let value = context.cell_value(range.row1() + row_offset, range.column1() + column_offset);

                test.matches(&value)
            });

            if !matched {
                continue;
            }

            if counting {
                count += 1.0;
                continue;
            }

            let value = context.cell_value(result.row1() + row_offset, result.column1() + column_offset);

            if value.is_error() {
                return value;
            }

            if let CellValue::Number(number) = value {
                total += number;
                count += 1.0;

                extremum = Some(match extremum {
                    None => number,
                    Some(current) => match aggregate {
                        MultiAggregate::Min => current.min(number),
                        MultiAggregate::Max => current.max(number),
                        _ => current,
                    },
                });
            }
        }
    }

    match aggregate {
        MultiAggregate::Count => CellValue::number(count),
        MultiAggregate::Sum => CellValue::number(total),
        MultiAggregate::Average => {
            if count == 0.0 {
                CellValue::error(CellError::DivideByZero)
            } else {
                CellValue::number(total / count)
            }
        }
        MultiAggregate::Max | MultiAggregate::Min => CellValue::number(extremum.unwrap_or(0.0)),
    }
}
