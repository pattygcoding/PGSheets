//! A single sparse cell record: exactly what the user typed, plus what the engine computed for it.

use crate::number_format;
use crate::style::CellStyle;
use crate::value::CellValue;

/// One cell: its raw input, the number format and style that go with it, and the evaluated result.
#[derive(Clone, Debug)]
pub struct CellData {
    raw_input: String,
    number_format: Option<String>,
    style: CellStyle,
    value: CellValue,
    display_value: String,
}

impl CellData {
    /// Creates a cell holding raw input, with no computed result yet and no formatting.
    pub fn new(raw_input: impl Into<String>) -> Self {
        Self {
            raw_input: raw_input.into(),
            number_format: None,
            style: CellStyle::DEFAULT,
            value: CellValue::Empty,
            display_value: String::new(),
        }
    }

    /// The stored input: `"1200"`, `"North"` or `"=SUM(A1:A5)"`.
    pub fn raw_input(&self) -> &str {
        &self.raw_input
    }

    /// The number format code for the cell, e.g. `"0.00"` or `"yyyy-mm-dd"`. `None` means General.
    pub fn number_format(&self) -> Option<&str> {
        self.number_format.as_deref()
    }

    /// The cell's styling, which is [`CellStyle::DEFAULT`] when it has none.
    pub fn style(&self) -> CellStyle {
        self.style
    }

    /// The typed result produced by the engine.
    pub fn value(&self) -> &CellValue {
        &self.value
    }

    /// The text the grid shows, kept in step with [`CellData::value`].
    pub fn display_value(&self) -> &str {
        &self.display_value
    }

    /// True when the raw input is a formula.
    pub fn is_formula(&self) -> bool {
        self.raw_input.starts_with('=')
    }

    /// True when the cell exists only to carry formatting, with no value of its own.
    ///
    /// A cell given a number format *or* a style and then left empty still exists, because formatting
    /// is remembered for whatever is typed next — which is what makes "format the column, then fill it
    /// in" work.
    pub fn is_format_only(&self) -> bool {
        self.raw_input.is_empty() && self.has_format()
    }

    /// True when the cell carries a number format or a non-default style.
    pub fn has_format(&self) -> bool {
        self.number_format.is_some() || !self.style.is_default()
    }

    /// Replaces the stored input.
    pub fn set_raw_input(&mut self, raw_input: impl Into<String>) {
        self.raw_input = raw_input.into();
    }

    /// Replaces the number format code. An empty code means General.
    pub fn set_number_format(&mut self, format: Option<String>) {
        self.number_format = format.filter(|code| !code.is_empty());
    }

    /// Replaces the cell's styling.
    pub fn set_style(&mut self, style: CellStyle) {
        self.style = style;
    }

    /// Stores an evaluated result, and the text the grid will show for it.
    ///
    /// This is where a number format earns its keep: it is applied here, while recalculating, and
    /// never while the grid paints.
    pub fn set_computed(&mut self, value: CellValue) {
        self.display_value = match &value {
            CellValue::Number(number) => number_format::apply(*number, self.number_format.as_deref()),
            other => other.to_display_string(),
        };

        self.value = value;
    }
}

impl core::fmt::Display for CellData {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{} => {}", self.raw_input, self.display_value)
    }
}
