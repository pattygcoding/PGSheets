using Avalonia;
using PGSheets.Controls;

namespace PGSheets.Tests;

/// <summary>
/// Geometry and hit-testing of <see cref="SpreadsheetGridControl"/>. These paths depend only on the
/// scroll offset and the layout metrics, so they are testable without a layout pass or a window.
/// </summary>
public class GridGeometryTests
{
    private static SpreadsheetGridControl CreateGrid() => new()
    {
        ColumnWidth = 100d,
        RowHeight = 20d,
        HeaderWidth = 50d,
        HeaderHeight = 25d,
    };

    [Fact]
    public void Hit_test_maps_points_to_cells()
    {
        var grid = CreateGrid();

        // Column A starts at x = 50; row 1 starts at y = 25.
        Assert.True(grid.TryHitTest(new Point(50d, 25d), out var row, out var column));
        Assert.Equal(0, row);
        Assert.Equal(0, column);

        Assert.True(grid.TryHitTest(new Point(149d, 44d), out row, out column));
        Assert.Equal(0, row);
        Assert.Equal(0, column);

        Assert.True(grid.TryHitTest(new Point(150d, 45d), out row, out column));
        Assert.Equal(1, row);
        Assert.Equal(1, column);
    }

    [Fact]
    public void Hit_test_rejects_the_header_strips()
    {
        var grid = CreateGrid();

        Assert.False(grid.TryHitTest(new Point(10d, 100d), out _, out _)); // row header
        Assert.False(grid.TryHitTest(new Point(100d, 10d), out _, out _)); // column header
        Assert.False(grid.TryHitTest(new Point(10d, 10d), out _, out _));  // corner
    }

    [Fact]
    public void Hit_test_accounts_for_the_scroll_offset()
    {
        var grid = CreateGrid();
        grid.SetScrollOffset(100d, 40d); // exactly one column width and two row heights

        Assert.True(grid.TryHitTest(new Point(50d, 25d), out var row, out var column));
        Assert.Equal(1, column);
        Assert.Equal(2, row);
    }

    [Fact]
    public void Cell_bounds_describe_the_cell_rectangle()
    {
        var grid = CreateGrid();
        var bounds = grid.GetCellBounds(2, 1); // row 3, column B

        Assert.Equal(150d, bounds.X, 3);
        Assert.Equal(65d, bounds.Y, 3);
        Assert.Equal(100d, bounds.Width, 3);
        Assert.Equal(20d, bounds.Height, 3);
    }

    [Fact]
    public void Cell_bounds_move_with_scrolling()
    {
        var grid = CreateGrid();
        grid.SetScrollOffset(30d, 10d);

        var bounds = grid.GetCellBounds(1, 1);

        Assert.Equal(120d, bounds.X, 3); // 50 + 100 - 30
        Assert.Equal(35d, bounds.Y, 3);  // 25 + 20 - 10
    }

    [Fact]
    public void Scroll_offset_is_clamped_to_zero()
    {
        var grid = CreateGrid();
        grid.SetScrollOffset(-500d, -500d);

        Assert.Equal(0d, grid.ScrollOffsetX, 3);
        Assert.Equal(0d, grid.ScrollOffsetY, 3);
    }

    [Fact]
    public void Active_cell_is_clamped_to_the_sheet_bounds()
    {
        var grid = CreateGrid();
        grid.ColumnCount = 5;
        grid.RowCount = 3;

        grid.SetActiveCell(100, 100);
        Assert.Equal(2, grid.ActiveRow);
        Assert.Equal(4, grid.ActiveColumn);

        grid.SetActiveCell(-5, -5);
        Assert.Equal(0, grid.ActiveRow);
        Assert.Equal(0, grid.ActiveColumn);
    }

    [Fact]
    public void Active_cell_change_raises_selection_changed_once()
    {
        var grid = CreateGrid();
        var raised = 0;

        grid.SelectionChanged += (_, _) => raised++;

        grid.SetActiveCell(3, 2);
        Assert.Equal(1, raised);

        grid.SetActiveCell(3, 2); // same cell: no event
        Assert.Equal(1, raised);
    }

    [Fact]
    public void Keyboard_navigation_moves_the_active_cell()
    {
        var grid = CreateGrid();
        var selections = 0;

        grid.SelectionChanged += (_, _) => selections++;
        grid.SetActiveCell(4, 1);

        grid.MoveActiveBy(1, 2);

        Assert.Equal(5, grid.ActiveRow);
        Assert.Equal(3, grid.ActiveColumn);
        Assert.Equal(2, selections);
    }

    [Fact]
    public void Reset_view_returns_to_a1()
    {
        var grid = CreateGrid();
        grid.SetActiveCell(9, 4);
        grid.SetScrollOffset(120d, 60d);

        var selections = 0;
        grid.SelectionChanged += (_, e) =>
        {
            selections++;
            Assert.Equal(0, e.Row);
            Assert.Equal(0, e.Column);
        };

        grid.ResetView();

        Assert.Equal(0, grid.ActiveRow);
        Assert.Equal(0, grid.ActiveColumn);
        Assert.Equal(0d, grid.ScrollOffsetX, 3);
        Assert.Equal(0d, grid.ScrollOffsetY, 3);
        Assert.Equal(1, selections);
    }

    [Fact]
    public void Row_and_column_counts_are_clamped_to_the_sheet_limits()
    {
        var grid = CreateGrid();

        grid.RowCount = int.MaxValue;
        grid.ColumnCount = int.MaxValue;

        Assert.Equal(Core.SheetBounds.MaxRows, grid.RowCount);
        Assert.Equal(Core.SheetBounds.MaxColumns, grid.ColumnCount);

        grid.RowCount = 0;
        grid.ColumnCount = 0;

        Assert.Equal(1, grid.RowCount);
        Assert.Equal(1, grid.ColumnCount);
    }

    [Fact]
    public void Scrolling_raises_viewport_changed()
    {
        var grid = CreateGrid();
        var raised = 0;

        grid.ViewportChanged += (_, _) => raised++;

        grid.SetScrollOffset(50d, 20d);
        Assert.Equal(1, raised);

        grid.SetScrollOffset(50d, 20d); // unchanged: no event
        Assert.Equal(1, raised);
    }

    [Fact]
    public void Extent_is_the_virtual_sheet_size()
    {
        var grid = CreateGrid();
        grid.RowCount = 1000;
        grid.ColumnCount = 12;

        Assert.Equal(1200d, grid.ExtentWidth, 3);
        Assert.Equal(20000d, grid.ExtentHeight, 3);
    }

    [Fact]
    public void Column_labels_follow_spreadsheet_conventions()
    {
        Assert.Equal("A", Core.CellAddress.ToColumnName(0));
        Assert.Equal("Z", Core.CellAddress.ToColumnName(25));
        Assert.Equal("AA", Core.CellAddress.ToColumnName(26));
        Assert.Equal("AZ", Core.CellAddress.ToColumnName(51));
        Assert.Equal("BA", Core.CellAddress.ToColumnName(52));
        Assert.Equal("XFD", Core.CellAddress.ToColumnName(Core.SheetBounds.MaxColumns - 1));
    }
}
