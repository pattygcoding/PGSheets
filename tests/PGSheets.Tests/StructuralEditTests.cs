using PGSheets.Core;

namespace PGSheets.Tests;

/// <summary>
/// Inserting and deleting rows and columns: cells move, references follow, and the whole thing is a
/// single undo step.
/// </summary>
public class StructuralEditTests
{
    [Fact]
    public void Inserting_rows_moves_cells_down()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "first");
        Sheet.Set(model, "A2", "second");

        Assert.True(model.InsertRows(1, 1));

        Assert.Equal("first", Sheet.Display(model, "A1"));
        Assert.Equal(string.Empty, Sheet.Display(model, "A2"));
        Assert.Equal("second", Sheet.Display(model, "A3"));
        Assert.Equal(2, model.CellCount);
    }

    [Fact]
    public void Inserting_a_row_rewrites_references_below_it()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "1");
        Sheet.Set(model, "A5", "=A1");
        Sheet.Set(model, "B1", "=A5");

        Assert.True(model.InsertRows(2, 1));

        // The moved formula is unchanged, but the reference to it followed.
        Assert.Equal("=A1", model.GetRawInput(5, 0));
        Assert.Equal("=A6", model.GetRawInput(0, 1));
        Assert.Equal("1", Sheet.Display(model, "B1"));
    }

    [Fact]
    public void Inserting_rows_above_everything_shifts_the_whole_sheet()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "top");
        Sheet.Set(model, "B1", "=A1");

        Assert.True(model.InsertRows(0, 2));

        Assert.Equal("top", Sheet.Display(model, "A3"));
        Assert.Equal("=A3", model.GetRawInput(2, 1));
    }

    [Fact]
    public void Deleting_rows_pulls_cells_up()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "keep");
        Sheet.Set(model, "A2", "drop");
        Sheet.Set(model, "A3", "also keep");

        Assert.True(model.DeleteRows(1, 1));

        Assert.Equal("keep", Sheet.Display(model, "A1"));
        Assert.Equal("also keep", Sheet.Display(model, "A2"));
        Assert.Equal(2, model.CellCount);
    }

    [Fact]
    public void Deleting_a_referenced_row_produces_a_reference_error()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A2", "5");
        Sheet.Set(model, "B1", "=A2");

        Assert.True(model.DeleteRows(1, 1));

        Assert.Equal("=#REF!", model.GetRawInput(0, 1));
        Assert.Equal("#REF!", Sheet.Display(model, "B1"));
    }

    [Fact]
    public void Inserting_columns_shifts_cells_right_and_rewrites_references()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "1");
        Sheet.Set(model, "B1", "=A1");
        Sheet.Set(model, "C1", "=B1");

        Assert.True(model.InsertColumns(1, 1));

        Assert.Equal("1", Sheet.Display(model, "A1"));
        Assert.Equal("=A1", model.GetRawInput(0, 2));
        Assert.Equal("=C1", model.GetRawInput(0, 3));
        Assert.Equal("1", Sheet.Display(model, "D1"));
    }

    [Fact]
    public void Deleting_columns_pulls_cells_left()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "keep");
        Sheet.Set(model, "B1", "drop");
        Sheet.Set(model, "C1", "also keep");

        Assert.True(model.DeleteColumns(1, 1));

        Assert.Equal("keep", Sheet.Display(model, "A1"));
        Assert.Equal("also keep", Sheet.Display(model, "B1"));
        Assert.Equal(2, model.CellCount);
    }

    [Fact]
    public void A_structural_edit_is_one_undo_step()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "first");
        Sheet.Set(model, "A2", "second");
        model.Undo.Clear();

        Assert.True(model.InsertRows(0, 1));
        Assert.Equal(1, model.Undo.UndoDepth);

        model.Undo.Undo();

        Assert.Equal("first", Sheet.Display(model, "A1"));
        Assert.Equal("second", Sheet.Display(model, "A2"));
        Assert.Equal(2, model.CellCount);
    }

    [Fact]
    public void Undoing_a_structural_edit_restores_rewritten_references()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "1");
        Sheet.Set(model, "A5", "=A1");
        Sheet.Set(model, "B1", "=A5");
        model.Undo.Clear();

        Assert.True(model.InsertRows(2, 1));
        Assert.Equal("=A6", model.GetRawInput(0, 1));

        model.Undo.Undo();

        Assert.Equal("=A5", model.GetRawInput(0, 1));
        Assert.Equal("=A1", model.GetRawInput(4, 0));
        Assert.Equal("1", Sheet.Display(model, "B1"));
    }

    [Fact]
    public void Undoing_a_deletion_restores_the_deleted_cells()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "keep");
        Sheet.Set(model, "A2", "gone");
        Sheet.Set(model, "A3", "last");
        model.Undo.Clear();

        Assert.True(model.DeleteRows(1, 1));
        Assert.Equal(2, model.CellCount);

        model.Undo.Undo();

        Assert.Equal(3, model.CellCount);
        Assert.Equal("gone", Sheet.Display(model, "A2"));
        Assert.Equal("last", Sheet.Display(model, "A3"));
    }

    [Fact]
    public void Inserting_rows_refuses_to_push_data_off_the_sheet()
    {
        var model = Sheet.CreateModel();
        model.SetRawInput(SheetBounds.MaxRows - 1, 0, "last");
        model.Undo.Clear();

        Assert.False(model.InsertRows(0, 1));

        Assert.Equal("last", model.GetDisplayValue(SheetBounds.MaxRows - 1, 0));
        Assert.False(model.Undo.CanUndo);
    }

    [Fact]
    public void Inserting_into_an_empty_sheet_is_harmless()
    {
        var model = Sheet.CreateModel();

        Assert.True(model.InsertRows(0, 5));
        Assert.True(model.DeleteColumns(0, 5));

        Assert.Equal(0, model.CellCount);
        Assert.False(model.Undo.CanUndo);
    }

    [Fact]
    public void A_count_of_zero_changes_nothing()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "value");
        model.Undo.Clear();

        Assert.True(model.InsertRows(0, 0));
        Assert.True(model.DeleteRows(0, 0));

        Assert.Equal("value", Sheet.Display(model, "A1"));
        Assert.False(model.Undo.CanUndo);
    }

    [Fact]
    public void Inserting_rows_shifts_anchored_and_unanchored_references_alike()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A4", "4");
        Sheet.Set(model, "B1", "=A4+$A$4");

        Assert.True(model.InsertRows(0, 1));

        Assert.Equal("=A5+$A$5", model.GetRawInput(1, 1));
        Assert.Equal("8", Sheet.Display(model, "B2"));
    }
}
