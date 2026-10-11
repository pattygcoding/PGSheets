using PGSheets.Core;

namespace PGSheets.Controls;

/// <summary>Raised when the grid wants the host to open the in-place editor for a cell.</summary>
public sealed class EditRequestedEventArgs : EventArgs
{
    public EditRequestedEventArgs(int row, int column, string? seedText)
    {
        Row = row;
        Column = column;
        SeedText = seedText;
    }

    public int Row { get; }

    public int Column { get; }

    /// <summary>
    /// Text the editor should start with, or null to edit the cell's existing raw input
    /// (double-click / F2).
    /// </summary>
    public string? SeedText { get; }
}

/// <summary>Raised when the active cell or the selected range changes.</summary>
public sealed class CellSelectionChangedEventArgs : EventArgs
{
    public CellSelectionChangedEventArgs(int row, int column, CellRange range)
    {
        Row = row;
        Column = column;
        Range = range;
    }

    /// <summary>The active (focused) cell, which is the moving end of the selection.</summary>
    public int Row { get; }

    public int Column { get; }

    /// <summary>The whole selected block, which may be a single cell.</summary>
    public CellRange Range { get; }
}
