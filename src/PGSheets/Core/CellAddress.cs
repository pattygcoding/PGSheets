using System.Globalization;

namespace PGSheets.Core;

/// <summary>
/// A zero-based cell coordinate. Row 0 / Column 0 maps to the A1 notation "A1".
/// A <see cref="CellAddress"/> is a value type, so it can be used as a dictionary key
/// without boxing or heap allocation.
/// </summary>
public readonly record struct CellAddress(int Row, int Column)
{
    public override string ToString() => ToA1(Row, Column);

    /// <summary>Converts a zero-based column index to spreadsheet letters (0 =&gt; "A", 25 =&gt; "Z", 26 =&gt; "AA").</summary>
    public static string ToColumnName(int column)
    {
        if (column < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(column));
        }

        Span<char> buffer = stackalloc char[8];
        var index = buffer.Length;
        var value = column;

        do
        {
            buffer[--index] = (char)('A' + value % 26);
            value = value / 26 - 1;
        }
        while (value >= 0);

        return new string(buffer[index..]);
    }

    /// <summary>Formats a zero-based (row, column) pair as A1 notation.</summary>
    public static string ToA1(int row, int column)
    {
        if (row < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(row));
        }

        return string.Concat(ToColumnName(column), (row + 1).ToString(CultureInfo.InvariantCulture));
    }

    /// <summary>
    /// Parses A1 notation such as "A1" or "$B$7". Returns <c>false</c> for anything malformed.
    /// </summary>
    public static bool TryParseA1(string? text, out int row, out int column)
    {
        row = -1;
        column = -1;

        if (string.IsNullOrEmpty(text))
        {
            return false;
        }

        var index = 0;
        if (text[index] == '$')
        {
            index++;
        }

        var columnNumber = 0;
        var letterCount = 0;

        while (index < text.Length)
        {
            var ch = text[index];
            if (ch is >= 'A' and <= 'Z')
            {
                columnNumber = columnNumber * 26 + (ch - 'A' + 1);
                letterCount++;
                index++;
                continue;
            }

            if (ch is >= 'a' and <= 'z')
            {
                columnNumber = columnNumber * 26 + (ch - 'a' + 1);
                letterCount++;
                index++;
                continue;
            }

            break;
        }

        if (letterCount is 0 or > 3)
        {
            return false;
        }

        if (index < text.Length && text[index] == '$')
        {
            index++;
        }

        var rowNumber = 0;
        var digitCount = 0;

        while (index < text.Length)
        {
            var ch = text[index];
            if (ch is < '0' or > '9')
            {
                return false;
            }

            rowNumber = rowNumber * 10 + (ch - '0');
            digitCount++;
            index++;
        }

        if (digitCount == 0 || rowNumber < 1)
        {
            return false;
        }

        row = rowNumber - 1;
        column = columnNumber - 1;
        return true;
    }
}
