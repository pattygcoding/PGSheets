using PGSheets.Core;

namespace PGSheets.Formulas;

/// <summary>
/// Lookup functions. A failed match is #N/A — which is what makes IFERROR and IFNA worth wrapping
/// around a lookup — and an index outside the table is #REF!.
/// </summary>
public static partial class BuiltinFunctions
{
    private static CellValue VLookUp(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => Lookup(arguments, context, vertical: true);

    private static CellValue HLookUp(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => Lookup(arguments, context, vertical: false);

    /// <summary>
    /// VLOOKUP and HLOOKUP. The key is searched down the first column, or along the first row, and
    /// the result is read <paramref name="index"/> columns or rows further across.
    /// </summary>
    private static CellValue Lookup(
        IReadOnlyList<IFormulaNode> arguments,
        IFormulaContext context,
        bool vertical)
    {
        if (arguments.Count is < 3 or > 4)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (arguments[1] is not RangeRefNode range)
        {
            // The table must be a range, because the coordinates are what the result is read from.
            return CellValue.FromError(CellError.Value);
        }

        var key = arguments[0].Evaluate(context);

        if (key.IsError)
        {
            return key;
        }

        if (!TryInteger(arguments, 2, context, out var index, out var indexError))
        {
            return indexError;
        }

        var approximate = true;

        if (arguments.Count == 4)
        {
            var rangeLookup = arguments[3].Evaluate(context);

            if (rangeLookup.IsError)
            {
                return rangeLookup;
            }

            approximate = rangeLookup.AsBoolean();
        }

        var length = vertical ? range.Row2 - range.Row1 + 1 : range.Column2 - range.Column1 + 1;
        var width = vertical ? range.Column2 - range.Column1 + 1 : range.Row2 - range.Row1 + 1;

        if (index < 1 || index > width)
        {
            return CellValue.FromError(CellError.Reference);
        }

        var found = -1;

        for (var offset = 0; offset < length; offset++)
        {
            var probe = vertical
                ? context.GetCellValue(range.Row1 + offset, range.Column1)
                : context.GetCellValue(range.Row1, range.Column1 + offset);

            if (probe.IsError)
            {
                return probe;
            }

            var comparison = CompareValues(probe, key);

            if (comparison == 0)
            {
                found = offset;
                break;
            }

            if (!approximate)
            {
                // Exact matching keeps scanning; only an identical value counts.
                continue;
            }

            if (comparison < 0)
            {
                // Below the key, so a candidate — the table is assumed sorted ascending.
                found = offset;
            }
            else
            {
                break;
            }
        }

        if (found < 0)
        {
            return CellValue.FromError(CellError.NotAvailable);
        }

        return vertical
            ? context.GetCellValue(range.Row1 + found, range.Column1 + index - 1)
            : context.GetCellValue(range.Row1 + index - 1, range.Column1 + found);
    }

    /// <summary>MATCH(value, range, [type]) — the 1-based position within a single row or column.</summary>
    private static CellValue Match(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count is < 2 or > 3)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (arguments[1] is not RangeRefNode range)
        {
            return CellValue.FromError(CellError.Value);
        }

        var key = arguments[0].Evaluate(context);

        if (key.IsError)
        {
            return key;
        }

        var matchType = 1;

        if (arguments.Count == 3 && !TryInteger(arguments, 2, context, out matchType, out var typeError))
        {
            return typeError;
        }

        var vertical = range.Row2 > range.Row1;

        if (vertical && range.Column2 > range.Column1)
        {
            // A position only exists along one dimension, so a block has no single answer.
            return CellValue.FromError(CellError.NotAvailable);
        }

        var length = vertical ? range.Row2 - range.Row1 + 1 : range.Column2 - range.Column1 + 1;
        var found = -1;

        for (var offset = 0; offset < length; offset++)
        {
            var probe = vertical
                ? context.GetCellValue(range.Row1 + offset, range.Column1)
                : context.GetCellValue(range.Row1, range.Column1 + offset);

            if (probe.IsError)
            {
                return probe;
            }

            var comparison = CompareValues(probe, key);

            if (comparison == 0)
            {
                found = offset;
                break;
            }

            if (matchType == 0)
            {
                // Exact matching only.
                continue;
            }

            // Type 1 expects the list sorted ascending and type -1 descending; either way the answer
            // is the last value still on the correct side of the key.
            if (matchType > 0 ? comparison < 0 : comparison > 0)
            {
                found = offset;
            }
            else
            {
                break;
            }
        }

        return found < 0 ? CellValue.FromError(CellError.NotAvailable) : CellValue.FromNumber(found + 1);
    }

    /// <summary>
    /// INDEX(range, row, [column]) — 1-based. A single-row range accepts one index, which is then
    /// read as the column, matching a spreadsheet.
    /// </summary>
    private static CellValue Index(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count is < 2 or > 3)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (arguments[0] is not RangeRefNode range)
        {
            return CellValue.FromError(CellError.Value);
        }

        var rows = range.Row2 - range.Row1 + 1;
        var columns = range.Column2 - range.Column1 + 1;

        if (!TryInteger(arguments, 1, context, out var row, out var rowError))
        {
            return rowError;
        }

        var column = 1;

        if (arguments.Count == 3)
        {
            if (!TryInteger(arguments, 2, context, out column, out var columnError))
            {
                return columnError;
            }
        }
        else if (rows == 1)
        {
            column = row;
            row = 1;
        }

        if (row < 1 || row > rows || column < 1 || column > columns)
        {
            return CellValue.FromError(CellError.Reference);
        }

        return context.GetCellValue(range.Row1 + row - 1, range.Column1 + column - 1);
    }

    /// <summary>CHOOSE(index, value, ...) — only the chosen value is evaluated.</summary>
    private static CellValue Choose(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count < 2)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryInteger(arguments, 0, context, out var index, out var indexError))
        {
            return indexError;
        }

        return index < 1 || index >= arguments.Count
            ? CellValue.FromError(CellError.Value)
            : arguments[index].Evaluate(context);
    }
}
