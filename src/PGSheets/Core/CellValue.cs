using System.Globalization;

namespace PGSheets.Core;

/// <summary>The kind of value a cell holds after evaluation.</summary>
public enum CellValueKind : byte
{
    Empty,
    Number,
    Text,
    Boolean,
    Error,
}

/// <summary>Spreadsheet error codes surfaced as "#..." display strings.</summary>
public enum CellError : byte
{
    None,
    DivideByZero,
    Value,
    Reference,
    Name,
    Number,
    Circular,
}

/// <summary>
/// An evaluated cell value. This is a value type containing no heap references for numeric or
/// boolean results, which keeps the recalculation path allocation free for the common case.
/// </summary>
public readonly struct CellValue : IEquatable<CellValue>
{
    public static readonly CellValue Empty = default;

    public CellValueKind Kind { get; }

    public double Number { get; }

    public string? Text { get; }

    public bool Boolean { get; }

    public CellError Error { get; }

    private CellValue(CellValueKind kind, double number, string? text, bool boolean, CellError error)
    {
        Kind = kind;
        Number = number;
        Text = text;
        Boolean = boolean;
        Error = error;
    }

    public static CellValue FromNumber(double value) => new(CellValueKind.Number, value, null, false, CellError.None);

    public static CellValue FromText(string? text) =>
        string.IsNullOrEmpty(text) ? Empty : new CellValue(CellValueKind.Text, 0d, text, false, CellError.None);

    public static CellValue FromBoolean(bool value) => new(CellValueKind.Boolean, 0d, null, value, CellError.None);

    public static CellValue FromError(CellError error) => new(CellValueKind.Error, 0d, null, false, error);

    public bool IsError => Kind == CellValueKind.Error;

    public bool IsEmpty => Kind == CellValueKind.Empty;

    /// <summary>
    /// Numeric coercion used by arithmetic and aggregation. Mirrors spreadsheet semantics:
    /// empty equals 0, booleans equal 1/0 and numeric-looking text is coerced.
    /// </summary>
    public bool TryGetNumber(out double value)
    {
        switch (Kind)
        {
            case CellValueKind.Number:
                value = Number;
                return true;
            case CellValueKind.Boolean:
                value = Boolean ? 1d : 0d;
                return true;
            case CellValueKind.Empty:
                value = 0d;
                return true;
            case CellValueKind.Text:
                return double.TryParse(
                    Text,
                    NumberStyles.Float | NumberStyles.AllowThousands,
                    CultureInfo.InvariantCulture,
                    out value);
            default:
                value = 0d;
                return false;
        }
    }

    /// <summary>Numeric coercion with a 0 fallback, for hosts that extend the function set.</summary>
    public double AsNumberOrDefault() => TryGetNumber(out var value) ? value : 0d;

    /// <summary>Interprets the value as a condition (used by IF).</summary>
    public bool AsBoolean() => Kind switch
    {
        CellValueKind.Boolean => Boolean,
        CellValueKind.Number => Number != 0d,
        CellValueKind.Text => string.Equals(Text, "TRUE", StringComparison.OrdinalIgnoreCase),
        _ => false,
    };

    /// <summary>The string the grid renders for this value.</summary>
    public string ToDisplayString() => Kind switch
    {
        CellValueKind.Number => FormatNumber(Number),
        CellValueKind.Text => Text ?? string.Empty,
        CellValueKind.Boolean => Boolean ? "TRUE" : "FALSE",
        CellValueKind.Error => ErrorText(Error),
        _ => string.Empty,
    };

    private static string FormatNumber(double value) => value.ToString("G15", CultureInfo.InvariantCulture);

    public static string ErrorText(CellError error) => error switch
    {
        CellError.DivideByZero => "#DIV/0!",
        CellError.Value => "#VALUE!",
        CellError.Reference => "#REF!",
        CellError.Name => "#NAME?",
        CellError.Number => "#NUM!",
        CellError.Circular => "#CIRCULAR!",
        _ => string.Empty,
    };

    public bool Equals(CellValue other) =>
        Kind == other.Kind &&
        Number.Equals(other.Number) &&
        Boolean == other.Boolean &&
        Error == other.Error &&
        string.Equals(Text, other.Text, StringComparison.Ordinal);

    public override bool Equals(object? obj) => obj is CellValue other && Equals(other);

    public override int GetHashCode() => HashCode.Combine((int)Kind, Number, Text, Boolean, (int)Error);

    public override string ToString() => ToDisplayString();
}
