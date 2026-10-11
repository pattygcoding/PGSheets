using System.Globalization;
using PGSheets.Core;

namespace PGSheets.Tests;

public class UndoStackTests
{
    /// <summary>A stack over a plain dictionary, so these tests need no spreadsheet.</summary>
    private sealed class Target
    {
        private readonly Dictionary<(int Row, int Column), string> _cells = new();
        private readonly Dictionary<(int Row, int Column), string?> _formats = new();

        public Target() => Stack = new UndoStack(Set);

        public UndoStack Stack { get; }

        public string Get(int row, int column) =>
            _cells.TryGetValue((row, column), out var value) ? value : string.Empty;

        public string? GetFormat(int row, int column) =>
            _formats.TryGetValue((row, column), out var value) ? value : null;

        /// <summary>Simulates a user edit: apply the value, then record it.</summary>
        public void Edit(int row, int column, string value)
        {
            var before = Get(row, column);
            var format = GetFormat(row, column);

            Set(row, column, value, format);
            Stack.Record(row, column, before, value, format, format);
        }

        /// <summary>Simulates applying a number format, which leaves the value alone.</summary>
        public void Format(int row, int column, string? numberFormat)
        {
            var value = Get(row, column);
            var before = GetFormat(row, column);

            Set(row, column, value, numberFormat);
            Stack.Record(row, column, value, value, before, numberFormat);
        }

        private void Set(int row, int column, string value, string? numberFormat)
        {
            _cells[(row, column)] = value;
            _formats[(row, column)] = numberFormat;
        }
    }

    [Fact]
    public void Starts_empty()
    {
        var target = new Target();

        Assert.False(target.Stack.CanUndo);
        Assert.False(target.Stack.CanRedo);
        Assert.Equal(0, target.Stack.UndoDepth);
    }

    [Fact]
    public void Undo_restores_the_previous_value()
    {
        var target = new Target();
        target.Edit(0, 0, "first");
        target.Edit(0, 0, "second");

        Assert.True(target.Stack.Undo());
        Assert.Equal("first", target.Get(0, 0));

        Assert.True(target.Stack.Undo());
        Assert.Equal(string.Empty, target.Get(0, 0));

        Assert.False(target.Stack.Undo());
    }

    [Fact]
    public void Redo_reapplies_the_value()
    {
        var target = new Target();
        target.Edit(0, 0, "value");
        target.Stack.Undo();

        Assert.True(target.Stack.Redo());
        Assert.Equal("value", target.Get(0, 0));
        Assert.False(target.Stack.Redo());
    }

    [Fact]
    public void A_new_edit_discards_the_redo_history()
    {
        var target = new Target();
        target.Edit(0, 0, "first");
        target.Stack.Undo();

        Assert.True(target.Stack.CanRedo);

        target.Edit(0, 0, "second");

        Assert.False(target.Stack.CanRedo);
        Assert.Equal("second", target.Get(0, 0));
    }

    [Fact]
    public void A_group_undoes_as_one_step()
    {
        var target = new Target();

        target.Stack.BeginGroup();
        target.Edit(0, 0, "a");
        target.Edit(1, 0, "b");
        target.Edit(2, 0, "c");
        target.Stack.EndGroup();

        Assert.Equal(1, target.Stack.UndoDepth);

        target.Stack.Undo();

        Assert.Equal(string.Empty, target.Get(0, 0));
        Assert.Equal(string.Empty, target.Get(1, 0));
        Assert.Equal(string.Empty, target.Get(2, 0));
    }

    [Fact]
    public void A_group_records_each_cell_once()
    {
        var target = new Target();

        target.Stack.BeginGroup();
        target.Edit(0, 0, "a");
        target.Edit(0, 0, "b");
        target.Edit(0, 0, "c");
        target.Stack.EndGroup();

        target.Stack.Undo();

        // One step, and it returns to the value from before the group, not to "b".
        Assert.Equal(string.Empty, target.Get(0, 0));
        Assert.False(target.Stack.CanUndo);
    }

