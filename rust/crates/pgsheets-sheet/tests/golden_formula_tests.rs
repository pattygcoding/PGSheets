//! The formula engine, checked case by case against reference output from the implementation this one
//! replaced.
//!
//! `golden/formulas.tsv` is that implementation run over a fixed sheet and a corpus of formulas covering
//! every function the library ships and the error branches that are easy to get wrong: the original
//! C#/.NET tree that the Rust port was written against, and which has since been deleted. Comparing
//! against it is what makes "the engine behaves the same" a fact rather than a hope — it is the same
//! trick that caught the halfway-rounding defect in the number-format interpreter.
//!
//! The fixture is **frozen**: the tree that produced it is gone, so a disagreement is a behaviour change
//! to be argued for rather than a file to re-record.

use pgsheets_core::CellAddress;
use pgsheets_formulas::Engine;
use pgsheets_sheet::Workbook;

/// The reference output, embedded at compile time so the test depends on no working directory.
const GOLDEN: &str = include_str!("golden/formulas.tsv");

/// The cell every formula is evaluated in.
///
/// The corpus uses it because no seed lands there, so a formula can never observe itself — and a
/// divergence shows up as one failing assertion rather than as a confusing cascade.
const PROBE: &str = "Z1";

fn at(a1: &str) -> CellAddress {
    let (row, column) = CellAddress::try_parse_a1(a1).unwrap_or_else(|| panic!("bad address {a1}"));
    CellAddress::new(row, column)
}

/// A workbook seeded the way the corpus expects, with the formula in the probe cell.
fn seeded(seeds: &[(String, String)], formula: &str) -> Workbook {
    let mut workbook = Workbook::new(Box::new(Engine::new()));

    workbook.begin_bulk_update();

    for (a1, raw) in seeds {
        let address = at(a1);
        workbook.set_raw_input(address.row, address.column, raw);
    }

    workbook.end_bulk_update();

    let probe = at(PROBE);
    workbook.set_raw_input(probe.row, probe.column, formula);

    workbook
}

#[test]
fn every_formula_case_matches_the_reference_implementation() {
    // The seeds are the same for every case, so they are collected once and the expected results are
    // evaluated against them one at a time.
    let mut seeds: Vec<(String, String)> = Vec::new();
    let mut cases: Vec<(String, String)> = Vec::new();

    for line in GOLDEN.lines() {
        // A byte-order mark would be part of the first field rather than of the line.
        let line = line.trim_start_matches('\u{feff}');
        let fields: Vec<&str> = line.split('\t').collect();

        match fields.as_slice() {
            ["seed", a1, raw] => seeds.push((a1.to_string(), raw.to_string())),
            ["fml", formula, _, expected] => cases.push((formula.to_string(), expected.to_string())),
            _ => panic!("a golden row is malformed: {line}"),
        }
    }

    assert!(seeds.len() >= 15, "the corpus should seed a sheet");
    assert!(cases.len() >= 240, "the corpus is suspiciously small");

    let mut failures = Vec::new();

    for (formula, expected) in &cases {
        let workbook = seeded(&seeds, formula);
        let probe = at(PROBE);
        let actual = workbook.display_value(probe.row, probe.column);

        if actual != expected.as_str() {
            failures.push(format!("{formula} gave {actual}, expected {expected}"));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} formulas disagree with the reference implementation:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}
