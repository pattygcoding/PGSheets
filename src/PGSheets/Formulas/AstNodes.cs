using PGSheets.Core;

namespace PGSheets.Formulas;

/// <summary>Read access to other cells while a formula is being evaluated.</summary>
public interface IFormulaContext
{
    CellValue GetCellValue(int row, int column);
}

/// <summary>A parsed formula expression node.</summary>
public interface IFormulaNode
{
    CellValue Evaluate(IFormulaContext context);

    /// <summary>Collects the cells this node reads, for dependency tracking.</summary>
    void CollectPrecedents(HashSet<CellAddress> into);
}

/// <summary>A literal that always evaluates to an error (e.g. an out-of-range reference).</summary>
public sealed class ErrorNode : IFormulaNode
{
    private readonly CellError _error;

    public ErrorNode(CellError error) => _error = error;

    public CellValue Evaluate(IFormulaContext context) => CellValue.FromError(_error);

    public void CollectPrecedents(HashSet<CellAddress> into)
    {
    }
}

public sealed class NumberNode : IFormulaNode
{
    private readonly double _value;

    public NumberNode(double value) => _value = value;

    public CellValue Evaluate(IFormulaContext context) => CellValue.FromNumber(_value);

    public void CollectPrecedents(HashSet<CellAddress> into)
    {
    }
}

public sealed class TextNode : IFormulaNode
{
    private readonly string _value;

    public TextNode(string value) => _value = value;

    public CellValue Evaluate(IFormulaContext context) => CellValue.FromText(_value);

    public void CollectPrecedents(HashSet<CellAddress> into)
    {
    }
}

public sealed class BooleanNode : IFormulaNode
{
    private readonly bool _value;

    public BooleanNode(bool value) => _value = value;

    public CellValue Evaluate(IFormulaContext context) => CellValue.FromBoolean(_value);

    public void CollectPrecedents(HashSet<CellAddress> into)
    {
    }
}

public sealed class CellRefNode : IFormulaNode
{
    public CellRefNode(int row, int column)
    {
        Row = row;
        Column = column;
    }

    public int Row { get; }

    public int Column { get; }

    public CellValue Evaluate(IFormulaContext context) => context.GetCellValue(Row, Column);

    public void CollectPrecedents(HashSet<CellAddress> into) => into.Add(new CellAddress(Row, Column));
}

/// <summary>
/// A rectangular range such as A1:B5. A range is only meaningful as a function argument, so
/// evaluating one directly is an error; functions enumerate it instead.
/// </summary>
public sealed class RangeRefNode : IFormulaNode
{
    public RangeRefNode(int row1, int column1, int row2, int column2)
    {
        Row1 = Math.Min(row1, row2);
        Row2 = Math.Max(row1, row2);
        Column1 = Math.Min(column1, column2);
        Column2 = Math.Max(column1, column2);
    }

    public int Row1 { get; }

    public int Row2 { get; }

    public int Column1 { get; }

    public int Column2 { get; }

    public CellValue Evaluate(IFormulaContext context) => CellValue.FromError(CellError.Value);

    public void CollectPrecedents(HashSet<CellAddress> into)
    {
        for (var row = Row1; row <= Row2; row++)
        {
            for (var column = Column1; column <= Column2; column++)
            {
                into.Add(new CellAddress(row, column));
            }
        }
    }

    public IEnumerable<CellValue> Enumerate(IFormulaContext context)
    {
        for (var row = Row1; row <= Row2; row++)
        {
            for (var column = Column1; column <= Column2; column++)
            {
                yield return context.GetCellValue(row, column);
            }
        }
    }
}
