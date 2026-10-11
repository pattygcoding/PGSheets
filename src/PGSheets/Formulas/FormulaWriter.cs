using System.Globalization;
using System.Text;
using PGSheets.Core;

namespace PGSheets.Formulas;

/// <summary>
/// Rewrites the references of a formula as it is written out. Returning null means the reference no
/// longer exists, which becomes <c>#REF!</c>.
/// </summary>
public delegate CellReference? ReferenceRewriter(CellReference reference);

/// <summary>
/// Renders a parsed formula back to text. The output round-trips through <see cref="FormulaParser"/>,
/// so a rewritten formula means exactly what it says.
/// </summary>
public static class FormulaWriter
{
    // Precedence, lowest first.
    private const int ComparisonPrecedence = 1;
    private const int AdditivePrecedence = 2;
    private const int MultiplicativePrecedence = 3;
    private const int UnaryPrecedence = 4;
    private const int PowerPrecedence = 5;
    private const int PrimaryPrecedence = 6;

    /// <summary>
    /// Writes <paramref name="node"/> as a formula, including the leading '='. When
    /// <paramref name="rewrite"/> is supplied it is applied to every reference while writing, which
    /// is how a formula is adjusted for a copy or for an inserted row.
    /// </summary>
    public static string Write(IFormulaNode node, ReferenceRewriter? rewrite = null)
    {
        var builder = new StringBuilder();
        builder.Append('=');
        Append(node, builder, 0, rewrite);
        return builder.ToString();
    }

    /// <summary>
    /// Writes a node, wrapping it in parentheses when it binds less tightly than its position
    /// requires. Without this a rewrite could silently change what a formula means.
    /// </summary>
    private static void Append(
        IFormulaNode node,
        StringBuilder builder,
        int minimumPrecedence,
        ReferenceRewriter? rewrite)
    {
        var parenthesise = PrecedenceOf(node) < minimumPrecedence;

        if (parenthesise)
        {
            builder.Append('(');
        }

        switch (node)
        {
            case NumberNode number:
                // "R" round-trips the double exactly.
                builder.Append(number.Value.ToString("R", CultureInfo.InvariantCulture));
                break;

            case TextNode text:
                AppendText(builder, text.Value);
                break;

            case BooleanNode boolean:
                builder.Append(boolean.Value ? "TRUE" : "FALSE");
                break;

            case ErrorNode error:
                builder.Append(CellValue.ErrorText(error.Error));
                break;

            case CellRefNode cell:
                AppendReference(builder, Apply(cell.Reference, rewrite));
                break;

            case RangeRefNode range:
                AppendRange(builder, range, rewrite);
                break;

            case UnaryNode unary:
                builder.Append(unary.Operator == UnaryOperator.Negate ? '-' : '+');
                Append(unary.Operand, builder, UnaryPrecedence, rewrite);
                break;

            case BinaryNode binary:
                AppendBinary(builder, binary, rewrite);
                break;

            case ComparisonNode comparison:
                Append(comparison.Left, builder, ComparisonPrecedence, rewrite);
                builder.Append(SymbolOf(comparison.Operator));
                Append(comparison.Right, builder, ComparisonPrecedence + 1, rewrite);
                break;

            case FunctionNode function:
                AppendFunction(builder, function, rewrite);
                break;

            default:
                // A host-supplied node type the writer cannot render.
                throw new NotSupportedException($"Cannot write a {node.GetType().Name}.");
        }

        if (parenthesise)
        {
            builder.Append(')');
        }
    }

    private static void AppendBinary(StringBuilder builder, BinaryNode binary, ReferenceRewriter? rewrite)
    {
        var precedence = PrecedenceOf(binary);

        // Every operator except '^' is left associative, so the right operand must bind tighter or
        // it needs parentheses. '^' takes a bare signed exponent, as in 2^-1.
        var leftMinimum = binary.Operator == BinaryOperator.Power ? PowerPrecedence + 1 : precedence;
        var rightMinimum = binary.Operator == BinaryOperator.Power ? UnaryPrecedence : precedence + 1;

        Append(binary.Left, builder, leftMinimum, rewrite);
        builder.Append(SymbolOf(binary.Operator));
        Append(binary.Right, builder, rightMinimum, rewrite);
    }

    private static void AppendFunction(StringBuilder builder, FunctionNode function, ReferenceRewriter? rewrite)
    {
        builder.Append(function.Name);
        builder.Append('(');

        for (var index = 0; index < function.Arguments.Count; index++)
        {
            if (index > 0)
            {
                builder.Append(',');
            }

            // Arguments sit at the top level, so a comparison is written bare: IF(A1>2,..).
            Append(function.Arguments[index], builder, 0, rewrite);
        }

        builder.Append(')');
    }

    private static void AppendRange(StringBuilder builder, RangeRefNode range, ReferenceRewriter? rewrite)
    {
        var start = Apply(
            new CellReference(range.Row1, range.Column1, range.AbsoluteRow1, range.AbsoluteColumn1),
            rewrite);

        var end = Apply(
            new CellReference(range.Row2, range.Column2, range.AbsoluteRow2, range.AbsoluteColumn2),
            rewrite);

        // A range with a missing corner is written as #REF! rather than as a broken endpoint.
        if (start is null || end is null)
        {
            builder.Append(CellValue.ErrorText(CellError.Reference));
            return;
        }

        var first = start.Value;
        var second = end.Value;

        // Rewriting can invert the corners, and a range is always written top-left first.
        if (first.Row > second.Row || (first.Row == second.Row && first.Column > second.Column))
        {
            (first, second) = (second, first);
        }

        builder.Append(first.ToA1());
        builder.Append(':');
        builder.Append(second.ToA1());
    }

    private static void AppendReference(StringBuilder builder, CellReference? reference) =>
        builder.Append(reference is null ? CellValue.ErrorText(CellError.Reference) : reference.Value.ToA1());

    private static void AppendText(StringBuilder builder, string value)
    {
        builder.Append('"');

        foreach (var ch in value)
        {
            // A quote inside a string literal is doubled.
            if (ch == '"')
            {
                builder.Append('"');
            }

            builder.Append(ch);
        }

        builder.Append('"');
    }

    private static CellReference? Apply(CellReference reference, ReferenceRewriter? rewrite) =>
        rewrite is null ? reference : rewrite(reference);

    private static int PrecedenceOf(IFormulaNode node) => node switch
    {
        ComparisonNode => ComparisonPrecedence,
        BinaryNode binary => binary.Operator switch
        {
            BinaryOperator.Add or BinaryOperator.Subtract => AdditivePrecedence,
            BinaryOperator.Power => PowerPrecedence,
            _ => MultiplicativePrecedence,
        },
        UnaryNode => UnaryPrecedence,
        _ => PrimaryPrecedence,
    };

    private static string SymbolOf(BinaryOperator @operator) => @operator switch
    {
        BinaryOperator.Add => "+",
        BinaryOperator.Subtract => "-",
        BinaryOperator.Multiply => "*",
        BinaryOperator.Divide => "/",
        _ => "^",
    };

    private static string SymbolOf(ComparisonOperator @operator) => @operator switch
    {
        ComparisonOperator.Equal => "=",
        ComparisonOperator.NotEqual => "<>",
        ComparisonOperator.LessThan => "<",
        ComparisonOperator.LessThanOrEqual => "<=",
        ComparisonOperator.GreaterThan => ">",
        _ => ">=",
    };
}
