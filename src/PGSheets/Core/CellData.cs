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

    /// <summary>The typed result produced by the engine.</summary>
    public CellValue Value { get; internal set; } = CellValue.Empty;

    /// <summary>Text shown by the grid. Kept in sync with <see cref="Value"/> by the model.</summary>
    public string DisplayValue { get; internal set; } = string.Empty;

    /// <summary>True when the raw input is a formula (starts with '=').</summary>
    public bool IsFormula => RawInput.Length > 0 && RawInput[0] == '=';

    public override string ToString() => $"{RawInput} => {DisplayValue}";
}
