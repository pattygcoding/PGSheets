namespace PGSheets.Core;

/// <summary>
/// Records cell edits so they can be replayed backwards and forwards.
/// </summary>
/// <remarks>
/// Edits are grouped: everything between <see cref="BeginGroup"/> and <see cref="EndGroup"/> becomes
/// one undo step, which is what makes clearing or pasting a whole range undo in a single action.
/// Inside a group each cell is recorded once, keeping its first "before" and its last "after".
/// </remarks>
public sealed class UndoStack
{
    private const int MaxSteps = 200;

    private readonly record struct Change(
        int Row,
        int Column,
        string Before,
        string After,
        string? BeforeFormat,
        string? AfterFormat);

    private readonly Action<int, int, string, string?> _apply;
    private readonly List<List<Change>> _undo = new();
    private readonly List<List<Change>> _redo = new();
    private readonly Dictionary<(int Row, int Column), int> _groupIndex = new();

    private List<Change>? _group;
    private int _groupDepth;
    private int _suppressDepth;

    /// <summary>
    /// <paramref name="apply"/> writes a raw value and its number format <em>without</em> recording
    /// them, so undo and redo can replay a change without appending a new one.
    /// </summary>
    public UndoStack(Action<int, int, string, string?> apply) =>
        _apply = apply ?? throw new ArgumentNullException(nameof(apply));

    /// <summary>Raised whenever the availability of undo or redo changes.</summary>
    public event EventHandler? Changed;

    public bool CanUndo => _undo.Count > 0;

    public bool CanRedo => _redo.Count > 0;

    /// <summary>Number of undo steps held.</summary>
    public int UndoDepth => _undo.Count;

    /// <summary>Opens a group; nested calls share the outermost one.</summary>
    public void BeginGroup() => _groupDepth++;

    /// <summary>Closes a group, committing it as one step. A group with no net change is dropped.</summary>
    public void EndGroup()
    {
        if (_groupDepth == 0)
        {
            return;
        }

        _groupDepth--;

        if (_groupDepth > 0)
        {
            return;
        }

        var group = _group;
        _group = null;
        _groupIndex.Clear();

        if (group is not null)
        {
            Commit(group);
        }
    }

    /// <summary>
    /// Notes one cell change: its raw input and its number format. No-ops when neither actually
    /// changed, so applying the same format twice is not an undo step.
    /// </summary>
    public void Record(
        int row,
        int column,
        string before,
        string after,
        string? beforeFormat,
        string? afterFormat)
    {
        var rawUnchanged = string.Equals(before, after, StringComparison.Ordinal);
        var formatUnchanged = string.Equals(beforeFormat, afterFormat, StringComparison.Ordinal);

        if (_suppressDepth > 0 || (rawUnchanged && formatUnchanged))
        {
            return;
        }

        if (_groupDepth == 0)
        {
            Commit(new List<Change> { new(row, column, before, after, beforeFormat, afterFormat) });
            return;
        }

        _group ??= new List<Change>();

        if (_groupIndex.TryGetValue((row, column), out var index))
        {
            _group[index] = _group[index] with { After = after, AfterFormat = afterFormat };
            return;
        }

        _groupIndex[(row, column)] = _group.Count;
        _group.Add(new Change(row, column, before, after, beforeFormat, afterFormat));
    }

    /// <summary>Replays the most recent step backwards. False when there is nothing to undo.</summary>
    public bool Undo()
    {
        if (_undo.Count == 0)
        {
            return false;
        }

        var step = _undo[^1];
        _undo.RemoveAt(_undo.Count - 1);

        // Reverse order, so a cell touched twice inside one step ends on its earliest value.
        for (var index = step.Count - 1; index >= 0; index--)
        {
            _apply(step[index].Row, step[index].Column, step[index].Before, step[index].BeforeFormat);
        }

        _redo.Add(step);
        Changed?.Invoke(this, EventArgs.Empty);
        return true;
    }

    /// <summary>Replays the most recently undone step. False when there is nothing to redo.</summary>
    public bool Redo()
    {
        if (_redo.Count == 0)
        {
            return false;
        }

        var step = _redo[^1];
        _redo.RemoveAt(_redo.Count - 1);

        foreach (var change in step)
        {
            _apply(change.Row, change.Column, change.After, change.AfterFormat);
        }

        _undo.Add(step);
        Changed?.Invoke(this, EventArgs.Empty);
        return true;
    }

    /// <summary>Drops all history, e.g. when the document is replaced wholesale.</summary>
    public void Clear()
    {
        var hadHistory = _undo.Count > 0 || _redo.Count > 0;

        _undo.Clear();
        _redo.Clear();
        _group = null;
        _groupIndex.Clear();

        if (hadHistory)
        {
            Changed?.Invoke(this, EventArgs.Empty);
        }
    }

    /// <summary>
    /// Suspends recording until the returned scope is disposed. Used by work that replaces the
    /// document rather than editing it, where there is nothing meaningful to undo back to.
    /// </summary>
    public IDisposable Suspend()
    {
        _suppressDepth++;
        return new Scope(this);
    }

    private void Commit(List<Change> step)
    {
        _undo.Add(step);
        _redo.Clear();

        while (_undo.Count > MaxSteps)
        {
            _undo.RemoveAt(0);
        }

        Changed?.Invoke(this, EventArgs.Empty);
    }

    private sealed class Scope : IDisposable
    {
        private readonly UndoStack _owner;

        public Scope(UndoStack owner) => _owner = owner;

        public void Dispose() => _owner._suppressDepth--;
    }
}
