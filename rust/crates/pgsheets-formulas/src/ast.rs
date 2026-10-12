//! The parsed shape of a formula, and what it evaluates to.
//!
//! A formula is stored as text and is the source of truth; this is what the text parses into, and it
//! exists so a formula can be evaluated, its references collected for dependency tracking, and its
//! text re-rendered after a structural change.

use std::collections::HashSet;
use std::rc::Rc;

use pgsheets_core::{CellAddress, CellError, CellReference, CellValue};

use crate::functions::FunctionLibrary;

/// Read access to other cells while a formula is being evaluated.
pub trait FormulaContext {
    /// The value of another cell, evaluating it first when it has not been evaluated yet.
    fn cell_value(&mut self, row: i32, column: i32) -> CellValue;

    /// The library a function call is resolved against.
    ///
    /// It is reached through the context rather than through a global so that a caller can extend the
    /// language — or replace it — without touching this crate's state, and so that two workbooks can
    /// never see each other's functions.
    fn functions(&self) -> &FunctionLibrary;
}

/// A binary arithmetic operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOperator {
    /// `+`
    Add,

    /// `-`
    Subtract,

    /// `*`
    Multiply,

    /// `/`
    Divide,

    /// `^`
    Power,
}

/// A unary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOperator {
    /// A leading `-`.
    Negate,
}

/// A comparison operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonOperator {
    /// `=`
    Equal,

    /// `<>`
    NotEqual,

    /// `<`
    LessThan,

    /// `<=`
    LessThanOrEqual,

    /// `>`
    GreaterThan,

    /// `>=`
    GreaterThanOrEqual,
}

/// A rectangular range as it was written, keeping each corner's anchors.
///
/// A range is only meaningful as a function argument, which is why the corners are kept apart rather
/// than collapsed into a start and a size: a copy has to shift one corner and leave the other alone
/// when only one of them is anchored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellRangeRef {
    /// Top-left corner, after normalisation.
    pub start: CellReference,

    /// Bottom-right corner, after normalisation.
    pub end: CellReference,
}

impl CellRangeRef {
    /// Normalises the corners, carrying each corner's anchors with it.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        row1: i32,
        column1: i32,
        row2: i32,
        column2: i32,
        absolute_row1: bool,
        absolute_column1: bool,
        absolute_row2: bool,
        absolute_column2: bool,
    ) -> Self {
        let rows_ordered = row1 <= row2;
        let columns_ordered = column1 <= column2;

        let (top, top_row_anchor, bottom, bottom_row_anchor) = if rows_ordered {
            (row1, absolute_row1, row2, absolute_row2)
        } else {
            (row2, absolute_row2, row1, absolute_row1)
        };

        let (left, left_column_anchor, right, right_column_anchor) = if columns_ordered {
            (column1, absolute_column1, column2, absolute_column2)
        } else {
            (column2, absolute_column2, column1, absolute_column1)
        };

        Self {
            start: CellReference {
                row: top,
                column: left,
                absolute_row: top_row_anchor,
                absolute_column: left_column_anchor,
            },
            end: CellReference {
                row: bottom,
                column: right,
                absolute_row: bottom_row_anchor,
                absolute_column: right_column_anchor,
            },
        }
    }

    /// Top row of the range.
    pub fn row1(&self) -> i32 {
        self.start.row
    }

    /// Bottom row of the range.
    pub fn row2(&self) -> i32 {
        self.end.row
    }

    /// Left column of the range.
    pub fn column1(&self) -> i32 {
        self.start.column
    }

    /// Right column of the range.
    pub fn column2(&self) -> i32 {
        self.end.column
    }

    /// Number of rows covered.
    pub fn row_count(&self) -> i32 {
        self.row2() - self.row1() + 1
    }

    /// Number of columns covered.
    pub fn column_count(&self) -> i32 {
        self.column2() - self.column1() + 1
    }

    /// Every cell in the range, row by row.
    pub fn cells(&self) -> impl Iterator<Item = CellAddress> {
        (self.row1()..=self.row2()).flat_map(move |row| {
            (self.column1()..=self.column2()).map(move |column| CellAddress::new(row, column))
        })
    }
}

