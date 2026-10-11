using System.Globalization;
using System.Text;

namespace PGSheets.Core;

/// <summary>
/// A single cell reference as it appears in a formula, including its <c>$</c> anchors.
/// </summary>
/// <remarks>
/// The anchors are what make a reference behave correctly when a formula is copied: a relative part
/// shifts with the copy, an anchored one stays put. Inserting or deleting rows shifts every
/// reference regardless of its anchors, which is also how a spreadsheet behaves.
/// </remarks>
public readonly record struct CellReference(int Row, int Column, bool AbsoluteRow, bool AbsoluteColumn)
{
    /// <summary>An unanchored reference, which is what a plain <c>A1</c> means.</summary>
    public static CellReference Relative(int row, int column) => new(row, column, false, false);

    /// <summary>Formats as <c>B7</c>, <c>$B$7</c>, <c>B$7</c> or <c>$B7</c>.</summary>
    public string ToA1()
    {
        var builder = new StringBuilder(8);

        if (AbsoluteColumn)
        {
            builder.Append('$');
        }

        builder.Append(CellAddress.ToColumnName(Column));

        if (AbsoluteRow)
        {
            builder.Append('$');
        }

        builder.Append((Row + 1).ToString(CultureInfo.InvariantCulture));

        return builder.ToString();
    }

    public override string ToString() => ToA1();
}
