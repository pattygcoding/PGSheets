//! A recursive-descent parser for a pragmatic spreadsheet formula subset:
//! `=SUM(A1:A5)`, `=A1 + B2 * 3`, `=-A1 ^ 2`, string literals, comparisons.
//!
//! Parsing happens only on recalculation, never while the grid paints.

mod names;
mod primary;

use pgsheets_core::CellError;

use crate::ast::{AstNode, BinaryOperator, ComparisonOperator, UnaryOperator};

/// Why a formula could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormulaError {
    /// The error code the formula surfaces as when it cannot be parsed.
    pub code: CellError,

    /// What went wrong, for a reader of an import's warnings.
    pub message: String,
}

/// Parses a formula, including its leading `=`.
///
/// `Err` carries what was wrong with it. The caller decides what to do: the grid shows `#VALUE!`, and
/// a structural edit leaves the text exactly as it was rather than guessing at it.
pub fn parse(formula: &str) -> Result<AstNode, FormulaError> {
    Parser::new(formula).parse_all()
}

/// Parses a formula, treating anything unreadable as an error node, which is what evaluation wants.
///
/// An error node evaluates to the code the failure would have produced, so a malformed formula behaves
/// like one rather than like an empty cell.
pub fn parse_or_error_node(formula: &str) -> AstNode {
    match parse(formula) {
        Ok(node) => node,
        Err(error) => AstNode::Error(error.code),
    }
}

/// The recursive-descent parser.
struct Parser {
    text: Vec<char>,
    index: usize,
}

impl Parser {
    fn new(text: &str) -> Self {
        Self {
            text: text.chars().collect(),
            index: 0,
        }
    }

    fn parse_all(mut self) -> Result<AstNode, FormulaError> {
        self.skip_whitespace();

        if self.peek() == Some('=') {
            self.index += 1;
        }

        let node = self.parse_comparison()?;
        self.skip_whitespace();

        if self.index < self.text.len() {
            let found = self.text[self.index];
            return Err(self.error(
                CellError::Value,
                format!("Unexpected '{found}' at {}.", self.index),
            ));
        }

        Ok(node)
    }

    /// Lowest precedence level: `=`, `<>`, `<`, `<=`, `>`, `>=`.
    fn parse_comparison(&mut self) -> Result<AstNode, FormulaError> {
        let mut left = self.parse_additive()?;

        loop {
            self.skip_whitespace();

            let operator = match self.peek() {
                Some('=') => {
                    self.index += 1;
                    ComparisonOperator::Equal
                }
                Some('<') => {
                    self.index += 1;

                    match self.peek() {
                        Some('>') => {
                            self.index += 1;
                            ComparisonOperator::NotEqual
                        }
                        Some('=') => {
                            self.index += 1;
                            ComparisonOperator::LessThanOrEqual
                        }
                        _ => ComparisonOperator::LessThan,
                    }
                }
                Some('>') => {
                    self.index += 1;

                    if self.peek() == Some('=') {
                        self.index += 1;
                        ComparisonOperator::GreaterThanOrEqual
                    } else {
                        ComparisonOperator::GreaterThan
                    }
                }
                _ => return Ok(left),
            };

            let right = self.parse_additive()?;

            left = AstNode::Comparison {
                operator,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
    }

    fn parse_additive(&mut self) -> Result<AstNode, FormulaError> {
        let mut left = self.parse_multiplicative()?;

        loop {
            self.skip_whitespace();

            let operator = match self.peek() {
                Some('+') => BinaryOperator::Add,
                Some('-') => BinaryOperator::Subtract,
                _ => return Ok(left),
            };

            self.index += 1;
            let right = self.parse_multiplicative()?;

            left = AstNode::Binary {
                operator,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
    }

    fn parse_multiplicative(&mut self) -> Result<AstNode, FormulaError> {
        let mut left = self.parse_unary()?;

        loop {
            self.skip_whitespace();

            let operator = match self.peek() {
                Some('*') => BinaryOperator::Multiply,
                Some('/') => BinaryOperator::Divide,
                _ => return Ok(left),
            };

            self.index += 1;
            let right = self.parse_unary()?;

            left = AstNode::Binary {
                operator,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
    }

    fn parse_unary(&mut self) -> Result<AstNode, FormulaError> {
        self.skip_whitespace();

        match self.peek() {
            Some('-') => {
                self.index += 1;

                Ok(AstNode::Unary {
                    operator: UnaryOperator::Negate,
                    operand: Box::new(self.parse_unary()?),
                })
            }
            // A leading '+' is a no-op rather than a node, so `+A1` writes back as `A1`.
            Some('+') => {
                self.index += 1;
                self.parse_unary()
            }
            _ => self.parse_power(),
        }
    }

    fn parse_power(&mut self) -> Result<AstNode, FormulaError> {
        let left = self.parse_primary()?;
        self.skip_whitespace();

        if self.peek() != Some('^') {
            return Ok(left);
        }

        self.index += 1;

        // Right associative, and the right operand may itself be signed: 2^-1.
        let right = self.parse_unary()?;

        Ok(AstNode::Binary {
            operator: BinaryOperator::Power,
            left: Box::new(left),
            right: Box::new(right),
        })
    }

    fn peek(&self) -> Option<char> {
        self.text.get(self.index).copied()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(ch) if ch.is_whitespace()) {
            self.index += 1;
        }
    }

    fn error(&self, code: CellError, message: String) -> FormulaError {
        FormulaError { code, message }
    }
}
