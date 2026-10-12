//! References with their `$` anchors, and the parser that reads them back.

use pgsheets_core::{CellAddress, CellReference};

#[test]
fn formats_with_anchors() {
    for (row, column, absolute_row, absolute_column, expected) in [
        (0, 0, false, false, "A1"),
        (6, 1, false, false, "B7"),
        (6, 1, true, true, "$B$7"),
        (6, 1, true, false, "B$7"),
        (6, 1, false, true, "$B7"),
        (0, 26, false, false, "AA1"),
    ] {
        let reference = CellReference {
            row,
            column,
            absolute_row,
            absolute_column,
        };

        assert_eq!(reference.to_a1(), expected);
    }
}

#[test]
fn relative_is_unanchored() {
    let reference = CellReference::relative(2, 3);

    assert!(!reference.absolute_row);
    assert!(!reference.absolute_column);
    assert_eq!(reference.to_a1(), "D3");
}

#[test]
fn display_uses_a1_notation() {
    let reference = CellReference {
        row: 6,
        column: 1,
        absolute_row: true,
        absolute_column: true,
    };

    assert_eq!(reference.to_string(), "$B$7");
}

#[test]
fn anchored_parsing_reports_the_anchors() {
    let parsed = CellAddress::try_parse_a1_anchored("$B$7").unwrap();

    assert_eq!(parsed.row, 6);
    assert_eq!(parsed.column, 1);
    assert!(parsed.absolute_row);
    assert!(parsed.absolute_column);
}

#[test]
fn partially_anchored_parsing_reports_one_anchor() {
    let parsed = CellAddress::try_parse_a1_anchored("$B7").unwrap();

    assert!(!parsed.absolute_row);
    assert!(parsed.absolute_column);
}

#[test]
fn unanchored_parsing_reports_no_anchors() {
    let parsed = CellAddress::try_parse_a1_anchored("b7").unwrap();

    assert!(!parsed.absolute_row);
    assert!(!parsed.absolute_column);
}

#[test]
fn malformed_anchors_are_still_rejected() {
    for text in ["$$A1", "$", "A$"] {
        assert!(CellAddress::try_parse_a1_anchored(text).is_none(), "'{text}'");
    }
}
