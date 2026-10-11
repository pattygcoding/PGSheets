namespace PGSheets.Import;

using PGSheets.Core;

/// <summary>Outcome of an import: the used extent plus any non-fatal problems encountered.</summary>
public sealed class ImportResult
{
    /// <summary>Number of rows that contained data (1-based count).</summary>
    public int Rows { get; set; }

    /// <summary>Number of columns that contained data.</summary>
    public int Columns { get; set; }

    /// <summary>Human readable warnings: unsupported functions, truncated ranges, and so on.</summary>
    public List<string> Warnings { get; } = new();

    /// <summary>Name of the sheet/table that was imported, when known.</summary>
    public string? SheetName { get; set; }

    /// <summary>How many non-empty cells were written to the model.</summary>
    public int CellsImported { get; set; }

    /// <summary>
    /// How many worksheets the source document held. Saving writes a single worksheet, so a caller
    /// that sees more than one here must not write back over the original file: it would destroy
    /// every sheet that was never loaded.
    /// </summary>
    public int WorksheetCount { get; set; } = 1;

    public void AddWarning(string message) => CappedWarnings.Add(Warnings, message);
}
