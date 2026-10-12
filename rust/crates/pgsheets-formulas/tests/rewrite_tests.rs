//! The writer and the rewriter: rendering a formula back to text, and adjusting its references for a
//! copy or for a row being inserted or removed.

use pgsheets_formulas::rewriter;
use pgsheets_formulas::{parse, write};

/// Parses a formula and renders it again, which is what every rewrite does.
fn round_trip(formula: &str) -> String {
    let node = parse(formula).unwrap_or_else(|error| panic!("could not parse {formula}: {error:?}"));
    write(&node, None)
}

#[test]
fn rendering_round_trips_what_it_was_given() {
    for formula in [
        "=1",
        "=1.5",
        "=0.1",
        "=\"text\"",
        "=\"a\"\"b\"",
        "=TRUE",
        "=A1",
        "=$A$1",
        "=A1+B2",
        "=SUM(A1:B5)",
        "=SUM(A1:A5,10,\"x\")",
        "=IF(A1>2,\"yes\",\"no\")",
        "=-(A1+B1)",
        "=2^-1",
        "=(A1+B1)*2",
        "=A1/B1-C1*D1",
        "=#N/A",
    ] {
        assert_eq!(round_trip(formula), formula, "rendering {formula}");
    }
}

#[test]
fn rendering_adds_the_parentheses_that_keep_the_meaning() {
    // The parse tree is `(1+2)*3`, which must not be written as `1+2*3`.
    assert_eq!(round_trip("=(1+2)*3"), "=(1+2)*3");

    // A subtraction whose right operand is a sum has to keep its parentheses too.
    assert_eq!(round_trip("=1-(2+3)"), "=1-(2+3)");

    // A power binds tighter than a sum, so the sum needs none.
    assert_eq!(round_trip("=1+2^3"), "=1+2^3");

    // A comparison inside a call is written bare, because arguments sit at the top level.
    assert_eq!(round_trip("=IF(A1>2,1,0)"), "=IF(A1>2,1,0)");
}

#[test]
fn a_number_is_written_so_that_it_reads_back_identically() {
    // A large number is written in exponent form, exactly as .NET's "R" format writes it, and reads
    // back as the same value.
    for formula in ["=1E+21", "=0.3", "=1E-07", "=1234567890123456789"] {
        let rendered = round_trip(formula);
        let node = parse(&rendered).unwrap_or_else(|error| panic!("{rendered}: {error:?}"));

        assert_eq!(
            node,
            parse(formula).unwrap(),
            "rendering {formula} gave {rendered}"
        );
    }

    assert_eq!(round_trip("=1000000000000000000000"), "=1E+21");
}

#[test]
fn a_copy_moves_relative_references_and_leaves_anchored_ones_alone() {
    assert_eq!(rewriter::translate("=A1", 1, 0), "=A2");
    assert_eq!(rewriter::translate("=A1", 0, 1), "=B1");
    assert_eq!(rewriter::translate("=$A$1", 1, 1), "=$A$1");
    assert_eq!(rewriter::translate("=$A1", 1, 1), "=$A2");
    assert_eq!(rewriter::translate("=A$1", 1, 1), "=B$1");
    assert_eq!(rewriter::translate("=SUM(A1:B2)", 2, 3), "=SUM(D3:E4)");

    // A copy that does not move changes nothing at all.
    assert_eq!(rewriter::translate("=A1+B1", 0, 0), "=A1+B1");
}

#[test]
fn a_reference_pushed_off_the_sheet_becomes_a_reference_error() {
    assert_eq!(rewriter::translate("=A1", -1, 0), "=#REF!");
    assert_eq!(rewriter::translate("=A1:B1", -1, 0), "=#REF!");
    assert_eq!(rewriter::translate("=A1+B1", 0, -1), "=#REF!+A1");
}

#[test]
fn inserting_a_row_moves_every_reference_below_it() {
    assert_eq!(rewriter::adjust_for_rows("=A1", 2, 1, true), "=A1");
    assert_eq!(rewriter::adjust_for_rows("=A3", 2, 1, true), "=A4");

    // Anchors do not protect a reference here, which matches a spreadsheet.
    assert_eq!(rewriter::adjust_for_rows("=$A$3", 2, 1, true), "=$A$4");
    assert_eq!(
        rewriter::adjust_for_rows("=SUM(A1:A3)", 2, 1, true),
        "=SUM(A1:A4)"
    );
}

#[test]
fn deleting_a_row_removes_the_references_that_pointed_at_it() {
    assert_eq!(rewriter::adjust_for_rows("=A2", 1, 1, false), "=#REF!");
    assert_eq!(rewriter::adjust_for_rows("=A1", 3, 1, false), "=A1");
    assert_eq!(rewriter::adjust_for_rows("=A4", 1, 1, false), "=A3");

    // A range with one deleted corner collapses to #REF! rather than to a partial range.
    assert_eq!(
        rewriter::adjust_for_rows("=SUM(A1:A2)", 1, 1, false),
        "=SUM(#REF!)"
    );
}

#[test]
fn a_column_edit_only_moves_columns() {
    assert_eq!(rewriter::adjust_for_columns("=B1", 1, 1, true), "=C1");
    assert_eq!(rewriter::adjust_for_columns("=A1", 5, 1, true), "=A1");
    assert_eq!(rewriter::adjust_for_columns("=B1", 1, 1, false), "=#REF!");
    assert_eq!(
        rewriter::adjust_for_columns("=SUM(A1:B1)", 0, 1, true),
        "=SUM(B1:C1)"
    );
}

#[test]
fn a_formula_that_cannot_be_read_is_left_exactly_as_it_was() {
    for formula in [
        "=1+",
        "=SUM(",
        "=@@@",
        "=\"unterminated",
        "=#WAT",
        "not a formula",
    ] {
        assert_eq!(rewriter::translate(formula, 1, 1), formula);
        assert_eq!(rewriter::adjust_for_rows(formula, 1, 1, true), formula);
    }
}
