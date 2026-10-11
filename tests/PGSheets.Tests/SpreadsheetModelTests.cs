using PGSheets.Core;

namespace PGSheets.Tests;

public class SpreadsheetModelTests
{
    [Fact]
    public void Only_populated_cells_are_stored()
    {
        var model = Sheet.CreateModel();

        Assert.Equal(0, model.CellCount);

        Sheet.Set(model, "A1", "1");
        Sheet.Set(model, "Z100", "2");

        Assert.Equal(2, model.CellCount);

        Sheet.Set(model, "A1", string.Empty);

        Assert.Equal(1, model.CellCount);
    }

    [Fact]
    public void Raw_input_and_display_value_are_kept_apart()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "=1+2");
        var (row, column) = Sheet.At("A1");

        Assert.Equal("=1+2", model.GetRawInput(row, column));
        Assert.Equal("3", model.GetDisplayValue(row, column));
        Assert.True(model.GetCell(row, column)!.IsFormula);
    }

    [Fact]
    public void Deleting_a_precedent_recalculates_dependants()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "5");
        Sheet.Set(model, "B1", "=A1+1");

        Assert.Equal("6", Sheet.Display(model, "B1"));

        Sheet.Set(model, "A1", string.Empty);

        Assert.Equal("1", Sheet.Display(model, "B1"));
    }

    [Fact]
    public void Bulk_update_defers_recalculation_until_the_end()
    {
        var model = Sheet.CreateModel();

        var changeNotifications = 0;
        model.Changed += (_, _) => changeNotifications++;

        model.BeginBulkUpdate();

        Sheet.Set(model, "A1", "2");
        Sheet.Set(model, "A2", "3");
        Sheet.Set(model, "A3", "=SUM(A1:A2)");

        Assert.Equal(0, changeNotifications);

        model.EndBulkUpdate();

        Assert.Equal("5", Sheet.Display(model, "A3"));
        Assert.Equal(1, changeNotifications);
    }

    [Fact]
    public void Clearing_removes_everything()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "1");
        Sheet.Set(model, "B2", "=A1*2");

        model.Clear();

        Assert.Equal(0, model.CellCount);
        Assert.Equal(string.Empty, Sheet.Display(model, "B2"));
    }

    [Fact]
    public void Address_round_trips_through_a1_notation()
    {
        Assert.Equal("A1", CellAddress.ToA1(0, 0));
        Assert.Equal("B7", CellAddress.ToA1(6, 1));
        Assert.Equal("AA1", CellAddress.ToA1(0, 26));
        Assert.Equal("XFD1048576", CellAddress.ToA1(SheetBounds.MaxRows - 1, SheetBounds.MaxColumns - 1));

        Assert.True(CellAddress.TryParseA1("$B$7", out var row, out var column));
        Assert.Equal(6, row);
        Assert.Equal(1, column);

        Assert.False(CellAddress.TryParseA1("A0", out _, out _));
        Assert.False(CellAddress.TryParseA1("1A", out _, out _));
    }
}
