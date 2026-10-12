//! Renders a parsed formula back to text.

use pgsheets_core::numeric::write_round_trip;
use pgsheets_core::{CellError, CellReference};

use crate::ast::{AstNode, BinaryOperator, CellRangeRef, ComparisonOperator, UnaryOperator};

/// Rewrites a reference as it is written out. Returning `None` means the reference no longer exists,
/// which is written as `#REF!`.
pub type ReferenceRewrite<'a> = dyn Fn(CellReference) -> Option<CellReference> + 'a;

// Precedence, lowest first.
const COMPARISON_PRECEDENCE: u8 = 1;
const ADDITIVE_PRECEDENCE: u8 = 2;
const MULTIPLICATIVE_PRECEDENCE: u8 = 3;
const UNARY_PRECEDENCE: u8 = 4;
const POWER_PRECEDENCE: u8 = 5;
const PRIMARY_PRECEDENCE: u8 = 6;

/// Renders a parsed formula as text, including the leading `=`.
///
/// The output round-trips through the parser, so a rewritten formula means exactly what it says.
pub fn write(node: &AstNode, rewrite: Option<&ReferenceRewrite<'_>>) -> String {
    let mut text = String::new();
    text.push('=');
    append(node, &mut text, 0, rewrite);
    text
}

/// Writes a node, wrapping it in parentheses when it binds less tightly than its position requires.
///
/// Without this a rewrite could silently change what a formula means: `(A1+B1)*2` is not `A1+B1*2`.
fn append(node: &AstNode, text: &mut String, minimum_precedence: u8, rewrite: Option<&ReferenceRewrite<'_>>) {
    let parenthesise = precedence_of(node) < minimum_precedence;

    if parenthesise {
        text.push('(');
    }

    match node {
        AstNode::Number(value) => text.push_str(&write_round_trip(*value)),
        AstNode::Text(value) => append_text(text, value),
        AstNode::Boolean(value) => text.push_str(if *value { "TRUE" } else { "FALSE" }),
        AstNode::Error(error) => text.push_str(error.text()),

        AstNode::Cell(reference) => append_reference(text, apply(*reference, rewrite)),

        AstNode::Range(range) => append_range(text, range, rewrite),

        AstNode::Unary { operator, operand } => {
            text.push(match operator {
                UnaryOperator::Negate => '-',
            });
            append(operand, text, UNARY_PRECEDENCE, rewrite);
        }

        AstNode::Binary {
            operator,
            left,
            right,
        } => append_binary(text, *operator, left, right, rewrite),

        AstNode::Comparison {
            operator,
            left,
            right,
        } => {
            append(left, text, COMPARISON_PRECEDENCE, rewrite);
            text.push_str(comparison_symbol(*operator));
            // The right operand binds tighter, so `A1>B1=C1` keeps the shape it was parsed with.
            append(right, text, COMPARISON_PRECEDENCE + 1, rewrite);
        }

        AstNode::Function { name, arguments } => {
            text.push_str(name);
            text.push('(');

            for (index, argument) in arguments.iter().enumerate() {
                if index > 0 {
                    text.push(',');
                }

                // Arguments sit at the top level, so a comparison is written bare: IF(A1>2,..).
                append(argument, text, 0, rewrite);
            }

            text.push(')');
        }
    }

    if parenthesise {
        text.push(')');
    }
}

fn append_binary(
    text: &mut String,
    operator: BinaryOperator,
    left: &AstNode,
    right: &AstNode,
    rewrite: Option<&ReferenceRewrite<'_>>,
) {
    let precedence = binary_precedence(operator);

    // Every operator except '^' is left associative, so the right operand must bind tighter or it
    // needs parentheses. '^' takes a bare signed exponent, as in 2^-1.
    let left_minimum = if operator == BinaryOperator::Power {
        POWER_PRECEDENCE + 1
    } else {
        precedence
    };

    let right_minimum = if operator == BinaryOperator::Power {
        UNARY_PRECEDENCE
    } else {
        precedence + 1
    };

    append(left, text, left_minimum, rewrite);
    text.push_str(binary_symbol(operator));
    append(right, text, right_minimum, rewrite);
}

fn append_range(text: &mut String, range: &CellRangeRef, rewrite: Option<&ReferenceRewrite<'_>>) {
    let start = apply(range.start, rewrite);
    let end = apply(range.end, rewrite);

    // A range with a missing corner is written as #REF! rather than as a broken endpoint.
    let (Some(mut first), Some(mut second)) = (start, end) else {
        text.push_str(CellError::Reference.text());
        return;
    };

    // Rewriting can invert the corners, and a range is always written top-left first.
    if (first.row, first.column) > (second.row, second.column) {
        core::mem::swap(&mut first, &mut second);
    }

    text.push_str(&first.to_a1());
    text.push(':');
    text.push_str(&second.to_a1());
}

fn append_reference(text: &mut String, reference: Option<CellReference>) {
    match reference {
        Some(reference) => text.push_str(&reference.to_a1()),
        None => text.push_str(CellError::Reference.text()),
    }
}

/// A quote inside a string literal is doubled, so the literal survives a round trip.
fn append_text(text: &mut String, value: &str) {
    text.push('"');

    for ch in value.chars() {
        if ch == '"' {
            text.push('"');
        }

        text.push(ch);
    }

    text.push('"');
}

fn apply(reference: CellReference, rewrite: Option<&ReferenceRewrite<'_>>) -> Option<CellReference> {
    match rewrite {
        Some(rewrite) => rewrite(reference),
        None => Some(reference),
    }
}

fn binary_precedence(operator: BinaryOperator) -> u8 {
    match operator {
        BinaryOperator::Add | BinaryOperator::Subtract => ADDITIVE_PRECEDENCE,
        BinaryOperator::Power => POWER_PRECEDENCE,
        BinaryOperator::Multiply | BinaryOperator::Divide => MULTIPLICATIVE_PRECEDENCE,
    }
}

fn precedence_of(node: &AstNode) -> u8 {
    match node {
        AstNode::Comparison { .. } => COMPARISON_PRECEDENCE,
        AstNode::Binary { operator, .. } => binary_precedence(*operator),
        AstNode::Unary { .. } => UNARY_PRECEDENCE,
        _ => PRIMARY_PRECEDENCE,
    }
}

/// The symbol a binary operator is written with.
pub fn binary_symbol(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Add => "+",
        BinaryOperator::Subtract => "-",
        BinaryOperator::Multiply => "*",
        BinaryOperator::Divide => "/",
        BinaryOperator::Power => "^",
    }
}

/// The symbol a comparison is written with.
pub fn comparison_symbol(operator: ComparisonOperator) -> &'static str {
    match operator {
        ComparisonOperator::Equal => "=",
        ComparisonOperator::NotEqual => "<>",
        ComparisonOperator::LessThan => "<",
        ComparisonOperator::LessThanOrEqual => "<=",
        ComparisonOperator::GreaterThan => ">",
        ComparisonOperator::GreaterThanOrEqual => ">=",
    }
}
