using PGSheets.Core;

namespace PGSheets.Export;

/// <summary>Caller-supplied settings for a single export run.</summary>
public sealed class ExportOptions
{
    /// <summary>Worksheet name to write. Sanitised by the writer.</summary>
    public string SheetName { get; init; } = "Sheet1";

    /// <summary>
    /// When false (the default) each cell's <em>raw input</em> is written, so formulas survive a
    /// save/load round trip. When true the computed <em>display</em> value is written instead,
    /// matching what the grid shows on screen.
    /// </summary>
    public bool ExportValues { get; init; }

    /// <summary>
    /// Restricts the export to a rectangle. Null (the default) writes the whole used extent.
    /// A range keeps its full shape including empty cells, because the caller is exporting a
    /// selected block and it has to line up with that selection.
    /// </summary>
    public CellRange? Range { get; init; }
}
