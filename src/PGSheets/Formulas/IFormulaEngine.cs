using PGSheets.Core;

namespace PGSheets.Formulas;

/// <summary>
/// The formula calculation contract. Kept intentionally small so an alternative engine
/// (or a different evaluation strategy) can be dropped in behind the same surface.
/// </summary>
public interface IFormulaEngine
{
    /// <summary>Binds the engine to the store it should read from and write results to.</summary>
    void Attach(ICellStore store);

    /// <summary>
    /// Recomputation entry point for an edit: recalculates the changed cell and every cell that
    /// depends on it, in dependency order.
    /// </summary>
    void Recalculate(CellAddress changed);

    /// <summary>Recomputes the entire workbook. Used once after a bulk import.</summary>
    void Rebuild();

    /// <summary>Drops all cached state (parse trees and the dependency graph).</summary>
    void Reset();

    /// <summary>Notifies the engine that a cell was deleted so its edges can be detached.</summary>
    void OnCellRemoved(CellAddress cell);

    /// <summary>The cells a formula directly reads; empty for literals and unknown cells.</summary>
    IReadOnlyCollection<CellAddress> GetPrecedents(CellAddress cell);
}

/// <summary>Raised for malformed formulas and unsupported operations.</summary>
public sealed class FormulaException : Exception
{
    public FormulaException(CellError error, string message)
        : base(message)
    {
        Error = error;
    }

    public CellError Error { get; }
}
