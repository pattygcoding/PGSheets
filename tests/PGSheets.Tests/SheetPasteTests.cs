using PGSheets.Core;

namespace PGSheets.Tests;

public class SheetPasteTests
{
    private static CellBuffer Block(params (string A1, string Value)[] cells)
    {
        var buffer = new CellBuffer();

        foreach (var (a1, value) in cells)
        {
            var (row, column) = Sheet.At(a1);
            buffer.SetRawInput(row, column, value);
        }

        return buffer;
    }

    [Fact]
    public void A_shifted_paste_moves_relative_references()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "2");
        var block = Block(("A1", "=A1"));

        SheetPaste.Apply(block, model, new CellRange(4, 0, 4, 0), rowDelta: 4, columnDelta: 0);

        Assert.Equal("=A5", model.GetRawInput(4, 0));
    }

    [Fact]
    public void A_shifted_paste_leaves_anchored_references_alone()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "2");
        var block = Block(("A1", "=$A$1"));

        SheetPaste.Apply(block, model, new CellRange(4, 0, 4, 0), rowDelta: 4, columnDelta: 0);

        Assert.Equal("=$A$1", model.GetRawInput(4, 0));
    }

    [Theory]
    [InlineData(0, 1, 5, 1, 5, 0)]
    [InlineData(0, 0, 0, 0, 0, 0)]
    [InlineData(2, 3, 0, 0, -2, -3)]
    [InlineData(1, 1, 3, 4, 2, 3)]
    public void ShiftFor_is_the_distance_between_origins(
        int fromRow,
        int fromColumn,
        int toRow,
        int toColumn,
        int expectedRowDelta,
        int expectedColumnDelta)
    {
        var shift = SheetPaste.ShiftFor(
            new CellRange(fromRow, fromColumn, fromRow, fromColumn),
            new CellRange(toRow, toColumn, toRow, toColumn));

        Assert.Equal(expectedRowDelta, shift.RowDelta);
        Assert.Equal(expectedColumnDelta, shift.ColumnDelta);
    }

    [Fact]
    public void Writes_a_block_at_the_target_corner()
    {
        var model = Sheet.CreateModel();
        var block = Block(("A1", "a"), ("B1", "b"), ("A2", "c"), ("B2", "d"));

        var (written, covered) = SheetPaste.Apply(block, model, new CellRange(2, 2, 2, 2));

        Assert.Equal(4, written);
        Assert.Equal(new CellRange(2, 2, 3, 3), covered);
        Assert.Equal("a", Sheet.Display(model, "C3"));
        Assert.Equal("d", Sheet.Display(model, "D4"));
    }

    [Fact]
    public void A_single_cell_fills_the_whole_selection()
    {
        var model = Sheet.CreateModel();
        var block = Block(("A1", "x"));

        var (written, covered) = SheetPaste.Apply(block, model, new CellRange(0, 0, 2, 2));

        Assert.Equal(9, written);
        Assert.Equal(new CellRange(0, 0, 2, 2), covered);
        Assert.Equal("x", Sheet.Display(model, "C3"));
    }

    [Fact]
    public void A_block_is_tiled_to_cover_a_larger_selection()
    {
        var model = Sheet.CreateModel();
        var block = Block(("A1", "1"), ("B1", "2"));

        var (written, covered) = SheetPaste.Apply(block, model, new CellRange(0, 0, 3, 1));

        Assert.Equal(8, written);
        Assert.Equal(new CellRange(0, 0, 3, 1), covered);
        Assert.Equal("1", Sheet.Display(model, "A4"));
        Assert.Equal("2", Sheet.Display(model, "B4"));
    }

    [Fact]
    public void An_empty_field_clears_the_destination()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "old");

        var block = Block(("A1", string.Empty), ("B1", string.Empty));

        SheetPaste.Apply(block, model, new CellRange(0, 0, 0, 1));

        Assert.Equal(string.Empty, Sheet.Display(model, "A1"));
        Assert.Equal(0, model.CellCount);
    }

    [Fact]
    public void The_paste_is_a_single_undo_step()
    {
        var model = Sheet.CreateModel();
        var block = Block(("A1", "a"), ("B1", "b"));

        SheetPaste.Apply(block, model, new CellRange(0, 0, 0, 1));

        Assert.Equal(1, model.Undo.UndoDepth);

        model.Undo.Undo();

        Assert.Equal(0, model.CellCount);
    }

    [Fact]
    public void Formulae_paste_as_formulae()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "2");
        var block = Block(("A2", "=A1*3"));

        SheetPaste.Apply(block, model, new CellRange(1, 0, 1, 0));

        Assert.Equal("6", Sheet.Display(model, "A2"));
        Assert.Equal("=A1*3", model.GetRawInput(1, 0));
    }

    [Fact]
    public void An_empty_block_writes_nothing()
    {
        var model = Sheet.CreateModel();

        var (written, covered) = SheetPaste.Apply(new CellBuffer(), model, new CellRange(1, 1, 1, 1));

        Assert.Equal(0, written);
        Assert.Equal(new CellRange(1, 1, 1, 1), covered);
        Assert.False(model.Undo.CanUndo);
    }

    [Fact]
    public void Writes_stay_inside_the_sheet()
    {
        var model = Sheet.CreateModel();
        var block = Block(("A1", "a"), ("B1", "b"));

        // The target's second column is already the last column in the sheet.
        var lastColumn = SheetBounds.MaxColumns - 1;

        var (_, covered) = SheetPaste.Apply(block, model, new CellRange(0, lastColumn, 0, lastColumn));

        Assert.Equal(lastColumn, covered.Column2);
        Assert.Equal("a", Sheet.Display(model, "XFD1"));
    }
}
