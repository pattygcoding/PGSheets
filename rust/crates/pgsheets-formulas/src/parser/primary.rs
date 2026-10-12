//! The primary level: literals, references, ranges, calls and parentheses.

use pgsheets_core::CellError;

use super::{FormulaError, Parser};
use crate::ast::AstNode;

impl Parser {
    pub(super) fn parse_primary(&mut self) -> Result<AstNode, FormulaError> {
        self.skip_whitespace();

        let Some(ch) = self.peek() else {
            return Err(self.error(CellError::Value, "Unexpected end of formula.".to_string()));
        };

        match ch {
            '(' => {
                self.index += 1;
                let inner = self.parse_comparison()?;
                self.skip_whitespace();

                if self.peek() != Some(')') {
                    return Err(self.error(CellError::Value, "Missing ')'.".to_string()));
                }

                self.index += 1;
                Ok(inner)
            }

            '"' => self.parse_string(),
            '#' => self.parse_error_literal(),
            '$' => self.parse_cell_ref_or_function(),

            _ if ch.is_alphabetic() => self.parse_cell_ref_or_function(),
            _ if ch.is_ascii_digit() || ch == '.' => self.parse_number(),

            _ => Err(self.error(CellError::Value, format!("Unexpected character '{ch}'."))),
        }
    }

    fn parse_number(&mut self) -> Result<AstNode, FormulaError> {
        let start = self.index;

        while let Some(ch) = self.peek() {
            let exponent_sign = (ch == '+' || ch == '-')
                && self.index > start
                && matches!(self.text[self.index - 1], 'e' | 'E');

            if ch.is_ascii_digit() || ch == '.' || ch == 'e' || ch == 'E' || exponent_sign {
                self.index += 1;
                continue;
            }

            break;
        }

        let token: String = self.text[start..self.index].iter().collect();

        // Rust's own float parser accepts exactly the forms this scanner produces, and rejects the
        // same nonsense the original implementation's `double.TryParse` does.
        token
            .parse::<f64>()
            .map(AstNode::Number)
            .map_err(|_| self.error(CellError::Value, format!("Invalid number '{token}'.")))
    }

    fn parse_string(&mut self) -> Result<AstNode, FormulaError> {
        self.index += 1; // opening quote

        let mut value = String::new();

        while let Some(ch) = self.peek() {
            if ch == '"' {
                // "" is an escaped quote inside a string literal.
                if self.text.get(self.index + 1) == Some(&'"') {
                    value.push('"');
                    self.index += 2;
                    continue;
                }

                self.index += 1;
                return Ok(AstNode::Text(value.into()));
            }

            value.push(ch);
            self.index += 1;
        }

        Err(self.error(CellError::Value, "Unterminated string literal.".to_string()))
    }

    pub(super) fn parse_function_call(&mut self, name: &str) -> Result<AstNode, FormulaError> {
        self.index += 1; // '('
        let mut arguments = Vec::new();

        self.skip_whitespace();

        if self.peek() == Some(')') {
            self.index += 1;
            return Ok(AstNode::Function {
                name: name.to_string(),
                arguments,
            });
        }

        loop {
            arguments.push(self.parse_comparison()?);
            self.skip_whitespace();

            match self.peek() {
                Some(',') => {
                    self.index += 1;
                    self.skip_whitespace();
                }
                Some(')') => {
                    self.index += 1;
                    break;
                }
                _ => {
                    return Err(self.error(CellError::Value, format!("Expected ',' or ')' in {name}().")));
                }
            }
        }

        Ok(AstNode::Function {
            name: name.to_string(),
            arguments,
        })
    }
}
