//! Fixtures and assertion helpers shared by the PGSheets test suites.
//!
//! Test-only by design: nothing in the application depends on this crate.

use std::collections::HashMap;

use pgsheets_core::store::{ExportSource, ImportSink};
use pgsheets_core::{CellAddress, CellRange, CellStyle, CellValue, parse_literal};

/// A workbook held in memory, as an exporter sees one.
///
/// The writers take an [`ExportSource`] rather than a model, so a test does not need one.
#[derive(Default)]
pub struct MemorySource {
    cells: HashMap<CellAddress, MemoryCell>,
}

/// A cell held in memory: its raw input, the value it evaluates to, its number format and its style.
#[derive(Clone, Debug, Default)]
pub struct MemoryCell {
    /// The raw input, exactly as it was written.
    pub raw: String,

    /// The value the cell holds.
    pub value: CellValue,

    /// The number format code, or `None` for General.
    pub format: Option<String>,

    /// The cell's style; [`CellStyle::DEFAULT`] when it has none.
    pub style: CellStyle,
}

impl MemorySource {
    /// An empty source.
    pub fn new() -> Self {
        Self::default()
    }

    /// Puts a raw value in a cell and evaluates it with the literal rules, which is all an exporter
    /// needs: the raw input is what gets written.
    pub fn set(&mut self, a1: &str, raw: &str) -> &mut Self {
        self.insert(a1, raw, parse_literal(raw), None, CellStyle::DEFAULT)
    }

    /// Puts a value in a cell with no raw input behind it, as an imported date with a format has.
    pub fn set_value(&mut self, a1: &str, value: CellValue, format: Option<&str>) -> &mut Self {
        self.insert(a1, "", value, format.map(str::to_string), CellStyle::DEFAULT)
    }

    /// Puts a raw value in a cell and gives it a number format.
    pub fn set_formatted(&mut self, a1: &str, raw: &str, format: &str) -> &mut Self {
        self.insert(
            a1,
            raw,
            parse_literal(raw),
            Some(format.to_string()),
            CellStyle::DEFAULT,
        )
    }

    /// Puts a raw value in a cell and gives it a style, keeping the value's literal rules.
    pub fn set_styled(&mut self, a1: &str, raw: &str, style: CellStyle) -> &mut Self {
        self.insert(a1, raw, parse_literal(raw), None, style)
    }

    /// Puts a formula in a cell together with the value it evaluated to, which is what a writer sees
    /// for a cell the engine has already calculated.
    pub fn set_formula(&mut self, a1: &str, formula: &str, value: CellValue) -> &mut Self {
        self.insert(a1, formula, value, None, CellStyle::DEFAULT)
    }

    fn insert(
        &mut self,
        a1: &str,
        raw: &str,
        value: CellValue,
        format: Option<String>,
        style: CellStyle,
    ) -> &mut Self {
        let (row, column) = CellAddress::try_parse_a1(a1).unwrap_or_else(|| panic!("bad address {a1}"));

        self.cells.insert(
            CellAddress::new(row, column),
            MemoryCell {
                raw: raw.to_string(),
                value,
                format,
                style,
            },
        );

        self
    }

    /// How many cells the source holds.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// True when the source holds nothing.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// A rectangle of the source, for an export restricted to a selection.
    pub fn range(a1: &str, b1: &str) -> CellRange {
        let (row1, column1) = CellAddress::try_parse_a1(a1).unwrap();
        let (row2, column2) = CellAddress::try_parse_a1(b1).unwrap();
        CellRange::new(row1, column1, row2, column2)
    }
}

impl ExportSource for MemorySource {
    fn populated_cells(&self) -> Vec<CellAddress> {
        self.cells.keys().copied().collect()
    }

    fn raw_input(&self, row: i32, column: i32) -> &str {
        self.cells
            .get(&CellAddress::new(row, column))
            .map(|cell| cell.raw.as_str())
            .unwrap_or("")
    }

