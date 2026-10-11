using PGSheets.Core;

namespace PGSheets.Export;

/// <summary>Outcome of an export: the extent written plus any non-fatal problems encountered.</summary>
public sealed class ExportResult
{
    /// <summary>Number of rows in the written extent (1-based count).</summary>
    public int Rows { get; set; }

    /// <summary>Number of columns in the written extent.</summary>
    public int Columns { get; set; }

    /// <summary>How many non-empty cells were written.</summary>
    public int CellsWritten { get; set; }

    /// <summary>Human readable warnings: lossy conversions, renamed sheets, dropped content.</summary>
    public List<string> Warnings { get; } = new();

    public void AddWarning(string message) => CappedWarnings.Add(Warnings, message);
}