/// A parsed formula expression.
#[derive(Clone, Debug, PartialEq)]
pub enum AstNode {
    /// A number literal.
    Number(f64),

    /// A quoted string.
    Text(Rc<str>),

    /// `TRUE` or `FALSE`.
    Boolean(bool),

    /// An error literal such as `#N/A`, or a reference that cannot exist.
    Error(CellError),

    /// A single cell reference.
    Cell(CellReference),

    /// A rectangular range.
    Range(CellRangeRef),

    /// A signed operand.
    Unary {
        /// The operator.
        operator: UnaryOperator,
        /// What it applies to.
        operand: Box<AstNode>,
    },

    /// An arithmetic operation.
    Binary {
        /// The operator.
        operator: BinaryOperator,
        /// The left operand.
        left: Box<AstNode>,
        /// The right operand.
        right: Box<AstNode>,
    },

    /// A comparison.
    Comparison {
        /// The operator.
        operator: ComparisonOperator,
        /// The left operand.
        left: Box<AstNode>,
        /// The right operand.
        right: Box<AstNode>,
    },

    /// A call such as `SUM(A1:A5)`. The name is kept as written; the library resolves it case
    /// insensitively.
    Function {
        /// The function name.
        name: String,
        /// The arguments, unevaluated.
        arguments: Vec<AstNode>,
    },
}

impl AstNode {
    /// Evaluates the node.
    pub fn evaluate(&self, context: &mut dyn FormulaContext) -> CellValue {
        match self {
            Self::Number(value) => CellValue::number(*value),
            Self::Text(text) => CellValue::shared_text(Rc::clone(text)),
            Self::Boolean(value) => CellValue::boolean(*value),
            Self::Error(error) => CellValue::error(*error),
            Self::Cell(reference) => context.cell_value(reference.row, reference.column),
            // A range has no single value: a function enumerates it instead.
            Self::Range(_) => CellValue::error(CellError::Value),
            Self::Unary { operator, operand } => evaluate_unary(*operator, operand, context),
            Self::Binary {
                operator,
                left,
                right,
            } => evaluate_binary(*operator, left, right, context),
            Self::Comparison {
                operator,
                left,
                right,
            } => evaluate_comparison(*operator, left, right, context),
            Self::Function { name, arguments } => evaluate_function(name, arguments, context),
        }
    }

    /// Collects the cells this node reads, for dependency tracking.
    pub fn collect_precedents(&self, into: &mut HashSet<CellAddress>) {
        match self {
            Self::Cell(reference) => {
                into.insert(CellAddress::new(reference.row, reference.column));
            }
            Self::Range(range) => {
                for address in range.cells() {
                    into.insert(address);
                }
            }
            Self::Unary { operand, .. } => operand.collect_precedents(into),
            Self::Binary { left, right, .. } | Self::Comparison { left, right, .. } => {
                left.collect_precedents(into);
                right.collect_precedents(into);
            }
            Self::Function { arguments, .. } => {
                for argument in arguments {
                    argument.collect_precedents(into);
                }
            }
            Self::Number(_) | Self::Text(_) | Self::Boolean(_) | Self::Error(_) => {}
        }
    }

    /// The cell references this node writes, for a caller that wants to rewrite them.
    pub fn references(&self) -> Vec<(CellReference, CellReference)> {
        let mut found = Vec::new();
        self.collect_references(&mut found);
        found
    }

