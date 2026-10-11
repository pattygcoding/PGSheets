namespace PGSheets.Import;

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

    public void AddWarning(string message)
    {
        if (Warnings.Count < 100)
        {
            Warnings.Add(message);
        }
        else if (Warnings.Count == 100)
        {
            Warnings.Add("Further warnings suppressed.");
        }
    }
}
