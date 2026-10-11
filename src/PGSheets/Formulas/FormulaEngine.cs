using PGSheets.Core;

namespace PGSheets.Formulas;

/// <summary>
/// Default formula engine. Evaluation is a memoised depth-first walk: the changed cell is
/// evaluated, which recursively evaluates whatever it references, while a set of "in progress"
/// cells detects circular references. Results are written back to the store as they are produced,
/// and the dependency graph records the edges discovered along the way.
/// </summary>
public sealed class FormulaEngine : IFormulaEngine
{
    private readonly DependencyGraph _graph = new();

    // Parsed formulas are cached per cell: parsing only ever happens after an edit, never per frame.
    private readonly Dictionary<CellAddress, IFormulaNode?> _parseCache = new();

    private ICellStore? _store;

    public void Attach(ICellStore store) => _store = store ?? throw new ArgumentNullException(nameof(store));

    public void Reset()
    {
        _graph.Clear();
        _parseCache.Clear();
    }

    public void OnCellRemoved(CellAddress cell)
    {
        _graph.RemoveCell(cell);
        _parseCache.Remove(cell);
    }

    public void Rebuild()
    {
        if (_store is null)
        {
            return;
        }

        _graph.Clear();
        _parseCache.Clear();

        var context = new EvalContext(this);
        var cells = new List<CellAddress>();

        foreach (var (row, column) in _store.PopulatedCells)
        {
            cells.Add(new CellAddress(row, column));
        }

        foreach (var cell in cells)
        {
            Evaluate(cell, context);
        }
    }

    public void Recalculate(CellAddress changed)
    {
        if (_store is null)
        {
            return;
        }

        // The edited cell's formula text changed, so its cached parse tree is stale.
        _parseCache.Remove(changed);

        var affected = new HashSet<CellAddress>();
        _graph.CollectAffected(changed, affected);

        var context = new EvalContext(this);

        foreach (var cell in affected)
        {
            Evaluate(cell, context);
        }
    }

    public IReadOnlyCollection<CellAddress> GetPrecedents(CellAddress cell) =>
        _graph.TryGetPrecedents(cell, out var precedents) ? precedents : Array.Empty<CellAddress>();

    private CellValue Evaluate(CellAddress cell, EvalContext context)
    {
        if (context.Done.Contains(cell))
        {
            return _store!.GetValue(cell.Row, cell.Column);
        }

        if (!context.InProgress.Add(cell))
        {
            // Re-entering a cell that is still being evaluated means the formulas form a cycle.
            return CellValue.FromError(CellError.Circular);
        }

        var raw = _store!.GetRawInput(cell.Row, cell.Column);

        CellValue value;
        if (raw.Length == 0)
        {
            value = CellValue.Empty;
        }
        else if (raw[0] == '=')
        {
            value = EvaluateFormula(cell, raw, context);
        }
        else
        {
            value = Literals.Parse(raw);
        }

        context.InProgress.Remove(cell);
        context.Done.Add(cell);

        _store.SetComputed(cell.Row, cell.Column, value);
        return value;
    }

    private CellValue EvaluateFormula(CellAddress cell, string raw, EvalContext context)
    {
        if (!_parseCache.TryGetValue(cell, out var node))
        {
            FormulaParser.TryParse(raw, out node, out _);
            _parseCache[cell] = node;
        }

        if (node is null)
        {
            return CellValue.FromError(CellError.Value);
        }

        var precedents = new HashSet<CellAddress>();
        node.CollectPrecedents(precedents);
        _graph.SetPrecedents(cell, precedents);

        try
        {
            return node.Evaluate(context);
        }
        catch (FormulaException ex)
        {
            return CellValue.FromError(ex.Error);
        }
    }

    /// <summary>Per-run evaluation state. Allocated once per recalculation, not per cell.</summary>
    private sealed class EvalContext : IFormulaContext
    {
        private readonly FormulaEngine _engine;

        internal EvalContext(FormulaEngine engine)
        {
            _engine = engine;
            InProgress = new HashSet<CellAddress>();
            Done = new HashSet<CellAddress>();
        }

        internal HashSet<CellAddress> InProgress { get; }

        internal HashSet<CellAddress> Done { get; }

        public CellValue GetCellValue(int row, int column) =>
            _engine.Evaluate(new CellAddress(row, column), this);
    }
}
