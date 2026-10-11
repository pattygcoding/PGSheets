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

    /// <summary>Rounds down to the nearest whole number, the way INT does (towards -inf, not zero).</summary>
    private static CellValue Int(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => UnaryMath(arguments, context, Math.Floor);

    private static CellValue Sign(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => UnaryMath(arguments, context, value => Math.Sign(value));

    private static CellValue Exp(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => UnaryMath(arguments, context, Math.Exp);

    private static CellValue Ln(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => UnaryMath(arguments, context, value => value <= 0d ? double.NaN : Math.Log(value));

    private static CellValue Log10(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => UnaryMath(arguments, context, value => value <= 0d ? double.NaN : Math.Log10(value));

    private static CellValue Pi(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context) =>
        arguments.Count == 0 ? CellValue.FromNumber(Math.PI) : CellValue.FromError(CellError.Value);

    /// <summary>LOG(number, [base]) — base 10 unless one is given.</summary>
    private static CellValue Log(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count is < 1 or > 2)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryNumber(arguments, 0, context, out var number, out var error))
        {
            return error;
        }

        var logBase = 10d;

        if (arguments.Count == 2 && !TryNumber(arguments, 1, context, out logBase, out var baseError))
        {
            return baseError;
        }

        if (number <= 0d || logBase <= 0d || logBase == 1d)
        {
            return CellValue.FromError(CellError.Number);
        }

        return CellValue.FromNumber(Math.Log(number, logBase));
    }

    /// <summary>Round away from zero to a number of digits.</summary>
    private static CellValue RoundUp(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => RoundDirectional(arguments, context, value => value >= 0d ? Math.Ceiling(value) : Math.Floor(value));

    /// <summary>Round towards zero to a number of digits.</summary>
    private static CellValue RoundDown(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => RoundDirectional(arguments, context, Math.Truncate);

    private static CellValue Trunc(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => RoundDirectional(arguments, context, Math.Truncate);

    /// <summary>
    /// ROUNDUP, ROUNDDOWN and TRUNC differ only in which way they move a value that sits between two
    /// candidates, so the three share this.
    /// </summary>
    private static CellValue RoundDirectional(
        IReadOnlyList<IFormulaNode> arguments,
        IFormulaContext context,
        Func<double, double> adjust)
    {
        if (arguments.Count is < 1 or > 2)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryNumber(arguments, 0, context, out var number, out var error))
        {
            return error;
        }

        var digits = 0;

        if (arguments.Count == 2 && !TryInteger(arguments, 1, context, out digits, out var digitsError))
        {
            return digitsError;
        }

        // Scaling by a power of ten turns "round to n digits" into "round to a whole number".
        var factor = Math.Pow(10d, Math.Clamp(digits, -15, 15));

        return CellValue.FromNumber(adjust(number * factor) / factor);
    }

    private static CellValue Ceiling(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => MultipleOf(arguments, context, roundUp: true);

    private static CellValue Floor(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => MultipleOf(arguments, context, roundUp: false);

    /// <summary>CEILING and FLOOR round to a multiple of a significance, 1 by default.</summary>
    private static CellValue MultipleOf(
        IReadOnlyList<IFormulaNode> arguments,
        IFormulaContext context,
        bool roundUp)
    {
        if (arguments.Count is < 1 or > 2)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryNumber(arguments, 0, context, out var number, out var error))
        {
            return error;
        }

        var significance = 1d;

        if (arguments.Count == 2 && !TryNumber(arguments, 1, context, out significance, out var significanceError))
        {
            return significanceError;
        }

        if (significance == 0d)
        {
            return CellValue.FromNumber(0d);
        }

        // Working in multiples of the significance reduces both cases to a plain ceiling or floor.
        // A negative significance reverses the direction, which is what a spreadsheet does.
        var upward = significance > 0d ? roundUp : !roundUp;
        var multiple = number / significance;

        return CellValue.FromNumber((upward ? Math.Ceiling(multiple) : Math.Floor(multiple)) * significance);
    }

    /// <summary>MOD is the spreadsheet remainder, which follows the divisor's sign rather than C#'s '%'.</summary>
    private static CellValue Mod(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count != 2)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryNumber(arguments, 0, context, out var number, out var error))
        {
            return error;
        }

        if (!TryNumber(arguments, 1, context, out var divisor, out var divisorError))
        {
            return divisorError;
        }

        if (divisor == 0d)
        {
            return CellValue.FromError(CellError.DivideByZero);
        }

        // MOD(-3, 5) is 2, whereas -3 % 5 in C# is -3.
        return CellValue.FromNumber(number - (divisor * Math.Floor(number / divisor)));
    }

    /// <summary>Multiplies every numeric value it is given; text and blanks are ignored.</summary>
    private static CellValue Product(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        var numbers = new List<double>();

        if (!TryCollectNumbers(arguments, context, numbers, out var error))
        {
            return error;
        }

        if (numbers.Count == 0)
        {
            return CellValue.FromNumber(0d);
        }

        var product = 1d;

        foreach (var number in numbers)
        {
            product *= number;
        }

        return CellValue.FromNumber(product);
    }
}
