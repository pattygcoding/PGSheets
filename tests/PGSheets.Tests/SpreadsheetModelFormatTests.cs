using PGSheets.Core;

namespace PGSheets.Tests;

/// <summary>
/// Number formats through the model: what the grid shows, what survives an edit, and that a format
/// is one undo step like any other change.
/// </summary>
public class SpreadsheetModelFormatTests
{
    [Fact]
    public void A_format_changes_what_the_grid_shows()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "0.155");

        Assert.Equal("0.155", Sheet.Display(model, "A1"));

        model.SetNumberFormat(0, 0, "0.0%");

        Assert.Equal("15.5%", Sheet.Display(model, "A1"));
    }

    [Fact]
    public void A_format_turns_a_serial_into_a_date()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "45356");
        model.SetNumberFormat(0, 0, "yyyy-mm-dd");

        Assert.Equal("2024-03-05", Sheet.Display(model, "A1"));

        // The value stays a number, so arithmetic on a date still behaves like a spreadsheet.
        Sheet.Set(model, "A2", "=A1+1");
        model.SetNumberFormat(1, 0, "yyyy-mm-dd");

        Assert.Equal("2024-03-06", Sheet.Display(model, "A2"));
    }

    [Fact]
    public void Typing_a_new_value_keeps_the_format()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "1");
        model.SetNumberFormat(0, 0, "0.00");

        Sheet.Set(model, "A1", "2");

        Assert.Equal("2.00", Sheet.Display(model, "A1"));
        Assert.Equal("0.00", model.GetNumberFormat(0, 0));
    }

    [Fact]
    public void Text_is_not_formatted()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "North");

        model.SetNumberFormat(0, 0, "0.00");

        Assert.Equal("North", Sheet.Display(model, "A1"));
    }

    [Fact]
    public void A_format_is_one_undo_step_and_undo_leaves_the_value_alone()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "1");
        model.Undo.Clear();

        model.SetNumberFormat(0, 0, "0.00");

        Assert.Equal("1.00", Sheet.Display(model, "A1"));
        Assert.Equal(1, model.Undo.UndoDepth);

        model.Undo.Undo();

        Assert.Equal("1", Sheet.Display(model, "A1"));
        Assert.Null(model.GetNumberFormat(0, 0));

        model.Undo.Redo();

        Assert.Equal("1.00", Sheet.Display(model, "A1"));
    }

    [Fact]
    public void A_format_can_be_cleared()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "1");
        model.SetNumberFormat(0, 0, "0.00");

        model.SetNumberFormat(0, 0, null);

        Assert.Equal("1", Sheet.Display(model, "A1"));
        Assert.Null(model.GetNumberFormat(0, 0));
    }

    [Fact]
    public void Applying_the_same_format_twice_records_nothing()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "1");
        model.SetNumberFormat(0, 0, "0.00");
        model.Undo.Clear();

        model.SetNumberFormat(0, 0, "0.00");

        Assert.False(model.Undo.CanUndo);
    }

    [Fact]
    public void Clearing_the_value_leaves_the_format_behind_for_the_next_one()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "1");
        model.SetNumberFormat(0, 0, "0.00");

        Sheet.Set(model, "A1", string.Empty);

        // The format is still remembered, which is what formats the value typed next.
        Assert.Equal(1, model.CellCount);
        Assert.Equal(string.Empty, Sheet.Display(model, "A1"));
        Assert.Equal("0.00", model.GetNumberFormat(0, 0));

        Sheet.Set(model, "A1", "5");

        Assert.Equal("5.00", Sheet.Display(model, "A1"));
    }

    [Fact]
    public void Formatting_an_empty_cell_remembers_the_format_for_later()
    {
        var model = Sheet.CreateModel();

        model.SetNumberFormat(2, 3, "0.0%");

        Assert.Equal("0.0%", model.GetNumberFormat(2, 3));
        Assert.Equal(string.Empty, model.GetDisplayValue(2, 3));

        model.SetRawInput(2, 3, "0.25");

        Assert.Equal("25.0%", model.GetDisplayValue(2, 3));
    }

    [Fact]
    public void Clearing_a_format_from_an_empty_cell_removes_it_entirely()
    {
        var model = Sheet.CreateModel();
        model.SetNumberFormat(0, 0, "0.00");
        model.Undo.Clear();

        model.SetNumberFormat(0, 0, null);

        Assert.Equal(0, model.CellCount);

        // Clearing a format is a change like any other, so it can be undone.
        model.Undo.Undo();
        Assert.Equal("0.00", model.GetNumberFormat(0, 0));
    }

    [Fact]
    public void A_format_travels_with_its_cell_through_a_structural_edit()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A2", "0.155");
        model.SetNumberFormat(1, 0, "0.0%");

        Assert.True(model.InsertRows(0, 1));

        Assert.Equal("15.5%", Sheet.Display(model, "A3"));
        Assert.Equal("0.0%", model.GetNumberFormat(2, 0));
        Assert.Null(model.GetNumberFormat(1, 0));
    }

    [Fact]
    public void A_format_survives_undoing_a_structural_edit()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A2", "0.155");
        model.SetNumberFormat(1, 0, "0.0%");
        model.Undo.Clear();

        Assert.True(model.InsertRows(0, 1));
        model.Undo.Undo();

        Assert.Equal("15.5%", Sheet.Display(model, "A2"));
        Assert.Equal("0.0%", model.GetNumberFormat(1, 0));
    }

    [Fact]
    public void Deleting_a_formatted_row_takes_the_format_with_it()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "1");
        model.SetNumberFormat(0, 0, "0.00");
        Sheet.Set(model, "A2", "2");
        model.SetNumberFormat(1, 0, "0.0%");

        Assert.True(model.DeleteRows(0, 1));

        // The deleted row is gone, format and all; the row below moved up carrying its own format.
        Assert.Equal(1, model.CellCount);
        Assert.Equal("0.0%", model.GetNumberFormat(0, 0));
        Assert.Equal("200.0%", Sheet.Display(model, "A1"));
    }
}