    [Fact]
    public void Repeating_a_cell_inside_a_group_replays_the_last_value()
    {
        var target = new Target();

        target.Stack.BeginGroup();
        target.Edit(0, 0, "a");
        target.Edit(0, 0, "b");
        target.Stack.EndGroup();

        target.Stack.Undo();
        Assert.Equal(string.Empty, target.Get(0, 0));

        target.Stack.Redo();
        Assert.Equal("b", target.Get(0, 0));
    }

    [Fact]
    public void Nested_groups_still_produce_one_step()
    {
        var target = new Target();

        target.Stack.BeginGroup();
        target.Stack.BeginGroup();
        target.Edit(0, 0, "a");
        target.Stack.EndGroup();
        target.Edit(1, 0, "b");
        target.Stack.EndGroup();

        Assert.Equal(1, target.Stack.UndoDepth);
    }

    [Fact]
    public void An_empty_group_is_not_a_step()
    {
        var target = new Target();

        target.Stack.BeginGroup();
        target.Stack.EndGroup();

        Assert.False(target.Stack.CanUndo);
    }

    [Fact]
    public void An_unbalanced_end_group_is_harmless()
    {
        var target = new Target();

        target.Stack.EndGroup();

        Assert.False(target.Stack.CanUndo);
    }

    [Fact]
    public void Identical_values_are_not_recorded()
    {
        var target = new Target();

        target.Stack.Record(0, 0, "same", "same", null, null);

        Assert.False(target.Stack.CanUndo);
    }

    [Fact]
    public void A_number_format_change_is_its_own_step()
    {
        var target = new Target();
        target.Edit(0, 0, "1234");

        target.Format(0, 0, "0.00");

        Assert.True(target.Stack.CanUndo);
        Assert.Equal("0.00", target.GetFormat(0, 0));

        // Undoing the format leaves the value untouched.
        target.Stack.Undo();

        Assert.Null(target.GetFormat(0, 0));
        Assert.Equal("1234", target.Get(0, 0));

        target.Stack.Redo();

        Assert.Equal("0.00", target.GetFormat(0, 0));
        Assert.Equal("1234", target.Get(0, 0));
    }

    [Fact]
    public void Applying_the_same_format_twice_is_not_a_step()
    {
        var target = new Target();
        target.Edit(0, 0, "1");
        target.Format(0, 0, "0.00");

        var depth = target.Stack.UndoDepth;

        target.Format(0, 0, "0.00");

        Assert.Equal(depth, target.Stack.UndoDepth);
    }

    [Fact]
    public void Suspend_stops_recording_until_the_scope_ends()
    {
        var target = new Target();

        using (target.Stack.Suspend())
        {
            target.Edit(0, 0, "ignored");
        }

        Assert.False(target.Stack.CanUndo);
        Assert.Equal("ignored", target.Get(0, 0));

        target.Edit(1, 0, "recorded");
        Assert.True(target.Stack.CanUndo);
    }

    [Fact]
    public void Clear_drops_the_history()
    {
        var target = new Target();
        target.Edit(0, 0, "value");

        target.Stack.Clear();

        Assert.False(target.Stack.CanUndo);
        Assert.False(target.Stack.CanRedo);
    }

    [Fact]
    public void Changed_is_raised_once_per_step()
    {
        var target = new Target();
        var raised = 0;
        target.Stack.Changed += (_, _) => raised++;

        target.Edit(0, 0, "value");
        Assert.Equal(1, raised);

        target.Stack.Undo();
        Assert.Equal(2, raised);
    }

    [Fact]
    public void The_history_is_bounded()
    {
        var target = new Target();

        for (var index = 0; index < 250; index++)
        {
            target.Edit(0, 0, index.ToString(CultureInfo.InvariantCulture));
        }

        Assert.Equal(200, target.Stack.UndoDepth);
    }
}
