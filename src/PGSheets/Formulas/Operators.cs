using PGSheets.Core;

namespace PGSheets.Formulas;

public enum BinaryOperator
{
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
}

public enum UnaryOperator
{
    Negate,
}

public enum ComparisonOperator
{
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

/// <summary>
/// Comparison of two operands. Numbers compare numerically; anything else compares as text
/// (case-insensitive), which is what IF conditions need.
/// </summary>
public sealed class ComparisonNode : IFormulaNode
{
    private readonly ComparisonOperator _operator;
    private readonly IFormulaNode _left;
    private readonly IFormulaNode _right;

    public ComparisonNode(ComparisonOperator @operator, IFormulaNode left, IFormulaNode right)
    {
        _operator = @operator;
        _left = left;
        _right = right;
    }

    public CellValue Evaluate(IFormulaContext context)
    {
        var left = _left.Evaluate(context);
        if (left.IsError)
        {
            return left;
        }

        var right = _right.Evaluate(context);
        if (right.IsError)
        {
            return right;
        }

        var comparison = left.TryGetNumber(out var l) && right.TryGetNumber(out var r)
            ? l.CompareTo(r)
            : string.Compare(left.ToDisplayString(), right.ToDisplayString(), StringComparison.OrdinalIgnoreCase);

        var result = _operator switch
        {
            ComparisonOperator.Equal => comparison == 0,
            ComparisonOperator.NotEqual => comparison != 0,
            ComparisonOperator.LessThan => comparison < 0,
            ComparisonOperator.LessThanOrEqual => comparison <= 0,
            ComparisonOperator.GreaterThan => comparison > 0,
            _ => comparison >= 0,
        };

        return CellValue.FromBoolean(result);
    }

    public void CollectPrecedents(HashSet<CellAddress> into)
    {
        _left.CollectPrecedents(into);
        _right.CollectPrecedents(into);
    }
}


public sealed class UnaryNode : IFormulaNode
{
    private readonly UnaryOperator _operator;
    private readonly IFormulaNode _operand;

    public UnaryNode(UnaryOperator @operator, IFormulaNode operand)
    {
        _operator = @operator;
        _operand = operand;
    }

    public CellValue Evaluate(IFormulaContext context)
    {
        var value = _operand.Evaluate(context);
        if (value.IsError)
        {
            return value;
        }

        if (!value.TryGetNumber(out var number))
        {
            return CellValue.FromError(CellError.Value);
        }

        return _operator == UnaryOperator.Negate
            ? CellValue.FromNumber(-number)
            : CellValue.FromNumber(number);
    }

    public void CollectPrecedents(HashSet<CellAddress> into) => _operand.CollectPrecedents(into);
}

public sealed class BinaryNode : IFormulaNode
{
    private readonly BinaryOperator _operator;
    private readonly IFormulaNode _left;
    private readonly IFormulaNode _right;

    public BinaryNode(BinaryOperator @operator, IFormulaNode left, IFormulaNode right)
    {
        _operator = @operator;
        _left = left;
        _right = right;
    }

    public CellValue Evaluate(IFormulaContext context)
    {
        var left = _left.Evaluate(context);
        if (left.IsError)
        {
            return left;
        }

        var right = _right.Evaluate(context);
        if (right.IsError)
        {
            return right;
        }

        if (!left.TryGetNumber(out var l) || !right.TryGetNumber(out var r))
        {
            return CellValue.FromError(CellError.Value);
        }

        return _operator switch
        {
            BinaryOperator.Add => CellValue.FromNumber(l + r),
            BinaryOperator.Subtract => CellValue.FromNumber(l - r),
            BinaryOperator.Multiply => CellValue.FromNumber(l * r),
            BinaryOperator.Divide => r == 0d ? CellValue.FromError(CellError.DivideByZero) : CellValue.FromNumber(l / r),
            BinaryOperator.Power => CellValue.FromNumber(Math.Pow(l, r)),
            _ => CellValue.FromError(CellError.Value),
        };
    }

    public void CollectPrecedents(HashSet<CellAddress> into)
    {
        _left.CollectPrecedents(into);
        _right.CollectPrecedents(into);
    }
}

/// <summary>
/// A function call. Arguments stay as unevaluated nodes so a function can decide which branches to
/// evaluate (essential for IF, and for ranges, which must be enumerated rather than evaluated).
/// </summary>
public sealed class FunctionNode : IFormulaNode
{
    private readonly IFormulaNode[] _arguments;

    public FunctionNode(string name, IFormulaNode[] arguments)
    {
        Name = name;
        _arguments = arguments;
    }

    public string Name { get; }

    public IReadOnlyList<IFormulaNode> Arguments => _arguments;

    public CellValue Evaluate(IFormulaContext context)
    {
        if (!BuiltinFunctions.TryGet(Name, out var implementation))
        {
            return CellValue.FromError(CellError.Name);
        }

        try
        {
            return implementation(_arguments, context);
        }
        catch (FormulaException ex)
        {
            return CellValue.FromError(ex.Error);
        }
    }

    public void CollectPrecedents(HashSet<CellAddress> into)
    {
        foreach (var argument in _arguments)
        {
            argument.CollectPrecedents(into);
        }
    }
}
