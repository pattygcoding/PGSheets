using PGSheets.Core;

namespace PGSheets.Formulas;

/// <summary>
/// Logical functions. The conditional ones are lazy on purpose: IFERROR, IFS and SWITCH must not
/// evaluate a branch they are not going to return, or a guarded division would still raise.
/// </summary>
public static partial class BuiltinFunctions
{
    private enum LogicMode
    {
        And,
        Or,
        Xor,
    }

    /// <summary>Returns the fallback only when the first argument is an error.</summary>
    private static CellValue IfError(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count != 2)
        {
            return CellValue.FromError(CellError.Value);
        }

        var value = arguments[0].Evaluate(context);

        return value.IsError ? arguments[1].Evaluate(context) : value;
    }

    /// <summary>Like IFERROR, but only catches #N/A so a real mistake still shows through.</summary>
    private static CellValue IfNa(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count != 2)
        {
            return CellValue.FromError(CellError.Value);
        }

        var value = arguments[0].Evaluate(context);

        return value.IsError && value.Error == CellError.NotAvailable
            ? arguments[1].Evaluate(context)
            : value;
    }

    private static CellValue And(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => Logic(arguments, context, LogicMode.And);

    private static CellValue Or(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => Logic(arguments, context, LogicMode.Or);

    private static CellValue Xor(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => Logic(arguments, context, LogicMode.Xor);

    /// <summary>
    /// AND, OR and XOR over any number of arguments. Text and blanks inside a range are skipped
    /// rather than counted as false, which is what a spreadsheet does, and an error propagates.
    /// </summary>
    private static CellValue Logic(
        IReadOnlyList<IFormulaNode> arguments,
        IFormulaContext context,
        LogicMode mode)
    {
        var found = false;
        var accumulator = mode == LogicMode.And;
        var trueCount = 0;

        foreach (var argument in arguments)
        {
            foreach (var value in Expand(argument, context))
            {
                if (value.IsError)
                {
                    return value;
                }

                if (value.Kind is not (CellValueKind.Boolean or CellValueKind.Number))
                {
                    // Text and blanks in a range are ignored rather than treated as false.
                    continue;
                }

                var truth = value.Kind == CellValueKind.Boolean ? value.Boolean : value.Number != 0d;

                found = true;

                if (truth)
                {
                    trueCount++;
                }

                accumulator = mode switch
                {
                    LogicMode.And => accumulator && truth,
                    LogicMode.Or => accumulator || truth,
                    _ => accumulator,
                };
            }
        }

        if (!found)
        {
            return CellValue.FromError(CellError.Value);
        }

        return CellValue.FromBoolean(mode == LogicMode.Xor ? trueCount % 2 == 1 : accumulator);
    }

    private static CellValue Not(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count != 1)
        {
            return CellValue.FromError(CellError.Value);
        }

        var value = arguments[0].Evaluate(context);

        return value.IsError ? value : CellValue.FromBoolean(!value.AsBoolean());
    }

    /// <summary>Nested conditions, evaluated in order and lazily.</summary>
    private static CellValue Ifs(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count < 2 || arguments.Count % 2 != 0)
        {
            return CellValue.FromError(CellError.Value);
        }

        for (var index = 0; index + 1 < arguments.Count; index += 2)
        {
            var condition = arguments[index].Evaluate(context);

            if (condition.IsError)
            {
                return condition;
            }

            if (condition.AsBoolean())
            {
                return arguments[index + 1].Evaluate(context);
            }
        }

        // Nothing matched, which a spreadsheet reports as #N/A rather than as FALSE.
        return CellValue.FromError(CellError.NotAvailable);
    }

    /// <summary>SWITCH(expression, value, result, ..., [default]). Only the chosen result is evaluated.</summary>
    private static CellValue Switch(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count < 3)
        {
            return CellValue.FromError(CellError.Value);
        }

        var subject = arguments[0].Evaluate(context);
        if (subject.IsError)
        {
            return subject;
        }

        // An odd number of trailing arguments means the last one is the default result.
        var hasDefault = (arguments.Count - 1) % 2 == 1;
        var limit = hasDefault ? arguments.Count - 1 : arguments.Count;

        for (var index = 1; index + 1 < limit; index += 2)
        {
            var candidate = arguments[index].Evaluate(context);

            if (candidate.IsError)
            {
                return candidate;
            }

            if (CompareValues(subject, candidate) == 0)
            {
                return arguments[index + 1].Evaluate(context);
            }
        }

        return hasDefault ? arguments[^1].Evaluate(context) : CellValue.FromError(CellError.NotAvailable);
    }

    private static CellValue True(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context) =>
        arguments.Count == 0 ? CellValue.FromBoolean(true) : CellValue.FromError(CellError.Value);

    private static CellValue False(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context) =>
        arguments.Count == 0 ? CellValue.FromBoolean(false) : CellValue.FromError(CellError.Value);

    private static CellValue Na(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context) =>
        arguments.Count == 0 ? CellValue.FromError(CellError.NotAvailable) : CellValue.FromError(CellError.Value);
}
