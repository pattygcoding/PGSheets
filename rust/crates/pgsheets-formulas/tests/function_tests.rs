//! The worksheet functions beyond the original set: the lookups, the text helpers, the conditional
//! aggregates and the two random functions, driven through the engine over a small store.
//!
//! The original library is pinned by the frozen golden corpus; these tests are the ones that cover
//! what was added on top, so a regression shows up as a named case rather than as a shifted count.

use std::collections::HashMap;

use pgsheets_core::store::CellStore;
use pgsheets_core::{CellAddress, CellValue};
use pgsheets_formulas::Engine;

/// A store as the engine sees it: raw input in, computed values out, sparsity respected.
#[derive(Default)]
struct Store {
    cells: HashMap<CellAddress, CellValue>,
    raw: HashMap<CellAddress, String>,
}

impl Store {
    fn set_raw(&mut self, row: i32, column: i32, raw: &str) {
        let address = CellAddress::new(row, column);
        self.raw.insert(address, raw.to_string());

        if raw.is_empty() {
            self.cells.remove(&address);
        } else {
            self.cells.entry(address).or_insert(CellValue::Empty);
        }
    }

    fn display(&self, row: i32, column: i32) -> String {
        self.value(row, column).to_display_string()
    }
}

impl CellStore for Store {
    fn raw_input(&self, row: i32, column: i32) -> &str {
        self.raw
            .get(&CellAddress::new(row, column))
            .map(String::as_str)
            .unwrap_or("")
    }

    fn value(&self, row: i32, column: i32) -> CellValue {
        self.cells
            .get(&CellAddress::new(row, column))
            .cloned()
            .unwrap_or(CellValue::Empty)
    }

    fn set_computed(&mut self, row: i32, column: i32, value: CellValue) {
        // A cell that does not exist stays that way, which is what keeps a deleted cell deleted.
        if let Some(cell) = self.cells.get_mut(&CellAddress::new(row, column)) {
            *cell = value;
        }
    }

    fn populated_cells(&self) -> Vec<CellAddress> {
        self.cells.keys().copied().collect()
    }
}

/// Evaluates a formula placed in `Z1`, which no seed uses, and returns its display text.
fn eval(seeds: &[(&str, &str)], formula: &str) -> String {
    let mut engine = Engine::new();
    let mut store = Store::default();

    for (a1, raw) in seeds {
        let (row, column) = CellAddress::try_parse_a1(a1).unwrap_or_else(|| panic!("bad {a1}"));
        store.set_raw(row, column, raw);
    }

    engine.rebuild(&mut store);

    let (row, column) = CellAddress::try_parse_a1("Z1").unwrap();
    store.set_raw(row, column, formula);
    engine.recalculate(&mut store, CellAddress::new(row, column));

    store.display(row, column)
}

/// A two-column table: numbers down column A, labels down column B.
fn table() -> Vec<(&'static str, &'static str)> {
    vec![
        ("A1", "10"),
        ("A2", "20"),
        ("A3", "30"),
        ("B1", "low"),
        ("B2", "mid"),
        ("B3", "high"),
    ]
}

#[test]
fn lookup_finds_an_exact_or_approximate_entry() {
    let seeds = table();

    assert_eq!(eval(&seeds, "=LOOKUP(20,A1:A3,B1:B3)"), "mid");
    // No exact match, so the largest key at or below the value answers.
    assert_eq!(eval(&seeds, "=LOOKUP(25,A1:A3,B1:B3)"), "mid");
    assert_eq!(eval(&seeds, "=LOOKUP(5,A1:A3,B1:B3)"), "#N/A");
    // Without a result vector the value is read from the lookup vector itself.
    assert_eq!(eval(&seeds, "=LOOKUP(30,A1:A3)"), "30");
}

#[test]
fn xlookup_returns_a_fallback_instead_of_a_miss() {
    let seeds = table();

    assert_eq!(eval(&seeds, "=XLOOKUP(30,A1:A3,B1:B3)"), "high");
    assert_eq!(eval(&seeds, "=XLOOKUP(99,A1:A3,B1:B3,\"none\")"), "none");
    assert_eq!(eval(&seeds, "=XLOOKUP(99,A1:A3,B1:B3)"), "#N/A");
}

#[test]
fn xmatch_honours_its_match_modes() {
    let seeds = table();

    assert_eq!(eval(&seeds, "=XMATCH(20,A1:A3)"), "2");
    assert_eq!(eval(&seeds, "=XMATCH(99,A1:A3)"), "#N/A");
    // Mode -1 is next smaller, mode 1 is next larger.
    assert_eq!(eval(&seeds, "=XMATCH(25,A1:A3,-1)"), "2");
    assert_eq!(eval(&seeds, "=XMATCH(25,A1:A3,1)"), "3");
    // Mode 2 reads the key as a wildcard pattern.
    assert_eq!(eval(&seeds, "=XMATCH(\"m*\",B1:B3,2)"), "2");
}

#[test]
fn rows_and_columns_report_a_range_shape() {
    let seeds = table();

    assert_eq!(eval(&seeds, "=ROWS(A1:B3)"), "3");
    assert_eq!(eval(&seeds, "=COLUMNS(A1:B3)"), "2");
    assert_eq!(eval(&seeds, "=ROWS(A1)"), "1");
    assert_eq!(eval(&seeds, "=COLUMNS(5)"), "#VALUE!");
}

