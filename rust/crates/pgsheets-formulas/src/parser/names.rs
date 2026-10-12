//! Names: error literals, cell references, ranges, and function calls.

use pgsheets_core::{CellAddress, CellError, MAX_COLUMNS, MAX_ROWS};

use super::{FormulaError, Parser};
use crate::ast::{AstNode, CellRangeRef};

impl Parser {
    pub(super) fn parse_error_literal(&mut self) -> Result<AstNode, FormulaError> {
        let start = self.index;
        self.index += 1; // '#'

        while matches!(self.peek(), Some(ch) if is_error_literal_char(ch)) {
            self.index += 1;
        }

        let token: String = self.text[start..self.index]
            .iter()
            .collect::<String>()
            .to_uppercase();

        let error = match token.as_str() {
            "#DIV/0!" => CellError::DivideByZero,
            "#VALUE!" => CellError::Value,
            "#REF!" => CellError::Reference,
            "#NAME?" => CellError::Name,
            "#NUM!" => CellError::Number,
            "#N/A" | "#N/A!" => CellError::NotAvailable,
            _ => {
                return Err(self.error(CellError::Name, format!("Unknown error literal '{token}'.")));
            }
        };

        Ok(AstNode::Error(error))
    }

    pub(super) fn parse_cell_ref_or_function(&mut self) -> Result<AstNode, FormulaError> {
        let start = self.index;

        while matches!(self.peek(), Some(ch) if is_identifier_char(ch)) {
            self.index += 1;
        }

        let token: String = self.text[start..self.index].iter().collect();

        self.skip_whitespace();

        if self.peek() == Some('(') {
            return self.parse_function_call(&token);
        }

        if token.eq_ignore_ascii_case("TRUE") {
            return Ok(AstNode::Boolean(true));
        }

        if token.eq_ignore_ascii_case("FALSE") {
            return Ok(AstNode::Boolean(false));
        }

        let Some(parsed) = CellAddress::try_parse_a1_anchored(&token) else {
            return Err(self.error(CellError::Name, format!("Unknown name '{token}'.")));
        };

        // A reference past the edge of the sheet is a reference error rather than a parse failure: the
        // formula is readable, it just cannot point anywhere.
        if parsed.row >= MAX_ROWS || parsed.column >= MAX_COLUMNS {
            return Ok(AstNode::Error(CellError::Reference));
        }

        self.skip_whitespace();

        if self.peek() != Some(':') {
            return Ok(AstNode::Cell(pgsheets_core::CellReference {
                row: parsed.row,
                column: parsed.column,
                absolute_row: parsed.absolute_row,
                absolute_column: parsed.absolute_column,
            }));
        }

        self.index += 1; // ':'
        self.skip_whitespace();

        let range_start = self.index;

        while matches!(self.peek(), Some(ch) if is_identifier_char(ch)) {
            self.index += 1;
        }

        let range_token: String = self.text[range_start..self.index].iter().collect();

        let Some(end) = CellAddress::try_parse_a1_anchored(&range_token) else {
            return Err(self.error(
                CellError::Reference,
                format!("Invalid range end '{range_token}'."),
            ));
        };

        if end.row >= MAX_ROWS || end.column >= MAX_COLUMNS {
            return Err(self.error(
                CellError::Reference,
                "Range exceeds the sheet bounds.".to_string(),
            ));
        }

        Ok(AstNode::Range(CellRangeRef::new(
            parsed.row,
            parsed.column,
            end.row,
            end.column,
            parsed.absolute_row,
            parsed.absolute_column,
            end.absolute_row,
            end.absolute_column,
        )))
    }
}

/// The characters a reference or a function name is made of.
fn is_identifier_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '$' || ch == '_' || ch == '.'
}

/// The characters an error literal is made of, so `#DIV/0!` reads as one token.
fn is_error_literal_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '/' || ch == '!' || ch == '?'
}
