namespace PGSheets.Core;

/// <summary>
/// The minimal read-only view of cell storage that exporters need. Kept separate from
/// <see cref="ICellStore"/> so a writer cannot write back into the workbook, and so exporters stay
/// unit-testable with an in-memory fake.
/// </summary>
public interface IExportSource
{
    /// <summary>Every populated cell, in no particular order.</summary>
    IEnumerable<(int Row, int Column)> PopulatedCells { get; }

    /// <summary>Raw, unparsed input for a cell; empty string when the cell is not populated.</summary>
    string GetRawInput(int row, int column);

    /// <summary>The last computed value for a cell; <see cref="CellValue.Empty"/> when unknown.</summary>
    CellValue GetValue(int row, int column);

    /// <summary>
    /// The number format code of a cell, or null when it has none. A format changes only what a cell
    /// shows, so a writer needs it to keep the grid's appearance rather than just its values.
    /// </summary>
    string? GetNumberFormat(int row, int column);
}
