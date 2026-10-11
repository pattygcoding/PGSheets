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
        // Numbers
        ["SUM"] = Sum,
        ["PRODUCT"] = Product,
        ["ABS"] = Abs,
        ["SIGN"] = Sign,
        ["SQRT"] = Sqrt,
        ["POWER"] = Power,
        ["EXP"] = Exp,
        ["LN"] = Ln,
        ["LOG"] = Log,
        ["LOG10"] = Log10,
        ["PI"] = Pi,
        ["INT"] = Int,
        ["TRUNC"] = Trunc,
        ["MOD"] = Mod,
        ["ROUND"] = Round,
        ["ROUNDUP"] = RoundUp,
        ["ROUNDDOWN"] = RoundDown,
        ["CEILING"] = Ceiling,
        ["FLOOR"] = Floor,

        // Statistics and conditional aggregates
        ["MIN"] = Min,
        ["MAX"] = Max,
        ["AVERAGE"] = Average,
        ["MEDIAN"] = Median,
        ["MODE"] = Mode,
        ["COUNT"] = Count,
        ["COUNTA"] = CountA,
        ["COUNTBLANK"] = CountBlank,
        ["COUNTIF"] = CountIf,
        ["SUMIF"] = SumIf,
        ["AVERAGEIF"] = AverageIf,
        ["LARGE"] = Large,
        ["SMALL"] = Small,
        ["RANK"] = Rank,
        ["STDEV"] = StDev,
        ["VAR"] = Variance,
        ["SUMPRODUCT"] = SumProduct,

        // Logic
        ["IF"] = If,
        ["IFS"] = Ifs,
        ["IFERROR"] = IfError,
        ["IFNA"] = IfNa,
        ["AND"] = And,
        ["OR"] = Or,
        ["XOR"] = Xor,
        ["NOT"] = Not,
        ["SWITCH"] = Switch,
        ["CHOOSE"] = Choose,
        ["TRUE"] = True,
        ["FALSE"] = False,
        ["NA"] = Na,

        // Text
        ["CONCAT"] = Concat,
        ["CONCATENATE"] = Concat,
        ["TEXTJOIN"] = TextJoin,
        ["LEFT"] = Left,
        ["RIGHT"] = Right,
        ["MID"] = Mid,
        ["LEN"] = Len,
        ["UPPER"] = Upper,
        ["LOWER"] = Lower,
        ["PROPER"] = Proper,
        ["TRIM"] = Trim,
        ["REPT"] = Rept,
        ["SUBSTITUTE"] = Substitute,
        ["FIND"] = Find,
        ["EXACT"] = Exact,
        ["VALUE"] = Value,
        ["CHAR"] = Character,
        ["CODE"] = Code,

        // Lookup
        ["VLOOKUP"] = VLookUp,
        ["HLOOKUP"] = HLookUp,
        ["INDEX"] = Index,
        ["MATCH"] = Match,

        // Dates
        ["YEAR"] = Year,
        ["MONTH"] = Month,
        ["DAY"] = Day,
        ["WEEKDAY"] = WeekDay,

        // Type tests
        ["ISBLANK"] = IsBlank,
        ["ISNUMBER"] = IsNumber,
        ["ISTEXT"] = IsText,
        ["ISLOGICAL"] = IsLogical,
        ["ISERROR"] = IsError,
        ["ISNA"] = IsNa,
        ["ISEVEN"] = IsEven,
        ["ISODD"] = IsOdd,
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
