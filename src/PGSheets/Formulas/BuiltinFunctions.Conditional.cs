using PGSheets.Core;

namespace PGSheets.Formulas;

/// <summary>
/// Conditional aggregates. Working out what a criterion means is <see cref="Criteria"/>'s job; what
/// is left here is pairing the tested range with the range that gets summed or averaged.
/// </summary>
public static partial class BuiltinFunctions
{
    /// <summary>COUNTIF(range, criteria) — the criteria may be a number, text or "&gt;=10".</summary>
    private static CellValue CountIf(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count != 2)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryCriteria(arguments[1], context, out var criteria, out var error))
        {
            return error;
        }

        var count = 0;

        foreach (var value in Expand(arguments[0], context))
        {
            if (criteria.Matches(value))
            {
                count++;
            }
        }

        return CellValue.FromNumber(count);
    }

    private static CellValue SumIf(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => ConditionalAggregate(arguments, context, average: false);

    private static CellValue AverageIf(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => ConditionalAggregate(arguments, context, average: true);

    private static bool TryCriteria(
        IFormulaNode node,
        IFormulaContext context,
        out Criteria criteria,
        out CellValue error)
    {
        var value = node.Evaluate(context);

        if (value.IsError)
        {
            criteria = default;
            error = value;
            return false;
        }

        criteria = Criteria.Parse(value);
        error = CellValue.Empty;
        return true;
    }

    /// <summary>The top-left corner of a range or single-cell argument.</summary>
    private static (int Row, int Column)? TopLeft(IFormulaNode node) => node switch
    {
        RangeRefNode range => (range.Row1, range.Column1),
        CellRefNode cell => (cell.Row, cell.Column),
        _ => null,
    };

    /// <summary>
    /// SUMIF and AVERAGEIF. The range being added up lines up with the tested range by offset, so a
    /// sum range is anchored at its own top-left corner and then walked with the tested range's
    /// shape — which is exactly how a spreadsheet treats a sum range of the wrong size. Without a
    /// third argument the tested range is the one that gets added up.
    /// </summary>
    private static CellValue ConditionalAggregate(
        IReadOnlyList<IFormulaNode> arguments,
        IFormulaContext context,
        bool average)
    {
        if (arguments.Count is < 2 or > 3)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (arguments[0] is not RangeRefNode tested)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryCriteria(arguments[1], context, out var criteria, out var error))
        {
            return error;
        }

        (int Row, int Column)? origin = arguments.Count == 3
            ? TopLeft(arguments[2])
            : (tested.Row1, tested.Column1);

        var total = 0d;
        var matched = 0;

        for (var row = tested.Row1; row <= tested.Row2; row++)
        {
            for (var column = tested.Column1; column <= tested.Column2; column++)
            {
                if (origin is not { } corner ||
                    !criteria.Matches(context.GetCellValue(row, column)))
                {
                    continue;
                }

                var value = context.GetCellValue(
                    corner.Row + (row - tested.Row1),
                    corner.Column + (column - tested.Column1));

                if (value.IsError)
                {
                    return value;
                }

                if (value.Kind == CellValueKind.Number)
                {
                    total += value.Number;
                    matched++;
                }
            }
        }

        if (!average)
        {
            return CellValue.FromNumber(total);
        }

        return matched == 0
            ? CellValue.FromError(CellError.DivideByZero)
            : CellValue.FromNumber(total / matched);
    }
}
