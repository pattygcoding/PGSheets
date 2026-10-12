//! A1 addressing: parsing it, formatting it, and refusing what is not a reference.

use pgsheets_core::{CellAddress, MAX_COLUMNS, MAX_ROWS};

#[test]
fn parses_a1_notation() {
    for (text, expected_row, expected_column) in [
        ("A1", 0, 0),
        ("B7", 6, 1),
        ("AA1", 0, 26),
        ("$B$7", 6, 1),
        ("b7", 6, 1),
        ("XFD1048576", MAX_ROWS - 1, MAX_COLUMNS - 1),
    ] {
        let (row, column) =
            CellAddress::try_parse_a1(text).unwrap_or_else(|| panic!("'{text}' should have parsed"));

        assert_eq!(row, expected_row);
        assert_eq!(column, expected_column);
    }
}

#[test]
fn rejects_malformed_notation() {
    for text in [
        "", "A", "1", "1A", "A0", "A1B", "ABCD1", // four letters is not a valid column
        "A-1", "$", "$$A1",
    ] {
        assert!(
            CellAddress::try_parse_a1(text).is_none(),
            "'{text}' should have been refused"
        );
    }
}

#[test]
fn formats_a1_notation() {
    assert_eq!(CellAddress::to_a1(0, 0), "A1");
    assert_eq!(CellAddress::to_a1(6, 1), "B7");
    assert_eq!(CellAddress::to_a1(0, 26), "AA1");
}

#[test]
fn display_uses_a1_notation() {
    assert_eq!(CellAddress::new(3, 2).to_string(), "C4");
}

#[test]
fn equality_is_by_coordinate() {
    assert_eq!(CellAddress::new(1, 2), CellAddress::new(1, 2));
    assert_ne!(CellAddress::new(1, 2), CellAddress::new(2, 1));
}

#[test]
#[should_panic(expected = "a column index cannot be negative")]
fn negative_columns_are_rejected_when_formatting() {
    CellAddress::column_name(-1);
}

#[test]
#[should_panic(expected = "a row index cannot be negative")]
fn negative_rows_are_rejected_when_formatting() {
    CellAddress::to_a1(-1, 0);
}
