//! Geometry and hit-testing of the grid, ported from the C# suite's `GridGeometryTests`.
//!
//! These are the parts of the grid that need no window: the layout depends only on the metrics and the
//! scroll offset, which is exactly why it was separated out.

use pgsheets_app::grid::{Area, GridLayout, Metrics, Scrollbar};

use pgsheets_core::{CellAddress, CellRange, MAX_COLUMNS, MAX_ROWS};

/// The metrics the tests use: a column is 100 wide, a row 20, the headers 50 and 25.
fn layout() -> GridLayout {
    GridLayout::new(Metrics {
        column_width: 100.0,
        row_height: 20.0,
        header_width: 50.0,
        header_height: 25.0,
        font_size: 13.0,
    })
}

/// A layout with a viewport, so that scrolling has bounds to clamp against.
fn with_size() -> GridLayout {
    let mut grid = layout();

    grid.set_size(Area {
        width: 800.0,
        height: 600.0,
    });

    grid
}

#[test]
fn hit_test_maps_points_to_cells() {
    let grid = layout();

    // Column A starts at x = 50; row 1 starts at y = 25.
    assert_eq!(grid.try_hit_test(50.0, 25.0), Some(CellAddress::new(0, 0)));
    assert_eq!(grid.try_hit_test(149.0, 44.0), Some(CellAddress::new(0, 0)));
    assert_eq!(grid.try_hit_test(150.0, 45.0), Some(CellAddress::new(1, 1)));
}

#[test]
fn hit_test_rejects_the_header_strips() {
    let grid = layout();

    assert_eq!(grid.try_hit_test(10.0, 100.0), None); // row header
    assert_eq!(grid.try_hit_test(100.0, 10.0), None); // column header
    assert_eq!(grid.try_hit_test(10.0, 10.0), None); // corner
}

#[test]
fn hit_test_accounts_for_the_scroll_offset() {
    let mut grid = with_size();

    // Exactly one column width and two row heights.
    grid.set_scroll_offset(100.0, 40.0);

    assert_eq!(grid.try_hit_test(50.0, 25.0), Some(CellAddress::new(2, 1)));
}

#[test]
fn cell_bounds_describe_the_cell_rectangle() {
    let grid = layout();
    let bounds = grid.cell_bounds(2, 1); // row 3, column B

    assert_eq!(bounds.x, 150.0);
    assert_eq!(bounds.y, 65.0);
    assert_eq!(bounds.width, 100.0);
    assert_eq!(bounds.height, 20.0);
}

#[test]
fn cell_bounds_move_with_scrolling() {
    let mut grid = with_size();
    grid.set_scroll_offset(30.0, 10.0);

    let bounds = grid.cell_bounds(1, 1);

    assert_eq!(bounds.x, 120.0); // 50 + 100 - 30
    assert_eq!(bounds.y, 35.0); // 25 + 20 - 10
}

#[test]
fn scroll_offset_is_clamped_to_zero() {
    let mut grid = with_size();
    grid.set_scroll_offset(-500.0, -500.0);

    assert_eq!(grid.scroll_offset(), (0.0, 0.0));
}

#[test]
fn active_cell_is_clamped_to_the_sheet_bounds() {
    let mut grid = layout();
    grid.set_column_count(5);
    grid.set_row_count(3);

    grid.set_active_cell(100, 100);
    assert_eq!(grid.active_cell(), CellAddress::new(2, 4));

    grid.set_active_cell(-5, -5);
    assert_eq!(grid.active_cell(), CellAddress::new(0, 0));
}

#[test]
fn moving_the_active_cell_reports_a_change_once() {
    let mut grid = layout();

    assert!(grid.set_active_cell(3, 2));
    assert!(!grid.set_active_cell(3, 2), "the same cell is not a change");
}

#[test]
fn keyboard_navigation_moves_the_active_cell() {
    let mut grid = layout();
    grid.set_active_cell(4, 1);

    assert!(grid.move_active_by(1, 2));

    assert_eq!(grid.active_cell(), CellAddress::new(5, 3));
}

