using PGSheets.Core;

namespace PGSheets.Formulas;

/// <summary>
/// Adjusts the references inside a formula for a structural change: a copy, a paste, or rows and
/// columns being inserted or removed.
/// </summary>
/// <remarks>
/// A formula that cannot be parsed, or that contains a node the writer does not understand, is
/// returned unchanged. That is deliberate: leaving a formula exactly as it was is always safer than
/// replacing it with something subtly different.
/// </remarks>
public static class FormulaRewriter
{
    /// <summary>
    /// Shifts every <em>relative</em> reference by a delta and leaves anchored parts alone. This is
    /// what copying, pasting or filling a formula does.
    /// </summary>
    public static string Translate(string formula, int rowDelta, int columnDelta)
    {
        if (rowDelta == 0 && columnDelta == 0)
        {
            return formula;
        }

        return Rewrite(formula, reference => Translate(reference, rowDelta, columnDelta));
    }

    /// <summary>
    /// Adjusts references after rows are inserted at <paramref name="at"/> or removed from it.
    /// Anchors do not protect a reference here, which matches a spreadsheet: inserting a row above
    /// <c>$A$5</c> moves it to <c>$A$6</c>.
    /// </summary>
    public static string AdjustForRows(string formula, int at, int count, bool inserted) =>
        Rewrite(formula, reference => Shift(reference, at, count, inserted, rows: true));

    /// <summary>Adjusts references after columns are inserted or removed.</summary>
    public static string AdjustForColumns(string formula, int at, int count, bool inserted) =>
        Rewrite(formula, reference => Shift(reference, at, count, inserted, rows: false));

    private static string Rewrite(string formula, ReferenceRewriter rewrite)
    {
        if (formula.Length == 0 || formula[0] != '=')
        {
            return formula;
        }

        if (!FormulaParser.TryParse(formula, out var node, out _))
        {
            return formula;
        }

        try
        {
            return FormulaWriter.Write(node, rewrite);
        }
        catch (Exception)
        {
            // Never let a rewrite turn a working formula into a broken one.
            return formula;
        }
    }

    private static CellReference? Translate(CellReference reference, int rowDelta, int columnDelta)
    {
        var row = reference.AbsoluteRow ? reference.Row : reference.Row + rowDelta;
        var column = reference.AbsoluteColumn ? reference.Column : reference.Column + columnDelta;

        return IsOnSheet(row, column) ? reference with { Row = row, Column = column } : null;
    }

    private static CellReference? Shift(CellReference reference, int at, int count, bool inserted, bool rows)
    {
        var position = rows ? reference.Row : reference.Column;

        if (inserted)
        {
            if (position >= at)
            {
                position += count;
            }
        }
        else
        {
            // The reference sits inside the band that was removed.
            if (position >= at && position < at + count)
            {
                return null;
            }

            if (position >= at + count)
            {
                position -= count;
            }
        }

        var limit = rows ? SheetBounds.MaxRows : SheetBounds.MaxColumns;

        if (position < 0 || position >= limit)
        {
            return null;
        }

        return rows ? reference with { Row = position } : reference with { Column = position };
    }

    private static bool IsOnSheet(int row, int column) =>
        row >= 0 && row < SheetBounds.MaxRows && column >= 0 && column < SheetBounds.MaxColumns;
}