    fn value(&self, row: i32, column: i32) -> CellValue {
        self.cells
            .get(&CellAddress::new(row, column))
            .map(|cell| cell.value.clone())
            .unwrap_or(CellValue::Empty)
    }

    fn number_format(&self, row: i32, column: i32) -> Option<&str> {
        self.cells
            .get(&CellAddress::new(row, column))
            .and_then(|cell| cell.format.as_deref())
    }

    fn cell_style(&self, row: i32, column: i32) -> Option<CellStyle> {
        self.cells
            .get(&CellAddress::new(row, column))
            .map(|cell| cell.style)
            .filter(|style| !style.is_default())
    }
}

/// An import sink that records everything it is given, for asserting on a reader's output.
#[derive(Default)]
pub struct RecordingSink {
    /// The raw input of every cell, by address.
    pub cells: HashMap<CellAddress, String>,

    /// The number format of every cell, by address.
    pub formats: HashMap<CellAddress, Option<String>>,

    /// The style of every cell, by address.
    pub styles: HashMap<CellAddress, CellStyle>,
}

impl RecordingSink {
    /// An empty sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// The raw input recorded for a cell; empty when the reader said nothing about it.
    pub fn get(&self, a1: &str) -> &str {
        let (row, column) = CellAddress::try_parse_a1(a1).unwrap_or_else(|| panic!("bad address {a1}"));

        self.cells
            .get(&CellAddress::new(row, column))
            .map(String::as_str)
            .unwrap_or("")
    }

    /// The number format recorded for a cell, or `None`.
    pub fn get_format(&self, a1: &str) -> Option<String> {
        let (row, column) = CellAddress::try_parse_a1(a1).unwrap_or_else(|| panic!("bad address {a1}"));

        self.formats
            .get(&CellAddress::new(row, column))
            .cloned()
            .flatten()
    }

    /// The style recorded for a cell, or `None` when the reader said nothing about it.
    pub fn get_style(&self, a1: &str) -> Option<CellStyle> {
        let (row, column) = CellAddress::try_parse_a1(a1).unwrap_or_else(|| panic!("bad address {a1}"));

        self.styles.get(&CellAddress::new(row, column)).copied()
    }

    /// How many cells the reader wrote to, empty ones included.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// True when the reader wrote nothing.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

impl ImportSink for RecordingSink {
    fn set_raw_input(&mut self, row: i32, column: i32, raw_input: &str) {
        self.cells
            .insert(CellAddress::new(row, column), raw_input.to_string());
    }

    fn set_number_format(&mut self, row: i32, column: i32, number_format: Option<&str>) {
        self.formats
            .insert(CellAddress::new(row, column), number_format.map(str::to_string));
    }

    fn set_cell_style(&mut self, row: i32, column: i32, style: Option<&CellStyle>) {
        self.styles
            .insert(CellAddress::new(row, column), style.copied().unwrap_or_default());
    }
}

/// Asserts two floats are equal to within a tolerance that is meaningful for spreadsheet arithmetic.
///
/// Spelled out rather than pulled from a crate: comparison of computed numbers is the one thing the
/// whole suite does, and a six-line function is cheaper than a dependency.
#[macro_export]
macro_rules! assert_close {
    ($left:expr, $right:expr) => {{
        let left: f64 = $left;
        let right: f64 = $right;
        let scale = left.abs().max(right.abs()).max(1.0);
        assert!(
            (left - right).abs() <= 1e-9 * scale,
            "expected {left} to be within 1e-9 of {right}"
        );
    }};
}

/// Unwraps a value as a number, failing the test with a readable message when it is something else.
pub fn number_of(value: &CellValue) -> f64 {
    match value {
        CellValue::Number(number) => *number,
        other => panic!("expected a number but found {other:?}"),
    }
}

/// The display text of a value, which is what most assertions actually care about.
pub fn display_of(value: &CellValue) -> String {
    value.to_display_string()
}