#[test]
fn search_is_case_insensitive_and_supports_wildcards() {
    assert_eq!(eval(&[], "=SEARCH(\"b\",\"abc\")"), "2");
    assert_eq!(eval(&[], "=SEARCH(\"B\",\"abc\")"), "2");
    assert_eq!(eval(&[], "=SEARCH(\"b\",\"abc\",3)"), "#VALUE!");
    assert_eq!(eval(&[], "=SEARCH(\"a?c\",\"xxabc\")"), "3");
    assert_eq!(eval(&[], "=SEARCH(\"x\",\"abc\")"), "#VALUE!");
}

#[test]
fn replace_swaps_a_run_of_characters() {
    assert_eq!(eval(&[], "=REPLACE(\"abcdef\",2,3,\"XY\")"), "aXYef");
    assert_eq!(eval(&[], "=REPLACE(\"abc\",1,0,\"X\")"), "Xabc");
}

#[test]
fn text_around_a_delimiter_splits_on_the_nth_occurrence() {
    assert_eq!(eval(&[], "=TEXTBEFORE(\"a,b,c\",\",\")"), "a");
    assert_eq!(eval(&[], "=TEXTAFTER(\"a,b,c\",\",\")"), "b,c");
    assert_eq!(eval(&[], "=TEXTBEFORE(\"a,b,c\",\",\",2)"), "a,b");
    assert_eq!(eval(&[], "=TEXTAFTER(\"a,b,c\",\",\",-1)"), "c");
    assert_eq!(eval(&[], "=TEXTBEFORE(\"abc\",\"x\")"), "#N/A");
    assert_eq!(eval(&[], "=TEXTBEFORE(\"abc\",\"x\",1,0,0,\"none\")"), "none");
    // Matching the end of the text makes a missing delimiter fall back to the whole string.
    assert_eq!(eval(&[], "=TEXTBEFORE(\"abc\",\"x\",1,0,1)"), "abc");
}

#[test]
fn text_renders_a_number_with_a_format_code() {
    assert_eq!(eval(&[], "=TEXT(1234.5,\"0.00\")"), "1234.50");
    assert_eq!(eval(&[], "=TEXT(1234.5,\"#,##0.00\")"), "1,234.50");
    assert_eq!(eval(&[], "=TEXT(0.5,\"0%\")"), "50%");
}

/// A block with labels down column B, a count down column A and values down column C.
fn table_with_criteria() -> Vec<(&'static str, &'static str)> {
    vec![
        ("A1", "1"),
        ("A2", "2"),
        ("A3", "3"),
        ("A4", "4"),
        ("B1", "x"),
        ("B2", "y"),
        ("B3", "x"),
        ("B4", "y"),
        ("C1", "10"),
        ("C2", "20"),
        ("C3", "30"),
        ("C4", "40"),
    ]
}

#[test]
fn sumifs_adds_the_rows_a_criteria_selects() {
    let seeds = table_with_criteria();

    assert_eq!(eval(&seeds, "=SUMIFS(C1:C4,B1:B4,\"x\")"), "40");
    assert_eq!(eval(&seeds, "=SUMIFS(C1:C4,B1:B4,\"z\")"), "0");
    // Two criteria narrow the result to the single row that satisfies both.
    assert_eq!(eval(&seeds, "=SUMIFS(C1:C4,B1:B4,\"x\",A1:A4,\">1\")"), "30");
}

#[test]
fn countifs_counts_the_rows_every_criteria_matches() {
    let seeds = table_with_criteria();

    assert_eq!(eval(&seeds, "=COUNTIFS(B1:B4,\"x\")"), "2");
    assert_eq!(eval(&seeds, "=COUNTIFS(B1:B4,\"x\",A1:A4,\">1\")"), "1");
    assert_eq!(eval(&seeds, "=COUNTIFS(B1:B4,\"z\")"), "0");
}

#[test]
fn averageifs_maxifs_and_minifs_summarise_the_matching_rows() {
    let seeds = table_with_criteria();

    assert_eq!(eval(&seeds, "=AVERAGEIFS(C1:C4,B1:B4,\"y\")"), "30");
    assert_eq!(eval(&seeds, "=AVERAGEIFS(C1:C4,B1:B4,\"z\")"), "#DIV/0!");
    assert_eq!(eval(&seeds, "=MAXIFS(C1:C4,B1:B4,\"x\")"), "30");
    assert_eq!(eval(&seeds, "=MINIFS(C1:C4,B1:B4,\"x\")"), "10");
}

#[test]
fn mismatched_criteria_ranges_are_refused() {
    let seeds = table_with_criteria();

    assert_eq!(eval(&seeds, "=SUMIFS(C1:C4,B1:B2,\"x\")"), "#VALUE!");
}

#[test]
fn rand_stays_within_its_bounds() {
    for _ in 0..50 {
        let value: f64 = eval(&[], "=RAND()").parse().unwrap();
        assert!((0.0..1.0).contains(&value), "RAND gave {value}");
    }
}

#[test]
fn randbetween_returns_a_whole_number_in_range() {
    for _ in 0..50 {
        let value: f64 = eval(&[], "=RANDBETWEEN(1,3)").parse().unwrap();
        assert!((1.0..=3.0).contains(&value), "RANDBETWEEN gave {value}");
        assert_eq!(value.fract(), 0.0);
    }

    assert_eq!(eval(&[], "=RANDBETWEEN(5,1)"), "#NUM!");
}