#[test]
fn reset_view_returns_to_a1() {
    let mut grid = with_size();
    grid.set_active_cell(9, 4);
    grid.set_scroll_offset(120.0, 60.0);

    assert!(grid.reset_view());

    assert_eq!(grid.active_cell(), CellAddress::new(0, 0));
    assert_eq!(grid.scroll_offset(), (0.0, 0.0));
}

#[test]
fn row_and_column_counts_are_clamped_to_the_sheet_limits() {
    let mut grid = layout();

    grid.set_row_count(i32::MAX);
    grid.set_column_count(i32::MAX);

    assert_eq!(grid.row_count(), MAX_ROWS);
    assert_eq!(grid.column_count(), MAX_COLUMNS);

    grid.set_row_count(0);
    grid.set_column_count(0);

    assert_eq!(grid.row_count(), 1);
    assert_eq!(grid.column_count(), 1);
}

#[test]
fn scrolling_reports_a_change_once() {
    let mut grid = with_size();

    assert!(grid.set_scroll_offset(50.0, 20.0));
    assert!(
        !grid.set_scroll_offset(50.0, 20.0),
        "an unchanged offset is not a change"
    );
}

#[test]
fn extent_is_the_virtual_sheet_size() {
    let mut grid = layout();
    grid.set_row_count(1000);
    grid.set_column_count(12);

    assert_eq!(grid.extent().width, 1200.0);
    assert_eq!(grid.extent().height, 20000.0);
}

#[test]
fn a_scrollbar_thumb_is_proportional_to_what_is_visible() {
    let bar = Scrollbar {
        extent: 1000.0,
        viewport: 250.0,
        offset: 0.0,
    };

    assert_eq!(bar.max_offset(), 750.0);
    assert!(!bar.is_settled());
    assert_eq!(bar.thumb_length(400.0), 100.0, "a quarter of the content");
    assert_eq!(bar.thumb_start(400.0), 0.0);
}

#[test]
fn the_thumb_moves_with_the_offset_and_never_leaves_the_track() {
    let bar = Scrollbar {
        extent: 1000.0,
        viewport: 250.0,
        offset: 375.0,
    };

    // Half-way scrolled, so the thumb starts half-way along what is left of the track.
    assert_eq!(bar.thumb_start(400.0), 150.0);

    let at_end = Scrollbar {
        extent: 1000.0,
        viewport: 250.0,
        offset: 750.0,
    };

    assert_eq!(at_end.thumb_start(400.0), 300.0, "the thumb ends at the end");
}

#[test]
fn a_tiny_viewport_still_leaves_a_thumb_that_can_be_grabbed() {
    let bar = Scrollbar {
        extent: 1_000_000.0,
        viewport: 10.0,
        offset: 0.0,
    };

    assert_eq!(bar.thumb_length(400.0), 24.0);
}

#[test]
fn a_settled_scrollbar_has_nothing_to_say() {
    let bar = Scrollbar {
        extent: 200.0,
        viewport: 400.0,
        offset: 0.0,
    };

    assert!(bar.is_settled());
    assert_eq!(bar.max_offset(), 0.0);
    assert_eq!(bar.thumb_start(400.0), 0.0);
    assert_eq!(bar.thumb_length(400.0), 400.0, "the whole track");
}

#[test]
fn dragging_the_thumb_maps_a_position_back_to_an_offset() {
    let bar = Scrollbar {
        extent: 1000.0,
        viewport: 250.0,
        offset: 0.0,
    };

    assert_eq!(bar.offset_at(400.0, 0.0), 0.0);
    assert_eq!(bar.offset_at(400.0, 300.0), 750.0);
    assert_eq!(bar.offset_at(400.0, 150.0), 375.0);

    // Past the end of the track is still the end of the content.
    assert_eq!(bar.offset_at(400.0, 5000.0), 750.0);
}

