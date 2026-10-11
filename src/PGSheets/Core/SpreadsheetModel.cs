using PGSheets.Formulas;

namespace PGSheets.Core;

/// <summary>
/// Sink used by importers so they can push data into the model without depending on the UI
/// or on the model's concrete type.
/// </summary>
public interface IImportSink
{
    void SetRawInput(int row, int column, string rawInput);

    /// <summary>
    /// Stores the number format that applies to a cell, as read from the file alongside its value.
    /// A separate call because the format arrives with the style rather than with the content, and
    /// most sinks have no use for it.
    /// </summary>
    void SetNumberFormat(int row, int column, string? numberFormat);
}

/// <summary>
/// The sparse workbook. Cells are stored only when they have content, so an "empty" grid costs
/// nothing. Raw input and computed output are kept side by side: editing writes <see cref="SetRawInput"/>
/// and the formula engine writes back through <see cref="SetComputed"/>.
/// </summary>
public sealed class SpreadsheetModel : ICellStore, IImportSink, IExportSource
{
    private readonly Dictionary<(int Row, int Column), CellData> _cells = new();
    private readonly IFormulaEngine _engine;
    private readonly UndoStack _undo;

    private int _bulkDepth;
    private bool _pendingRebuild;

    public SpreadsheetModel(IFormulaEngine engine)
    {
        _engine = engine ?? throw new ArgumentNullException(nameof(engine));
        _engine.Attach(this);

        // The stack replays changes through ApplyRawInput, which deliberately does not re-record.
        _undo = new UndoStack(ApplyRawInput);
    }

    /// <summary>Undo/redo history for edits made through <see cref="SetRawInput"/>.</summary>
    public UndoStack Undo => _undo;

    /// <summary>
    /// Suspends undo recording until the scope is disposed. Used when the document is replaced
    /// wholesale (open, new), where there is nothing meaningful to undo back to.
    /// </summary>
    public IDisposable SuspendUndoHistory() => _undo.Suspend();

    /// <summary>Raised whenever content or computed values change. The grid re-renders on this.</summary>
    public event EventHandler? Changed;

    /// <summary>Number of populated cells (not the grid extent).</summary>
    public int CellCount => _cells.Count;

    public bool TryGetCell(int row, int column, out CellData data) =>
        _cells.TryGetValue((row, column), out data!);

    public CellData? GetCell(int row, int column) =>
        _cells.TryGetValue((row, column), out var data) ? data : null;

    /// <summary>Text to render for a cell. Allocation free when the cell is absent.</summary>
    public string GetDisplayValue(int row, int column) =>
        _cells.TryGetValue((row, column), out var data) ? data.DisplayValue : string.Empty;

    public string GetRawInput(int row, int column) =>
        _cells.TryGetValue((row, column), out var data) ? data.RawInput : string.Empty;

    public CellValue GetValue(int row, int column) =>
        _cells.TryGetValue((row, column), out var data) ? data.Value : CellValue.Empty;

    /// <summary>
    /// The number format code of a cell, or null when it is General. A cell can hold a format with
    /// no value: formatting reaches an empty cell just as it does in a spreadsheet.
    /// </summary>
    public string? GetNumberFormat(int row, int column) =>
        _cells.TryGetValue((row, column), out var data) ? data.NumberFormat : null;

    public IEnumerable<(int Row, int Column)> PopulatedCells => _cells.Keys;

    /// <summary>
    /// Stores raw input for a cell, keeping the number format it already has, recalculates and
    /// records an undo step. An empty string removes the cell, unless a format is attached to it.
    /// </summary>
    public void SetRawInput(int row, int column, string rawInput)
    {
        rawInput ??= string.Empty;

        var cell = GetCell(row, column);
        var previous = cell?.RawInput ?? string.Empty;
        var format = cell?.NumberFormat;

        if (string.Equals(previous, rawInput, StringComparison.Ordinal))
        {
            return;
        }

        // Typing a value does not change the way that value is displayed.
        ApplyRawInput(row, column, rawInput, format);
        _undo.Record(row, column, previous, rawInput, format, format);
    }

