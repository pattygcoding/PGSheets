namespace PGSheets.Core;

/// <summary>
/// A single sparse cell record: exactly what the user typed (<see cref="RawInput"/>) plus the
/// result the formula engine computed for it (<see cref="Value"/> and <see cref="DisplayValue"/>).
/// </summary>
public sealed class CellData
{
    public CellData(string rawInput)
    {
        RawInput = rawInput;
    }

    /// <summary>The stored input, e.g. <c>"1200"</c>, <c>"North"</c> or <c>"=SUM(A1:A5)"</c>.</summary>
    public string RawInput { get; internal set; }

    /// <summary>
    /// The number format code for the cell, e.g. <c>"0.00"</c> or <c>"yyyy-mm-dd"</c>. Null means
    /// General: the plain rendering.
    /// </summary>
    public string? NumberFormat { get; internal set; }

    /// <summary>The typed result produced by the engine.</summary>
    public CellValue Value { get; internal set; } = CellValue.Empty;

    /// <summary>
    /// Text shown by the grid. Kept in sync with <see cref="Value"/> by the model, and rendered
    /// through <see cref="NumberFormat"/> so a serial shows as a date and a fraction as a percentage.
    /// </summary>
    public string DisplayValue { get; internal set; } = string.Empty;

    /// <summary>True when the raw input is a formula (starts with '=').</summary>
    public bool IsFormula => RawInput.Length > 0 && RawInput[0] == '=';

    /// <summary>True when the cell exists only to carry a format, with no value of its own.</summary>
    public bool IsFormatOnly => RawInput.Length == 0 && NumberFormat is not null;

    public override string ToString() => $"{RawInput} => {DisplayValue}";
}