    fn collect_references(&self, into: &mut Vec<(CellReference, CellReference)>) {
        match self {
            Self::Cell(reference) => into.push((*reference, *reference)),
            Self::Range(range) => into.push((range.start, range.end)),
            Self::Unary { operand, .. } => operand.collect_references(into),
            Self::Binary { left, right, .. } | Self::Comparison { left, right, .. } => {
                left.collect_references(into);
                right.collect_references(into);
            }
            Self::Function { arguments, .. } => {
                for argument in arguments {
                    argument.collect_references(into);
                }
            }
            Self::Number(_) | Self::Text(_) | Self::Boolean(_) | Self::Error(_) => {}
        }
    }
}

fn evaluate_unary(operator: UnaryOperator, operand: &AstNode, context: &mut dyn FormulaContext) -> CellValue {
    let value = operand.evaluate(context);

    if value.is_error() {
        return value;
    }

    let Some(number) = value.as_number() else {
        return CellValue::error(CellError::Value);
    };

    match operator {
        UnaryOperator::Negate => CellValue::number(-number),
    }
}

fn evaluate_binary(
    operator: BinaryOperator,
    left: &AstNode,
    right: &AstNode,
    context: &mut dyn FormulaContext,
) -> CellValue {
    let left = left.evaluate(context);

    if left.is_error() {
        return left;
    }

    let right = right.evaluate(context);

    if right.is_error() {
        return right;
    }

    let (Some(left_number), Some(right_number)) = (left.as_number(), right.as_number()) else {
        return CellValue::error(CellError::Value);
    };

    match operator {
        BinaryOperator::Add => CellValue::number(left_number + right_number),
        BinaryOperator::Subtract => CellValue::number(left_number - right_number),
        BinaryOperator::Multiply => CellValue::number(left_number * right_number),
        BinaryOperator::Divide => {
            if right_number == 0.0 {
                CellValue::error(CellError::DivideByZero)
            } else {
                CellValue::number(left_number / right_number)
            }
        }
        BinaryOperator::Power => CellValue::number(left_number.powf(right_number)),
    }
}

/// Numbers compare numerically and anything else compares as text, case-insensitively, which is what
/// an `IF` condition needs.
fn evaluate_comparison(
    operator: ComparisonOperator,
    left: &AstNode,
    right: &AstNode,
    context: &mut dyn FormulaContext,
) -> CellValue {
    let left = left.evaluate(context);

    if left.is_error() {
        return left;
    }

    let right = right.evaluate(context);

    if right.is_error() {
        return right;
    }

    let ordering = match (left.as_number(), right.as_number()) {
        (Some(left_number), Some(right_number)) => compare_numbers(left_number, right_number),
        _ => crate::functions::compare_ignore_case(&left.to_display_string(), &right.to_display_string()),
    };

    let result = match operator {
        ComparisonOperator::Equal => ordering.is_eq(),
        ComparisonOperator::NotEqual => ordering.is_ne(),
        ComparisonOperator::LessThan => ordering.is_lt(),
        ComparisonOperator::LessThanOrEqual => ordering.is_le(),
        ComparisonOperator::GreaterThan => ordering.is_gt(),
        ComparisonOperator::GreaterThanOrEqual => ordering.is_ge(),
    };

    CellValue::boolean(result)
}

/// A total order over two numbers, so that a comparison never has to answer "unordered".
///
/// The only value without an ordering of its own is NaN, which is not something a cell holds; it is
/// treated as less than everything so that `=A1=B1` on two NaNs still answers.
fn compare_numbers(left: f64, right: f64) -> core::cmp::Ordering {
    left.partial_cmp(&right).unwrap_or(core::cmp::Ordering::Less)
}

fn evaluate_function(name: &str, arguments: &[AstNode], context: &mut dyn FormulaContext) -> CellValue {
    let Some(function) = context.functions().get(name) else {
        // A name the engine does not know, which is what an unimplemented function surfaces as.
        return CellValue::error(CellError::Name);
    };

    // Arguments stay unevaluated, so a function decides which of them to evaluate — essential for IF,
    // and for ranges, which are enumerated rather than evaluated.
    function(arguments, context)
}
