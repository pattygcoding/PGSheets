//! The clipboard block: the shape of a pasted region, including its empty fields.

use pgsheets_core::store::ImportSink;
use pgsheets_core::{CellBuffer, CellRange};

#[test]
fn keeps_the_shape_of_the_block() {
    let mut buffer = CellBuffer::new();

    // A 2x2 block whose second column is empty must still cover two columns.
    buffer.set_raw_input(0, 0, "a");
    buffer.set_raw_input(0, 1, "");
    buffer.set_raw_input(1, 0, "c");
    buffer.set_raw_input(1, 1, "");

    assert_eq!(buffer.try_get_bounds(), Some(CellRange::new(0, 0, 1, 1)));
    assert_eq!(buffer.count(), 4);
    assert_eq!(buffer.non_empty_count(), 2);
}

#[test]
fn an_empty_buffer_has_no_bounds() {
    assert_eq!(CellBuffer::new().try_get_bounds(), None);
}

#[test]
fn reads_back_what_it_was_given() {
    let mut buffer = CellBuffer::new();
    buffer.set_raw_input(2, 3, "value");

    assert_eq!(buffer.get(2, 3), "value");
    assert_eq!(buffer.get(0, 0), "");
}

#[test]
fn bounds_span_the_outermost_fields() {
    let mut buffer = CellBuffer::new();
    buffer.set_raw_input(5, 5, "a");
    buffer.set_raw_input(7, 9, "b");

    assert_eq!(buffer.try_get_bounds(), Some(CellRange::new(5, 5, 7, 9)));
}

#[test]
fn cells_are_visited_with_their_coordinates() {
    let mut buffer = CellBuffer::new();
    buffer.set_raw_input(1, 2, "a");
    buffer.set_raw_input(0, 0, "b");

    let mut cells: Vec<(i32, i32, &str)> = buffer.cells().collect();
    cells.sort();

    assert_eq!(cells, vec![(0, 0, "b"), (1, 2, "a")]);
}

#[test]
fn a_number_format_is_ignored() {
    let mut buffer = CellBuffer::new();
    buffer.set_raw_input(0, 0, "1");
    buffer.set_number_format(0, 0, Some("0.00"));

    // A tab-separated clipboard carries values, not formats.
    assert_eq!(buffer.count(), 1);
    assert_eq!(buffer.get(0, 0), "1");
}
