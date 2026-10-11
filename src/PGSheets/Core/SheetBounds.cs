namespace PGSheets.Core;

/// <summary>
/// Hard limits of the virtual sheet. They match Excel so imported workbooks never exceed them.
/// </summary>
public static class SheetBounds
{
    /// <summary>Maximum number of rows (1-based 1,048,576).</summary>
    public const int MaxRows = 1_048_576;

    /// <summary>Maximum number of columns (A..XFD).</summary>
    public const int MaxColumns = 16_384;
}
