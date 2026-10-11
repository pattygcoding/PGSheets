using PGSheets.Core;

namespace PGSheets.Formulas;

/// <summary>
/// Type-testing functions. These deliberately do <em>not</em> propagate an error argument: reporting
/// on a value is the whole point, so ISERROR(#N/A) has to answer TRUE rather than return #N/A.
/// </summary>
public static partial class BuiltinFunctions
{
    private static CellValue IsBlank(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => Predicate(arguments, context, value => value.IsEmpty);

    private static CellValue IsNumber(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => Predicate(arguments, context, value => value.Kind == CellValueKind.Number);

    private static CellValue IsText(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => Predicate(arguments, context, value => value.Kind == CellValueKind.Text);

    private static CellValue IsLogical(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => Predicate(arguments, context, value => value.Kind == CellValueKind.Boolean);

    private static CellValue IsError(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => Predicate(arguments, context, value => value.IsError);

    private static CellValue IsNa(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => Predicate(arguments, context, value => value.IsError && value.Error == CellError.NotAvailable);

    private static CellValue IsEven(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => Parity(arguments, context, even: true);

    private static CellValue IsOdd(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => Parity(arguments, context, even: false);

    private static CellValue Predicate(
        IReadOnlyList<IFormulaNode> arguments,
        IFormulaContext context,
        Func<CellValue, bool> test)
    {
        if (arguments.Count != 1)
        {
            return CellValue.FromError(CellError.Value);
        }

        return CellValue.FromBoolean(test(arguments[0].Evaluate(context)));
    }

    private static CellValue Parity(
        IReadOnlyList<IFormulaNode> arguments,
        IFormulaContext context,
        bool even)
    {
        if (arguments.Count != 1)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryNumber(arguments, 0, context, out var number, out var error))
        {
            return error;
        }

        return CellValue.FromBoolean(even ? double.IsEvenInteger(number) : double.IsOddInteger(number));
    }
}
