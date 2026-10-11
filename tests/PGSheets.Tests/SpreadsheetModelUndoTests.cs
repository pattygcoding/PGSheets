using PGSheets.Core;

namespace PGSheets.Tests;

/// <summary>Undo/redo as wired into the model: editing, grouping, recalculating and suspension.</summary>
public class SpreadsheetModelUndoTests
{
    [Fact]
    public void An_edit_is_undoable()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "value");

        Assert.True(model.Undo.CanUndo);

        model.Undo.Undo();

        Assert.Equal(string.Empty, Sheet.Display(model, "A1"));
        Assert.Equal(0, model.CellCount);
    }

    [Fact]
    public void Undo_restores_the_previous_value_and_redo_reapplies_it()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "first");
        Sheet.Set(model, "A1", "second");

        model.Undo.Undo();
        Assert.Equal("first", Sheet.Display(model, "A1"));

        model.Undo.Redo();
        Assert.Equal("second", Sheet.Display(model, "A1"));
    }

    [Fact]
    public void Undo_recalculates_dependants()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "2");
        Sheet.Set(model, "B1", "=A1*10");

        Assert.Equal("20", Sheet.Display(model, "B1"));

        Sheet.Set(model, "A1", "3");
        Assert.Equal("30", Sheet.Display(model, "B1"));

        model.Undo.Undo();

        Assert.Equal("2", Sheet.Display(model, "A1"));
        Assert.Equal("20", Sheet.Display(model, "B1"));
    }

    [Fact]
    public void Undo_of_a_deleted_formula_restores_its_result()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "2");
        Sheet.Set(model, "B1", "=A1+1");

        Sheet.Set(model, "B1", string.Empty);
        Assert.Equal(string.Empty, Sheet.Display(model, "B1"));

        model.Undo.Undo();

        Assert.Equal("=A1+1", model.GetRawInput(0, 1));
        Assert.Equal("3", Sheet.Display(model, "B1"));
    }

    [Fact]
    public void Setting_the_same_value_records_nothing()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "value");

        var depth = model.Undo.UndoDepth;
        Sheet.Set(model, "A1", "value");

        Assert.Equal(depth, model.Undo.UndoDepth);
    }

    [Fact]
    public void Emptying_an_absent_cell_records_nothing()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", string.Empty);

        Assert.False(model.Undo.CanUndo);
    }

    [Fact]
    public void A_bulk_update_is_one_undo_step()
    {
        var model = Sheet.CreateModel();

        model.BeginBulkUpdate();

        for (var row = 0; row < 5; row++)
        {
            model.SetRawInput(row, 0, "value " + row.ToString(System.Globalization.CultureInfo.InvariantCulture));
        }

        model.EndBulkUpdate();

        Assert.Equal(1, model.Undo.UndoDepth);

        model.Undo.Undo();

        Assert.Equal(0, model.CellCount);
    }

    [Fact]
    public void Clear_drops_the_undo_history()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "value");

        model.Clear();

        Assert.False(model.Undo.CanUndo);
    }

    [Fact]
    public void Suspend_keeps_a_wholesale_replacement_out_of_the_history()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "before");

        using (model.SuspendUndoHistory())
        {
            model.BeginBulkUpdate();
            model.Clear();
            Sheet.Set(model, "A1", "loaded");
            model.EndBulkUpdate();
        }

        Assert.Equal("loaded", Sheet.Display(model, "A1"));
        Assert.False(model.Undo.CanUndo);
    }

    [Fact]
    public void Editing_again_after_a_suspension_is_recorded()
    {
        var model = Sheet.CreateModel();

        using (model.SuspendUndoHistory())
        {
            Sheet.Set(model, "A1", "loaded");
        }

        Sheet.Set(model, "A2", "edited");

        Assert.True(model.Undo.CanUndo);

        model.Undo.Undo();

        Assert.Equal(string.Empty, Sheet.Display(model, "A2"));
        Assert.Equal("loaded", Sheet.Display(model, "A1"));
    }
}
