using PGSheets.Core;

namespace PGSheets.Formulas;

/// <summary>
/// Shared argument handling for the built-in functions. Every function reports a bad argument as
/// #VALUE! and propagates an error it is given, so those rules live here rather than being repeated
/// in sixty implementations.
/// </summary>
public static partial class BuiltinFunctions
{
    /// <summary>Excel's limit on the characters one cell can hold, used to refuse absurd REPT.</summary>
    private const int MaxTextLength = 32767;

    /// <summary>
    /// Evaluates the argument at <paramref name="index"/>, requiring a number. Returns false with
    /// <paramref name="error"/> filled in when the argument is missing, an error, or not numeric.
    /// </summary>
    private static bool TryNumber(
        IReadOnlyList<IFormulaNode> arguments,
        int index,
        IFormulaContext context,
        out double value,
        out CellValue error)
    {
        value = 0d;

        if (index >= arguments.Count)
        {
            error = CellValue.FromError(CellError.Value);
            return false;
        }

        var result = arguments[index].Evaluate(context);

        if (result.IsError)
        {
            error = result;
            return false;
        }

        if (!result.TryGetNumber(out value))
        {
            error = CellValue.FromError(CellError.Value);
            return false;
        }

        error = CellValue.Empty;
        return true;
    }

    /// <summary>Evaluates an argument and rounds it to an integer, as a count or index argument.</summary>
    private static bool TryInteger(
        IReadOnlyList<IFormulaNode> arguments,
        int index,
        IFormulaContext context,
        out int value,
        out CellValue error)
    {
        value = 0;

        if (!TryNumber(arguments, index, context, out var number, out error))
        {
            return false;
        }

        value = (int)Math.Round(number, MidpointRounding.AwayFromZero);
        return true;
    }

    /// <summary>The text of a value: an empty cell contributes nothing, a number its display form.</summary>
    private static string TextOf(CellValue value) => value.IsEmpty ? string.Empty : value.ToDisplayString();

    /// <summary>Evaluates an argument and returns its text, using the display form of a number.</summary>
    private static CellValue TextArgument(IFormulaNode node, IFormulaContext context)
    {
        var value = node.Evaluate(context);

        if (value.IsError)
        {
            return value;
        }

        return CellValue.FromText(TextOf(value));
    }

    /// <summary>
    /// Gathers every numeric value the arguments contribute. Text and blanks are skipped, which is
    /// how an aggregate treats a range; an error anywhere is returned instead.
    /// </summary>
    private static bool TryCollectNumbers(
        IReadOnlyList<IFormulaNode> arguments,
        IFormulaContext context,
        List<double> numbers,
        out CellValue error)
    {
        foreach (var argument in arguments)
        {
            foreach (var value in Expand(argument, context))
            {
                if (value.IsError)
                {
                    error = value;
                    return false;
                }

                if (value.Kind == CellValueKind.Number)
                {
                    numbers.Add(value.Number);
                }
            }
        }

        error = CellValue.Empty;
        return true;
    }

    /// <summary>
    /// Compares two values the way a lookup does: numbers numerically, anything else as text, and
    /// text comparisons ignore case.
    /// </summary>
    private static int CompareValues(CellValue left, CellValue right)
    {
        // TryGetNumber is evaluated first (not after a guard) so the out arguments are definitely
        // assigned; an empty cell coerces to zero, which is why it is excluded separately.
        var leftIsNumeric = left.TryGetNumber(out var leftNumber) && !left.IsEmpty;
        var rightIsNumeric = right.TryGetNumber(out var rightNumber) && !right.IsEmpty;

        if (leftIsNumeric && rightIsNumeric)
        {
            return leftNumber.CompareTo(rightNumber);
        }

        return string.Compare(TextOf(left), TextOf(right), StringComparison.OrdinalIgnoreCase);
    }
}
