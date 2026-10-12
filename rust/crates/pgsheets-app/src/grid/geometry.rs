//! The grid's layout arithmetic: where a cell is on screen, and which cell a point is over.
//!
//! Nothing here touches a window. Every answer depends only on the layout metrics, the size of the
//! drawing area and the scroll offset, which is what makes hit-testing, scrolling and selection
//! testable headlessly — the same property the C# control had, and the reason the geometry lives in a
//! type of its own rather than inside the widget.

use pgsheets_core::{CellAddress, CellRange, MAX_COLUMNS, MAX_ROWS};

/// The layout metrics of the grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    /// Width of a data column, in points.
    pub column_width: f64,

    /// Height of a data row.
    pub row_height: f64,

    /// Width of the sticky row-header strip.
    pub header_width: f64,

    /// Height of the sticky column-header strip.
    pub header_height: f64,

    /// Font size used for cell contents.
    pub font_size: f32,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            column_width: 100.0,
            row_height: 24.0,
            header_width: 52.0,
            header_height: 24.0,
            font_size: 13.0,
        }
    }
}

/// A rectangle, in points, relative to the grid's top-left corner.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CellRect {
    /// Left edge.
    pub x: f64,

    /// Top edge.
    pub y: f64,

    /// Width.
    pub width: f64,

    /// Height.
    pub height: f64,
}

impl CellRect {
    /// True when a point falls inside the rectangle.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

/// The size of an area, in points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Area {
    /// Width.
    pub width: f64,

    /// Height.
    pub height: f64,
}

/// Where the grid is scrolled to, what is selected, and how big its cells are.
#[derive(Clone, Debug)]
pub struct GridLayout {
    metrics: Metrics,
    size: Area,
    scroll_x: f64,
    scroll_y: f64,

    /// The corner a selection was started from. Equal to the active cell when nothing is selected,
    /// which is why a plain move collapses the selection.
    anchor: CellAddress,

    /// The active cell: the moving end of the selection, and where keyboard input lands.
    active: CellAddress,

    row_count: i32,
    column_count: i32,
}

impl Default for GridLayout {
    fn default() -> Self {
        Self::new(Metrics::default())
    }
}

impl GridLayout {
    /// A layout over the whole virtual sheet, scrolled to A1 with A1 selected.
    pub fn new(metrics: Metrics) -> Self {
        Self {
            metrics,
            size: Area::default(),
            scroll_x: 0.0,
            scroll_y: 0.0,
            anchor: CellAddress::new(0, 0),
            active: CellAddress::new(0, 0),
            row_count: MAX_ROWS,
            column_count: MAX_COLUMNS,
        }
    }

    /// The layout metrics.
    pub fn metrics(&self) -> Metrics {
        self.metrics
    }

    /// Replaces the layout metrics, clamping the scroll offset because they change what "fully
    /// scrolled" means.
    pub fn set_metrics(&mut self, metrics: Metrics) -> bool {
        if metrics == self.metrics {
            return false;
        }

        self.metrics = metrics;
        let clamped = self.clamp_scroll();
        self.ensure_cell_visible(self.active.row, self.active.column) || clamped
    }

    /// Records the size of the drawing area.
    pub fn set_size(&mut self, size: Area) -> bool {
        if size == self.size {
            return false;
        }

        self.size = size;
        let clamped = self.clamp_scroll();
        self.ensure_cell_visible(self.active.row, self.active.column) || clamped
    }

    /// The size of the drawing area.
    pub fn size(&self) -> Area {
        self.size
    }

    /// How many virtual rows the grid shows.
    pub fn row_count(&self) -> i32 {
        self.row_count
    }

    /// How many virtual columns the grid shows.
    pub fn column_count(&self) -> i32 {
        self.column_count
    }

    /// Sets the number of virtual rows, clamped to the sheet's limits.
    pub fn set_row_count(&mut self, rows: i32) {
        self.row_count = rows.clamp(1, MAX_ROWS);
        self.set_active_cell(self.active.row, self.active.column);
    }

