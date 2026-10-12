//! A typed cell value: what a cell holds after the formula engine has evaluated it.

use core::fmt;
use std::rc::Rc;

use crate::numeric::{general_number, parse_number};

/// A spreadsheet error code, surfaced to the grid as a `#...` display string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CellError {
    /// A divisor of zero.
    DivideByZero,

    /// An operand of the wrong type, or a badly formed formula.
    Value,

    /// A reference to a cell, row or column that no longer exists.
    Reference,

    /// A name this engine does not know.
    Name,

    /// A result outside the range a spreadsheet can represent.
    Number,

    /// Circular references: reached by re-entering a cell that is already being evaluated.
    Circular,

    /// No match: what a failed lookup returns, and what `ISNA` and `IFNA` test for.
    NotAvailable,
}

impl CellError {
    /// The display text a spreadsheet shows for this error.
    pub fn text(self) -> &'static str {
        match self {
            CellError::DivideByZero => "#DIV/0!",
            CellError::Value => "#VALUE!",
            CellError::Reference => "#REF!",
            CellError::Name => "#NAME?",
            CellError::Number => "#NUM!",
            CellError::Circular => "#CIRCULAR!",
            CellError::NotAvailable => "#N/A",
        }
    }
}

/// An evaluated cell value.
///
/// Cheap to copy for the numeric and boolean cases, which is what keeps the recalculation path free
/// of allocation for the common case. Text is reference counted so that reading a value during
/// evaluation — which happens once per reference, per recalculation — does not duplicate it.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum CellValue {
    /// No value at all: an empty cell, or a function that produced nothing.
    #[default]
    Empty,

    /// A number.
    Number(f64),

    /// Text.
    Text(Rc<str>),

    /// A boolean, which a spreadsheet writes as `TRUE` or `FALSE`.
    Boolean(bool),

    /// An error.
    Error(CellError),
}

impl CellValue {
    /// An empty value, which is also the default.
    pub const EMPTY: Self = Self::Empty;

    /// Wraps a number.
    pub const fn number(value: f64) -> Self {
        Self::Number(value)
    }

    /// Wraps a boolean.
    pub const fn boolean(value: bool) -> Self {
        Self::Boolean(value)
    }

    /// Wraps an error code.
    pub const fn error(error: CellError) -> Self {
        Self::Error(error)
    }

    /// Wraps text. Empty text is an empty cell, which is what a blank field in a file means.
    pub fn text(value: &str) -> Self {
        if value.is_empty() {
            Self::Empty
        } else {
            Self::Text(Rc::from(value))
        }
    }

    /// Wraps an already shared string.
    pub fn shared_text(value: Rc<str>) -> Self {
        if value.is_empty() {
            Self::Empty
        } else {
            Self::Text(value)
        }
    }

    /// True when the value is an error.
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// True when the cell holds nothing.
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// The error code, when the value is an error.
    pub const fn error_code(&self) -> Option<CellError> {
        match self {
            Self::Error(error) => Some(*error),
            _ => None,
        }
    }

    /// Numeric coercion used by arithmetic and aggregation.
    ///
    /// Mirrors spreadsheet semantics: empty equals 0, booleans equal 1/0, and numeric-looking text is
    /// coerced. `None` means the value cannot be a number at all.
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            Self::Boolean(value) => Some(if *value { 1.0 } else { 0.0 }),
            Self::Empty => Some(0.0),
            Self::Text(text) => parse_number(text),
            Self::Error(_) => None,
        }
    }

    /// Interprets the value as a condition, which is what `IF` and the logic functions need.
    pub fn as_boolean(&self) -> bool {
        match self {
            Self::Boolean(value) => *value,
            Self::Number(value) => *value != 0.0,
            Self::Text(text) => text.eq_ignore_ascii_case("TRUE"),
            _ => false,
        }
    }

    /// Numeric coercion with a 0 fallback, for hosts that extend the function set.
    pub fn as_number_or_zero(&self) -> f64 {
        self.as_number().unwrap_or(0.0)
    }

    /// The string a function sees when it treats a value as text: a number is rendered through the
    /// general format, so `LEN(1234)` is 4.
    pub fn as_text(&self) -> Rc<str> {
        match self {
            Self::Text(text) => Rc::clone(text),
            _ => Rc::from(self.to_display_string().as_str()),
        }
    }

    /// The string the grid renders for this value, with no number format applied.
    pub fn to_display_string(&self) -> String {
        match self {
            Self::Number(value) => general_number(*value),
            Self::Text(text) => text.to_string(),
            Self::Boolean(value) => {
                if *value {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            }
            Self::Error(error) => error.text().to_string(),
            Self::Empty => String::new(),
        }
    }
}

impl fmt::Display for CellValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_display_string())
    }
}
