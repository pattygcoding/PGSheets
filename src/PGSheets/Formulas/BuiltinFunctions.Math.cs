using System.Text;
using PGSheets.Core;

namespace PGSheets.Formulas;

public static partial class BuiltinFunctions
{
    /// <summary>Lazy conditional: only the selected branch is evaluated.</summary>
    private static CellValue If(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count < 2)
        {
            return CellValue.FromError(CellError.Value);
        }

        var condition = arguments[0].Evaluate(context);
        if (condition.IsError)
        {
            return condition;
        }

        if (condition.AsBoolean())
        {
            return arguments[1].Evaluate(context);
        }

        return arguments.Count > 2 ? arguments[2].Evaluate(context) : CellValue.FromBoolean(false);
    }

    private static CellValue Abs(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => UnaryMath(arguments, context, Math.Abs);

    private static CellValue Sqrt(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => UnaryMath(arguments, context, value => value < 0d ? double.NaN : Math.Sqrt(value));

    private static CellValue UnaryMath(
        IReadOnlyList<IFormulaNode> arguments,
        IFormulaContext context,
        Func<double, double> operation)
    {
        if (arguments.Count != 1)
        {
            return CellValue.FromError(CellError.Value);
        }

        var value = arguments[0].Evaluate(context);
        if (value.IsError)
        {
            return value;
        }

        if (!value.TryGetNumber(out var number))
        {
            return CellValue.FromError(CellError.Value);
        }

        var result = operation(number);
        return double.IsNaN(result) ? CellValue.FromError(CellError.Number) : CellValue.FromNumber(result);
    }

    private static CellValue Round(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count is < 1 or > 2)
        {
            return CellValue.FromError(CellError.Value);
        }

        var value = arguments[0].Evaluate(context);
        if (value.IsError)
        {
            return value;
        }

        if (!value.TryGetNumber(out var number))
        {
            return CellValue.FromError(CellError.Value);
        }

        var digits = 0;

        if (arguments.Count == 2)
        {
            var digitsValue = arguments[1].Evaluate(context);
            if (digitsValue.IsError)
            {
                return digitsValue;
            }

            if (!digitsValue.TryGetNumber(out var d))
            {
                return CellValue.FromError(CellError.Value);
            }

            digits = (int)Math.Round(d);
        }

        return CellValue.FromNumber(Math.Round(number, Math.Clamp(digits, 0, 15), MidpointRounding.AwayFromZero));
    }

    private static CellValue Power(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count != 2)
        {
            return CellValue.FromError(CellError.Value);
        }

        var left = arguments[0].Evaluate(context);
        if (left.IsError)
        {
            return left;
        }

        var right = arguments[1].Evaluate(context);
        if (right.IsError)
        {
            return right;
        }

        if (!left.TryGetNumber(out var l) || !right.TryGetNumber(out var r))
        {
            return CellValue.FromError(CellError.Value);
        }

        return CellValue.FromNumber(Math.Pow(l, r));
    }

    private static CellValue Concat(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        var builder = new StringBuilder();

        foreach (var argument in arguments)
        {
            foreach (var value in Expand(argument, context))
            {
                if (value.IsError)
                {
                    return value;
                }

                builder.Append(value.ToDisplayString());
            }
        }

        return CellValue.FromText(builder.ToString());
    }
}
