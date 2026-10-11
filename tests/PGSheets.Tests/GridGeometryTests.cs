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

    // ---- Range selection ----------------------------------------------------------------------

    [Fact]
    public void A_new_grid_has_a_single_cell_selected()
    {
        var grid = CreateGrid();

        Assert.Equal(new Core.CellRange(0, 0, 0, 0), grid.Selection);
        Assert.True(grid.Selection.IsSingleCell);
    }

    [Fact]
    public void A_plain_move_collapses_the_selection()
    {
        var grid = CreateGrid();
        grid.SelectRange(new Core.CellRange(1, 1, 3, 3));

        grid.SetActiveCell(2, 2);

        Assert.Equal(new Core.CellRange(2, 2, 2, 2), grid.Selection);
    }

    [Fact]
    public void Extending_grows_the_selection_from_the_anchor()
    {
        var grid = CreateGrid();
        grid.SetActiveCell(1, 1);

        grid.ExtendSelectionTo(4, 3);

        Assert.Equal(new Core.CellRange(1, 1, 4, 3), grid.Selection);
        Assert.Equal(4, grid.ActiveRow);
        Assert.Equal(3, grid.ActiveColumn);
    }

    [Fact]
    public void Extending_up_and_left_normalises_the_selection()
    {
        var grid = CreateGrid();
        grid.SetActiveCell(5, 5);

        grid.ExtendSelectionTo(2, 2);

        Assert.Equal(new Core.CellRange(2, 2, 5, 5), grid.Selection);
    }

    [Fact]
    public void Extending_by_a_delta_keeps_the_anchor()
    {
        var grid = CreateGrid();
        grid.SetActiveCell(2, 2);

        grid.ExtendActiveBy(1, 2);

        Assert.Equal(new Core.CellRange(2, 2, 3, 4), grid.Selection);
    }

    [Fact]
    public void Selection_changed_carries_the_range()
    {
        var grid = CreateGrid();
        Core.CellRange reported = default;
        grid.SelectionChanged += (_, e) => reported = e.Range;

        grid.SetActiveCell(1, 1);
        grid.ExtendSelectionTo(3, 4);

        Assert.Equal(new Core.CellRange(1, 1, 3, 4), reported);
    }

    [Fact]
    public void Changing_only_the_anchor_still_raises_selection_changed()
    {
        var grid = CreateGrid();
        grid.SetActiveCell(2, 2);

        var raised = 0;
        grid.SelectionChanged += (_, _) => raised++;

        // The active cell stays put; only the anchor moves.
        grid.SelectRange(new Core.CellRange(2, 2, 5, 5));

        Assert.Equal(1, raised);
        Assert.Equal(new Core.CellRange(2, 2, 5, 5), grid.Selection);
    }

    [Fact]
    public void Select_used_range_covers_the_data()
    {
        var grid = CreateGrid();
        var model = Sheet.CreateModel();
        Sheet.Set(model, "B2", "x");
        Sheet.Set(model, "D4", "y");
        grid.Model = model;

        grid.SelectUsedRange();

        Assert.Equal(new Core.CellRange(1, 1, 3, 3), grid.Selection);
    }

    [Fact]
    public void Select_used_range_on_an_empty_sheet_selects_a1()
    {
        var grid = CreateGrid();
        grid.Model = Sheet.CreateModel();

        grid.SelectUsedRange();

        Assert.Equal(new Core.CellRange(0, 0, 0, 0), grid.Selection);
    }

    [Fact]
    public void Clearing_a_selection_is_one_undo_step()
    {
        var grid = CreateGrid();
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "a");
        Sheet.Set(model, "B1", "b");
        Sheet.Set(model, "A2", "c");
        grid.Model = model;

        // Setting up the cells was itself three edits; only the clear should be left to undo.
        model.Undo.Clear();

        grid.SelectRange(new Core.CellRange(0, 0, 1, 1));
        grid.ClearSelection();

        Assert.Equal(0, model.CellCount);
        Assert.Equal(1, model.Undo.UndoDepth);

        model.Undo.Undo();

        Assert.Equal(3, model.CellCount);
    }

    [Fact]
    public void Clearing_leaves_cells_outside_the_selection_alone()
    {
        var grid = CreateGrid();
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "inside");
        Sheet.Set(model, "C3", "outside");
        grid.Model = model;

        grid.SelectRange(new Core.CellRange(0, 0, 0, 0));
        grid.ClearSelection();

        Assert.Equal(string.Empty, Sheet.Display(model, "A1"));
        Assert.Equal("outside", Sheet.Display(model, "C3"));
    }

    [Fact]
    public void Clearing_an_empty_selection_records_nothing()
    {
        var grid = CreateGrid();
        grid.Model = Sheet.CreateModel();

        grid.SelectRange(new Core.CellRange(0, 0, 5, 5));
        grid.ClearSelection();

        Assert.Equal(0, grid.Model.CellCount);
    }
}
