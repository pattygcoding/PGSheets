namespace PGSheets.Core;

/// <summary>
/// Interprets raw, non-formula cell input ("1200", "North", "TRUE") as a typed value.
/// Kept in one place so both the engine and future importers agree on the rules.
/// </summary>
public static class Literals
{
    public static CellValue Parse(string raw)
    {
        if (string.IsNullOrEmpty(raw))
        {
            return CellValue.Empty;
        }

        if (raw[0] == '=')
        {
            // A formula that never reached the parser; surface it rather than guessing.
            return CellValue.FromText(raw);
        }

        if (double.TryParse(
                raw,
                System.Globalization.NumberStyles.Float | System.Globalization.NumberStyles.AllowThousands,
                System.Globalization.CultureInfo.InvariantCulture,
                out var number))
        {
            return CellValue.FromNumber(number);
        }

        if (string.Equals(raw, "TRUE", StringComparison.OrdinalIgnoreCase))
        {
            return CellValue.FromBoolean(true);
        }

        if (string.Equals(raw, "FALSE", StringComparison.OrdinalIgnoreCase))
        {
            return CellValue.FromBoolean(false);
        }

        return CellValue.FromText(raw);
    }
}
