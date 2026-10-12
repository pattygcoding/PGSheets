//! The number-format interpreter, checked against reference output from the implementation this one
//! replaced.
//!
//! `golden/number_format.tsv` is that implementation run over a fixed matrix of values: the original
//! C#/.NET tree that the Rust port was written against, and which has since been deleted. Comparing
//! against it is what makes "the format language behaves the same" a fact rather than a hope; the
//! halfway rounding cases in particular (`2.5` under `0`, `1.005` under `0.00`) are where a naive port
//! would diverge.
//!
//! The fixture is **frozen**: the tree that produced it is gone, so a disagreement is a behaviour change
//! to be argued for rather than a file to re-record.

use pgsheets_core::number_format;

/// The reference output, embedded at compile time so the test depends on no working directory.
const GOLDEN: &str = include_str!("golden/number_format.tsv");

#[test]
fn every_number_format_case_matches_the_reference_implementation() {
    let mut checked = 0;

    for line in GOLDEN.lines().skip(1) {
        let mut fields = line.split('\t');

        let (Some(kind), Some(value), Some(format), Some(expected)) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            panic!("a golden row is malformed: {line}");
        };

        let value: f64 = value
            .parse()
            .unwrap_or_else(|_| panic!("unparseable value in: {line}"));

        let format = if format == "<null>" { None } else { Some(format) };

        let actual = match kind {
            "fmt" => number_format::apply(value, format),
            "gen" => number_format::apply_general(value),
            other => panic!("unknown golden kind '{other}'"),
        };

        assert_eq!(actual, expected, "{kind} of {value} with {format:?}");
        checked += 1;
    }

    assert!(checked >= 88, "only {checked} golden rows were checked");
}