    /// <summary>
    /// Sets the number format of a cell and records one undo step. A blank format means General;
    /// clearing the format of a cell that holds no value removes the cell entirely.
    /// </summary>
    public void SetNumberFormat(int row, int column, string? numberFormat)
    {
        var format = string.IsNullOrWhiteSpace(numberFormat) ? null : numberFormat;
        var cell = GetCell(row, column);

        if (cell is null)
        {
            if (format is null)
            {
                return;
            }

            // A format on its own is still worth keeping: it is what a value typed later shows as.
            ApplyRawInput(row, column, string.Empty, format);
            _undo.Record(row, column, string.Empty, string.Empty, null, format);
            return;
        }

        if (string.Equals(cell.NumberFormat, format, StringComparison.Ordinal))
        {
            return;
        }

        // The previous state has to be captured before the mutation: ApplyRawInput writes through
        // the very CellData this reads, so reading it afterwards would record "no change".
        var raw = cell.RawInput;
        var previousFormat = cell.NumberFormat;

        ApplyRawInput(row, column, raw, format);
        _undo.Record(row, column, raw, raw, previousFormat, format);
    }

    /// <summary>
    /// Applies a raw value and its number format without touching the undo history. This is the
    /// single mutation point, shared by the setters above and by undo/redo replay.
    /// </summary>
    private void ApplyRawInput(int row, int column, string rawInput, string? numberFormat)
    {
        var key = (row, column);
        var address = new CellAddress(row, column);
        var format = string.IsNullOrEmpty(numberFormat) ? null : numberFormat;

        if (rawInput.Length == 0 && format is null)
        {
            // Nothing to show and nothing to remember: the cell goes, keeping the sheet sparse.
            _cells.Remove(key);
            _engine.OnCellRemoved(address);
        }
        else if (_cells.TryGetValue(key, out var cell))
        {
            cell.RawInput = rawInput;
            cell.NumberFormat = format;
        }
        else
        {
            _cells[key] = new CellData(rawInput) { NumberFormat = format };
        }

        RecalculateOrDefer(address);
        RaiseChanged();
    }

    /// <summary>
    /// Suppresses per-cell recalculation and change notifications. Used when importing many rows
    /// so the engine runs a single <see cref="IFormulaEngine.Rebuild"/> at the end instead of once
    /// per cell.
    /// </summary>
    public void BeginBulkUpdate()
    {
        if (_bulkDepth == 0)
        {
            // One group, so the whole bulk edit is a single undo step.
            _undo.BeginGroup();
        }

        _bulkDepth++;
    }

    public void EndBulkUpdate()
    {
        if (_bulkDepth == 0)
        {
            return;
        }

        _bulkDepth--;

        if (_bulkDepth > 0)
        {
            return;
        }

        if (_pendingRebuild)
        {
            _pendingRebuild = false;
            _engine.Rebuild();
        }

        _undo.EndGroup();
        RaiseChanged();
    }

    /// <summary>Removes every cell and the undo history that referred to them.</summary>
    public void Clear()
    {
        if (_cells.Count == 0)
        {
            _undo.Clear();
            return;
        }

        _cells.Clear();
        _engine.Reset();
        _undo.Clear();
        RaiseChanged();
    }

    // ---- Structural edits ---------------------------------------------------------------------

    /// <summary>
    /// Inserts blank rows at <paramref name="at"/>, pushing existing cells down and adjusting every
    /// formula. Returns false, changing nothing, when that would push data off the sheet.
    /// </summary>
    public bool InsertRows(int at, int count) => InsertLines(at, count, rows: true);

    /// <summary>Removes rows, pulling later cells up and rewriting references that pointed at them.</summary>
    public bool DeleteRows(int at, int count) => DeleteLines(at, count, rows: true);

    /// <summary>Inserts blank columns at <paramref name="at"/>.</summary>
    public bool InsertColumns(int at, int count) => InsertLines(at, count, rows: false);

    /// <summary>Removes columns, rewriting references that pointed at them.</summary>
    public bool DeleteColumns(int at, int count) => DeleteLines(at, count, rows: false);

    /// <summary>One cell as a structural edit sees it: its raw input and the format that goes with it.</summary>
    private readonly record struct CellContent(string Raw, string? Format);