    /// Sets the number of virtual columns, clamped to the sheet's limits.
    pub fn set_column_count(&mut self, columns: i32) {
        self.column_count = columns.clamp(1, MAX_COLUMNS);
        self.set_active_cell(self.active.row, self.active.column);
    }

    /// The scroll offset, in points.
    pub fn scroll_offset(&self) -> (f64, f64) {
        (self.scroll_x, self.scroll_y)
    }

    /// The total content size, which is what a scroll bar's range is measured against.
    pub fn extent(&self) -> Area {
        Area {
            width: f64::from(self.column_count) * self.metrics.column_width,
            height: f64::from(self.row_count) * self.metrics.row_height,
        }
    }

    /// The scrollable body: everything right of the row header and below the column header.
    pub fn viewport(&self) -> Area {
        Area {
            width: (self.size.width - self.metrics.header_width).max(0.0),
            height: (self.size.height - self.metrics.header_height).max(0.0),
        }
    }

    /// Left edge of a column's content area.
    pub fn column_x(&self, column: i32) -> f64 {
        self.metrics.header_width + f64::from(column) * self.metrics.column_width - self.scroll_x
    }

    /// Top edge of a row's content area.
    pub fn row_y(&self, row: i32) -> f64 {
        self.metrics.header_height + f64::from(row) * self.metrics.row_height - self.scroll_y
    }

    /// The on-screen rectangle of a cell.
    ///
    /// The host uses this to put the floating editor exactly over the active cell, which is what keeps
    /// one editor serving the whole sheet.
    pub fn cell_bounds(&self, row: i32, column: i32) -> CellRect {
        CellRect {
            x: self.column_x(column),
            y: self.row_y(row),
            width: self.metrics.column_width,
            height: self.metrics.row_height,
        }
    }

    /// Maps a point to a cell; `None` over the headers or outside the sheet.
    pub fn try_hit_test(&self, x: f64, y: f64) -> Option<CellAddress> {
        if x < self.metrics.header_width || y < self.metrics.header_height {
            return None;
        }

        let column = ((x - self.metrics.header_width + self.scroll_x) / self.metrics.column_width) as i32;
        let row = ((y - self.metrics.header_height + self.scroll_y) / self.metrics.row_height) as i32;

        if column < 0 || column >= self.column_count || row < 0 || row >= self.row_count {
            return None;
        }

        Some(CellAddress::new(row, column))
    }

    /// The rows the viewport covers, as inclusive bounds.
    pub fn visible_rows(&self) -> (i32, i32) {
        let first = ((self.scroll_y / self.metrics.row_height) as i32).max(0);
        let last =
            (first + (self.viewport().height / self.metrics.row_height) as i32 + 1).min(self.row_count - 1);

        (first, last)
    }

    /// The columns the viewport covers, as inclusive bounds.
    pub fn visible_columns(&self) -> (i32, i32) {
        let first = ((self.scroll_x / self.metrics.column_width) as i32).max(0);
        let last = (first + (self.viewport().width / self.metrics.column_width) as i32 + 1)
            .min(self.column_count - 1);

        (first, last)
    }

    /// The selected block, always containing the active cell.
    ///
    /// Normalised, so the anchor may be either corner and a range may be dragged in any direction.
    pub fn selection(&self) -> CellRange {
        CellRange::new(
            self.anchor.row,
            self.anchor.column,
            self.active.row,
            self.active.column,
        )
    }

    /// The active cell: where keyboard input lands, and the moving end of the selection.
    pub fn active_cell(&self) -> CellAddress {
        self.active
    }

    /// Moves the active cell and collapses the selection onto it.
    pub fn set_active_cell(&mut self, row: i32, column: i32) -> bool {
        self.set_selection(row, column, row, column)
    }

    /// Moves the active cell by a delta, collapsing the selection.
    pub fn move_active_by(&mut self, row_delta: i32, column_delta: i32) -> bool {
        self.set_active_cell(self.active.row + row_delta, self.active.column + column_delta)
    }

    /// Moves the active cell while keeping the anchor, growing or shrinking the selection.
    pub fn extend_selection_to(&mut self, row: i32, column: i32) -> bool {
        self.set_selection(self.anchor.row, self.anchor.column, row, column)
    }

