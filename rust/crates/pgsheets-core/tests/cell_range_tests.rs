//! Normalised rectangles and their addressing.

use pgsheets_core::{CellAddress, CellRange};

#[test]
fn normalises_any_two_corners() {
    let range = CellRange::new(4, 3, 1, 0);

    assert_eq!(range.row1, 1);
    assert_eq!(range.row2, 4);
    assert_eq!(range.column1, 0);
    assert_eq!(range.column2, 3);
}

#[test]
fn reports_its_shape() {
    let range = CellRange::new(1, 1, 3, 4);

    assert_eq!(range.row_count(), 3);
    assert_eq!(range.column_count(), 4);
    assert_eq!(range.cell_count(), 12);
    assert!(!range.is_single_cell());
}

#[test]
fn a_single_cell_is_its_own_range() {
    let range = CellRange::single(2, 2);

    assert!(range.is_single_cell());
    assert_eq!(range.cell_count(), 1);
}

#[test]
fn contains_only_cells_inside_it() {
    let range = CellRange::new(1, 1, 2, 2);

    assert!(range.contains(1, 1));
    assert!(range.contains(2, 2));
    assert!(!range.contains(0, 1));
    assert!(!range.contains(3, 1));
    assert!(!range.contains(1, 3));
}

#[test]
fn iteration_visits_every_cell_row_by_row() {
    let cells: Vec<CellAddress> = CellRange::new(0, 0, 1, 1).iter().collect();

    assert_eq!(
        cells,
        vec![
            CellAddress::new(0, 0),
            CellAddress::new(0, 1),
            CellAddress::new(1, 0),
            CellAddress::new(1, 1),
        ]
    );
}

#[test]
fn offset_shifts_the_whole_range() {
    assert_eq!(
        CellRange::new(1, 1, 2, 2).offset(10, 5),
        CellRange::new(11, 6, 12, 7)
    );
}

#[test]
fn formats_as_a1_notation() {
    for (range, expected) in [
        (CellRange::single(0, 0), "A1"),
        (CellRange::new(1, 1, 3, 4), "B2:E4"),
        (CellRange::new(3, 4, 1, 1), "B2:E4"),
    ] {
        assert_eq!(range.to_string(), expected);
    }
}

#[test]
fn equality_is_by_corners() {
    assert_eq!(CellRange::new(1, 1, 2, 2), CellRange::new(2, 2, 1, 1));
    assert_ne!(CellRange::new(1, 1, 2, 2), CellRange::new(1, 1, 2, 3));
}