#[test]
fn clicking_the_track_pages_by_a_viewport() {
    let bar = Scrollbar {
        extent: 1000.0,
        viewport: 250.0,
        offset: 250.0,
    };

    assert_eq!(bar.page(true), 500.0);
    assert_eq!(bar.page(false), 0.0);

    let at_end = Scrollbar {
        extent: 1000.0,
        viewport: 250.0,
        offset: 740.0,
    };

    assert_eq!(at_end.page(true), 750.0, "and it stops at the end");
}

#[test]
fn a_new_grid_has_a_single_cell_selected() {
    let grid = layout();

    assert_eq!(grid.selection(), CellRange::single(0, 0));
    assert!(grid.selection().is_single_cell());
}

#[test]
fn a_plain_move_collapses_the_selection() {
    let mut grid = layout();
    grid.select_range(CellRange::new(1, 1, 3, 3));

    grid.set_active_cell(2, 2);

    assert_eq!(grid.selection(), CellRange::single(2, 2));
}

#[test]
fn extending_grows_the_selection_from_the_anchor() {
    let mut grid = layout();
    grid.set_active_cell(1, 1);

    grid.extend_selection_to(4, 3);

    assert_eq!(grid.selection(), CellRange::new(1, 1, 4, 3));
    assert_eq!(grid.active_cell(), CellAddress::new(4, 3));
}

#[test]
fn extending_up_and_left_normalises_the_selection() {
    let mut grid = layout();
    grid.set_active_cell(5, 5);

    grid.extend_selection_to(2, 2);

    assert_eq!(grid.selection(), CellRange::new(2, 2, 5, 5));
}

#[test]
fn extending_by_a_delta_keeps_the_anchor() {
    let mut grid = layout();
    grid.set_active_cell(2, 2);

    grid.extend_active_by(1, 2);

    assert_eq!(grid.selection(), CellRange::new(2, 2, 3, 4));
}

#[test]
fn moving_only_the_anchor_is_still_a_change() {
    let mut grid = layout();
    grid.set_active_cell(2, 2);

    // The active cell stays put; only the anchor moves.
    assert!(grid.select_range(CellRange::new(2, 2, 5, 5)));
    assert_eq!(grid.selection(), CellRange::new(2, 2, 5, 5));
}

#[test]
fn selecting_a_whole_column_is_a_range_over_the_whole_column() {
    let mut grid = with_size();
    grid.set_row_count(50);

    grid.select_range(CellRange::new(0, 3, 49, 3));

    assert_eq!(grid.selection(), CellRange::new(0, 3, 49, 3));
    assert_eq!(grid.selection().row_count(), 50);
}

#[test]
fn select_used_range_covers_the_data() {
    let mut grid = layout();
    let cells = [CellAddress::new(1, 1), CellAddress::new(3, 3)];

    grid.select_used_range(cells.into_iter());

    assert_eq!(grid.selection(), CellRange::new(1, 1, 3, 3));
}

#[test]
fn select_used_range_on_an_empty_sheet_selects_a1() {
    let mut grid = layout();

    grid.select_used_range(std::iter::empty());

    assert_eq!(grid.selection(), CellRange::single(0, 0));
}

#[test]
fn a_keyboard_move_keeps_the_active_cell_on_screen() {
    let mut grid = with_size();

    // Row 100 is well past the bottom of a 600-point viewport.
    grid.set_active_cell(100, 0);

    let (_, scroll_y) = grid.scroll_offset();

    assert!(scroll_y > 0.0, "the view should have followed the active cell");
    assert!(
        grid.cell_bounds(100, 0).y < 600.0,
        "and the cell should be inside the viewport"
    );
}

#[test]
fn the_visible_window_is_the_viewport_only() {
    let grid = with_size();

    let (first_row, last_row) = grid.visible_rows();
    let (first_column, last_column) = grid.visible_columns();

    assert_eq!(first_row, 0);
    assert_eq!(first_column, 0);
    // An 800x600 area with 50/25 headers and 100x20 cells shows a screenful, not the whole sheet.
    assert!(last_row < 40, "rows: {first_row}..{last_row}");
    assert!(last_column < 12, "columns: {first_column}..{last_column}");
}
