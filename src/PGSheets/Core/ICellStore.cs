namespace PGSheets.Core;

/// <summary>
/// The minimal view of cell storage that the formula engine needs. Keeping this separate from
/// <see cref="SpreadsheetModel"/> lets the engine stay free of UI concerns and makes it trivially
/// testable with an in-memory fake.
/// </summary>
public interface ICellStore
{
    /// <summary>Raw, unparsed input for a cell; empty string when the cell is not populated.</summary>
    string GetRawInput(int row, int column);

    /// <summary>The last computed value for a cell; <see cref="CellValue.Empty"/> when unknown.</summary>
    CellValue GetValue(int row, int column);

    /// <summary>Stores an evaluated result. Implementations must ignore cells that do not exist.</summary>
    void SetComputed(int row, int column, CellValue value);

    /// <summary>Every populated cell, used for a full rebuild after a bulk import.</summary>
    IEnumerable<(int Row, int Column)> PopulatedCells { get; }
}
