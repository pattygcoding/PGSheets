using PGSheets.Core;

namespace PGSheets.Formulas;

/// <summary>
/// A built-in function implementation. Arguments are passed as unevaluated nodes so the
/// implementation controls evaluation order and can enumerate ranges.
/// </summary>
public delegate CellValue FormulaFunction(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context);

/// <summary>
/// Registry of supported worksheet functions. Extend the language by calling
/// <see cref="Register"/> once at start-up; no parser changes are required.
/// </summary>
public static partial class BuiltinFunctions
{
    private static readonly Dictionary<string, FormulaFunction> Registry = new(StringComparer.OrdinalIgnoreCase)
    {
        ["SUM"] = Sum,
        ["MIN"] = Min,
        ["MAX"] = Max,
        ["AVERAGE"] = Average,
        ["COUNT"] = Count,
        ["COUNTA"] = CountA,
        ["IF"] = If,
        ["ABS"] = Abs,
        ["ROUND"] = Round,
        ["SQRT"] = Sqrt,
        ["POWER"] = Power,
        ["CONCAT"] = Concat,
    };

    public static bool TryGet(string name, out FormulaFunction function) => Registry.TryGetValue(name, out function!);

    public static bool IsKnown(string name) => Registry.ContainsKey(name);

    /// <summary>Adds or replaces a function definition.</summary>
    public static void Register(string name, FormulaFunction function) => Registry[name] = function;

    public static IReadOnlyCollection<string> Names => Registry.Keys;

    /// <summary>Expands an argument into the sequence of values it contributes.</summary>
    private static IEnumerable<CellValue> Expand(IFormulaNode node, IFormulaContext context) =>
        node is RangeRefNode range ? range.Enumerate(context) : new[] { node.Evaluate(context) };

    private static CellValue Sum(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        var total = 0d;

        foreach (var argument in arguments)
        {
            foreach (var value in Expand(argument, context))
            {
                if (value.IsError)
                {
                    return value;
                }

                if (value.Kind == CellValueKind.Number)
                {
                    total += value.Number;
                }
                else if (value.Kind == CellValueKind.Boolean)
                {
                    total += value.Boolean ? 1d : 0d;
                }
            }
        }

        return CellValue.FromNumber(total);
    }

    private static CellValue Min(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => Extremum(arguments, context, findMaximum: false);

    private static CellValue Max(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => Extremum(arguments, context, findMaximum: true);

    private static CellValue Extremum(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context, bool findMaximum)
    {
        var found = false;
        var best = findMaximum ? double.NegativeInfinity : double.PositiveInfinity;

        foreach (var argument in arguments)
        {
            foreach (var value in Expand(argument, context))
            {
                if (value.IsError)
                {
                    return value;
                }

                if (value.Kind != CellValueKind.Number)
                {
                    continue;
                }

                found = true;
                best = findMaximum ? Math.Max(best, value.Number) : Math.Min(best, value.Number);
            }
        }

        return CellValue.FromNumber(found ? best : 0d);
    }

    private static CellValue Average(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        var total = 0d;
        var count = 0;

        foreach (var argument in arguments)
        {
            foreach (var value in Expand(argument, context))
            {
                if (value.IsError)
                {
                    return value;
                }

                if (value.Kind == CellValueKind.Number)
                {
                    total += value.Number;
                    count++;
                }
            }
        }

        return count == 0
            ? CellValue.FromError(CellError.DivideByZero)
            : CellValue.FromNumber(total / count);
    }

    private static CellValue Count(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        var count = 0;

        foreach (var argument in arguments)
        {
            foreach (var value in Expand(argument, context))
            {
                if (value.Kind == CellValueKind.Number)
                {
                    count++;
                }
            }
        }

        return CellValue.FromNumber(count);
    }

    private static CellValue CountA(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        var count = 0;

        foreach (var argument in arguments)
        {
            foreach (var value in Expand(argument, context))
            {
                if (!value.IsEmpty)
                {
                    count++;
                }
            }
        }

        return CellValue.FromNumber(count);
    }
}
