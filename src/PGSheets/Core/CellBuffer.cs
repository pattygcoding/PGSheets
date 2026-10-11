namespace PGSheets.Core;

/// <summary>
/// Collects imported cells in memory, keeping the rectangular shape of the source. Used as the
/// target of a clipboard paste: a sink that discarded empty fields would lose the alignment, and a
/// pasted block has to line up with a selection.
/// </summary>
public sealed class CellBuffer : IImportSink
{
    private readonly Dictionary<(int Row, int Column), string> _cells = new();

    /// <summary>Every field of the block, including empty ones, in no particular order.</summary>
    public IEnumerable<(int Row, int Column, string Value)> Cells
    {
        get
        {
            foreach (var pair in _cells)
            {
                yield return (pair.Key.Row, pair.Key.Column, pair.Value);
            }
        }
    }

    /// <summary>Number of fields, including empty ones.</summary>
    public int Count => _cells.Count;

    /// <summary>Number of fields that hold content.</summary>
    public int NonEmptyCount
    {
        get
        {
            var count = 0;

            foreach (var value in _cells.Values)
            {
                if (value.Length > 0)
                {
                    count++;
                }
            }

            return count;
        }
    }

    /// <summary>Records a field, including empty ones so the block keeps its shape.</summary>
    public void SetRawInput(int row, int column, string rawInput) =>
        _cells[(row, column)] = rawInput ?? string.Empty;

    /// <summary>
    /// Ignored: a tab-separated clipboard carries values, not formats. Pasting text should not
    /// silently reformat the cells it lands on.
    /// </summary>
    public void SetNumberFormat(int row, int column, string? numberFormat)
    {
    }

    public string Get(int row, int column) =>
        _cells.TryGetValue((row, column), out var value) ? value : string.Empty;

    /// <summary>The smallest rectangle covering the block. False when the buffer is empty.</summary>
    public bool TryGetBounds(out CellRange range)
    {
        range = default;

        if (_cells.Count == 0)
        {
            return false;
        }

        var minRow = int.MaxValue;
        var minColumn = int.MaxValue;
        var maxRow = int.MinValue;
        var maxColumn = int.MinValue;

        foreach (var key in _cells.Keys)
        {
            minRow = Math.Min(minRow, key.Row);
            maxRow = Math.Max(maxRow, key.Row);
            minColumn = Math.Min(minColumn, key.Column);
            maxColumn = Math.Max(maxColumn, key.Column);
        }

        range = new CellRange(minRow, minColumn, maxRow, maxColumn);
        return true;
    }
}