    private bool InsertLines(int at, int count, bool rows)
    {
        if (count <= 0)
        {
            return true;
        }

        var limit = rows ? SheetBounds.MaxRows : SheetBounds.MaxColumns;
        var lastIndex = limit - 1;

        at = Math.Clamp(at, 0, lastIndex);
        count = Math.Min(count, limit - at);

        foreach (var (row, column) in _cells.Keys)
        {
            var position = rows ? row : column;

            if (position >= at && position + count > lastIndex)
            {
                // Refuse rather than silently push data off the edge of the sheet.
                return false;
            }
        }

        var replacement = new Dictionary<(int Row, int Column), CellContent>();

        foreach (var ((row, column), cell) in _cells)
        {
            var position = rows ? row : column;
            var moved = position >= at ? position + count : position;
            var raw = RewriteForLineChange(cell.RawInput, at, count, inserted: true, rows);

            replacement[rows ? (moved, column) : (row, moved)] = new CellContent(raw, cell.NumberFormat);
        }

        ReplaceContents(replacement);
        return true;
    }

    private bool DeleteLines(int at, int count, bool rows)
    {
        if (count <= 0)
        {
            return true;
        }

        var limit = rows ? SheetBounds.MaxRows : SheetBounds.MaxColumns;

        at = Math.Clamp(at, 0, limit - 1);
        count = Math.Min(count, limit - at);

        var replacement = new Dictionary<(int Row, int Column), CellContent>();

        foreach (var ((row, column), cell) in _cells)
        {
            var position = rows ? row : column;

            // Cells inside the removed band are gone; everything after it moves up.
            if (position >= at && position < at + count)
            {
                continue;
            }

            var moved = position >= at + count ? position - count : position;
            var raw = RewriteForLineChange(cell.RawInput, at, count, inserted: false, rows);

            replacement[rows ? (moved, column) : (row, moved)] = new CellContent(raw, cell.NumberFormat);
        }

        ReplaceContents(replacement);
        return true;
    }

    private static string RewriteForLineChange(string raw, int at, int count, bool inserted, bool rows)
    {
        if (raw.Length == 0 || raw[0] != '=')
        {
            return raw;
        }

        return rows
            ? FormulaRewriter.AdjustForRows(raw, at, count, inserted)
            : FormulaRewriter.AdjustForColumns(raw, at, count, inserted);
    }

    /// <summary>
    /// Replaces the whole sheet with a transformed copy as a single undo step. Cells that no longer
    /// exist are cleared and the rest written — value and format together; every key is visited once,
    /// so the undo stack records one coherent before/after per cell and an undo puts the sheet back
    /// exactly as it was.
    /// </summary>
    private void ReplaceContents(Dictionary<(int Row, int Column), CellContent> replacement)
    {
        BeginBulkUpdate();

        try
        {
            // Snapshot the keys first: SetRawInput adds and removes entries.
            foreach (var key in _cells.Keys.ToList())
            {
                if (replacement.ContainsKey(key))
                {
                    continue;
                }

                // Clearing the format as well as the value is what removes a formatted empty cell.
                SetNumberFormat(key.Row, key.Column, null);
                SetRawInput(key.Row, key.Column, string.Empty);
            }

            foreach (var (key, content) in replacement)
            {
                SetRawInput(key.Row, key.Column, content.Raw);
                SetNumberFormat(key.Row, key.Column, content.Format);
            }
        }
        finally
        {
            EndBulkUpdate();
        }
    }

    // ---- ICellStore -------------------------------------------------------------------

    CellValue ICellStore.GetValue(int row, int column) => GetValue(row, column);

    string ICellStore.GetRawInput(int row, int column) => GetRawInput(row, column);

    /// <summary>
    /// Called by the engine. Cells that do not exist are ignored so that removing a cell is not
    /// undone by the recalculation that follows.
    /// </summary>
    public void SetComputed(int row, int column, CellValue value)
    {
        if (!_cells.TryGetValue((row, column), out var cell))
        {
            return;
        }

        cell.Value = value;

        // The number format is what turns a serial into a date and a fraction into a percentage, so
        // it is applied here — while recalculating, never while the grid paints.
        cell.DisplayValue = value.Kind == CellValueKind.Number
            ? NumberFormat.Apply(value.Number, cell.NumberFormat)
            : value.ToDisplayString();
    }

    private void RecalculateOrDefer(CellAddress address)
    {
        if (_bulkDepth > 0)
        {
            _pendingRebuild = true;
            return;
        }

        _engine.Recalculate(address);
    }

    private void RaiseChanged()
    {
        if (_bulkDepth == 0)
        {
            Changed?.Invoke(this, EventArgs.Empty);
        }
    }
}