    /// Extends the selection by a delta.
    pub fn extend_active_by(&mut self, row_delta: i32, column_delta: i32) -> bool {
        self.extend_selection_to(self.active.row + row_delta, self.active.column + column_delta)
    }

    /// Selects a whole block.
    pub fn select_range(&mut self, range: CellRange) -> bool {
        self.set_selection(range.row1, range.column1, range.row2, range.column2)
    }

    /// Selects the smallest rectangle covering every populated cell, or A1 when there is none.
    ///
    /// The cells are passed in rather than read from the workbook, so this stays arithmetic and can be
    /// tested without one.
    pub fn select_used_range(&mut self, cells: impl Iterator<Item = CellAddress>) -> bool {
        let mut min_row = i32::MAX;
        let mut min_column = i32::MAX;
        let mut max_row = i32::MIN;
        let mut max_column = i32::MIN;

        for cell in cells {
            min_row = min_row.min(cell.row);
            max_row = max_row.max(cell.row);
            min_column = min_column.min(cell.column);
            max_column = max_column.max(cell.column);
        }

        // Starting from A1 would sweep in empty leading rows and columns, so the box is the data's.
        let range = if max_row < 0 {
            CellRange::single(0, 0)
        } else {
            CellRange::new(min_row, min_column, max_row, max_column)
        };

        self.select_range(range)
    }

    /// Scrolls back to A1 and collapses the selection onto the first cell. True when it moved.
    pub fn reset_view(&mut self) -> bool {
        let changed =
            self.scroll_x != 0.0 || self.scroll_y != 0.0 || self.selection() != CellRange::single(0, 0);

        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
        self.anchor = CellAddress::new(0, 0);
        self.active = CellAddress::new(0, 0);

        changed
    }

    /// Sets the scroll offset, clamped to what the sheet can show. True when it moved.
    pub fn set_scroll_offset(&mut self, x: f64, y: f64) -> bool {
        let extent = self.extent();
        let viewport = self.viewport();

        let x = clamp(x, 0.0, (extent.width - viewport.width).max(0.0));
        let y = clamp(y, 0.0, (extent.height - viewport.height).max(0.0));

        if x == self.scroll_x && y == self.scroll_y {
            return false;
        }

        self.scroll_x = x;
        self.scroll_y = y;
        true
    }

    /// Scrolls so that a cell is fully visible, which is how a keyboard move stays on screen.
    pub fn ensure_cell_visible(&mut self, row: i32, column: i32) -> bool {
        let viewport = self.viewport();

        let left = f64::from(column) * self.metrics.column_width;
        let right = left + self.metrics.column_width;
        let top = f64::from(row) * self.metrics.row_height;
        let bottom = top + self.metrics.row_height;

        let mut x = self.scroll_x;
        let mut y = self.scroll_y;

        if left < x {
            x = left;
        } else if right > x + viewport.width {
            x = right - viewport.width;
        }

        if top < y {
            y = top;
        } else if bottom > y + viewport.height {
            y = bottom - viewport.height;
        }

        self.set_scroll_offset(x, y)
    }

    /// The selection setter every other one goes through: clamps both ends to the sheet, keeps the
    /// view on the active cell, and reports whether anything changed.
    fn set_selection(&mut self, anchor_row: i32, anchor_column: i32, row: i32, column: i32) -> bool {
        let anchor = CellAddress::new(
            anchor_row.clamp(0, self.row_count - 1),
            anchor_column.clamp(0, self.column_count - 1),
        );

        let active = CellAddress::new(
            row.clamp(0, self.row_count - 1),
            column.clamp(0, self.column_count - 1),
        );

        let changed = anchor != self.anchor || active != self.active;

        self.anchor = anchor;
        self.active = active;
        self.ensure_cell_visible(active.row, active.column);

        changed
    }

    /// Clamps the scroll offset to the sheet, which a resize can leave out of range.
    fn clamp_scroll(&mut self) -> bool {
        self.set_scroll_offset(self.scroll_x, self.scroll_y)
    }
}

fn clamp(value: f64, minimum: f64, maximum: f64) -> f64 {
    value.max(minimum).min(maximum)
}
