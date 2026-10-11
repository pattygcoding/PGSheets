using PGSheets.Formulas;

namespace PGSheets.Core;

/// <summary>
/// Sink used by importers so they can push data into the model without depending on the UI
/// or on the model's concrete type.
/// </summary>
public interface IImportSink
{
    void SetRawInput(int row, int column, string rawInput);
}

/// <summary>
/// The sparse workbook. Cells are stored only when they have content, so an "empty" grid costs
/// nothing. Raw input and computed output are kept side by side: editing writes <see cref="SetRawInput"/>
/// and the formula engine writes back through <see cref="SetComputed"/>.
/// </summary>
public sealed class SpreadsheetModel : ICellStore, IImportSink
{
    private readonly Dictionary<(int Row, int Column), CellData> _cells = new();
    private readonly IFormulaEngine _engine;

    private int _bulkDepth;
    private bool _pendingRebuild;

    public SpreadsheetModel(IFormulaEngine engine)
    {
        _engine = engine ?? throw new ArgumentNullException(nameof(engine));
        _engine.Attach(this);
    }

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

    public IEnumerable<(int Row, int Column)> PopulatedCells => _cells.Keys;

    /// <summary>
    /// Stores raw input for a cell and recalculates. An empty string removes the cell entirely,
    /// preserving the sparse representation.
    /// </summary>
    public void SetRawInput(int row, int column, string rawInput)
    {
        rawInput ??= string.Empty;

        var key = (row, column);
        var address = new CellAddress(row, column);

        if (rawInput.Length == 0)
        {
            var existed = _cells.Remove(key);
            _engine.OnCellRemoved(address);

            if (!existed)
            {
                return;
            }

            RecalculateOrDefer(address);
            RaiseChanged();
            return;
        }

        if (_cells.TryGetValue(key, out var cell))
        {
            if (string.Equals(cell.RawInput, rawInput, StringComparison.Ordinal))
            {
                return;
            }

            cell.RawInput = rawInput;
        }
        else
        {
            _cells[key] = new CellData(rawInput);
        }

        RecalculateOrDefer(address);
        RaiseChanged();
    }

    /// <summary>
    /// Suppresses per-cell recalculation and change notifications. Used when importing many rows
    /// so the engine runs a single <see cref="IFormulaEngine.Rebuild"/> at the end instead of once
    /// per cell.
    /// </summary>
    public void BeginBulkUpdate() => _bulkDepth++;

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

        RaiseChanged();
    }

    /// <summary>Removes every cell.</summary>
    public void Clear()
    {
        if (_cells.Count == 0)
        {
            return;
        }

        _cells.Clear();
        _engine.Reset();
        RaiseChanged();
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
        cell.DisplayValue = value.ToDisplayString();
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
