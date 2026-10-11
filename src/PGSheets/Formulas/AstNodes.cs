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
    public ErrorNode(CellError error) => Error = error;

    public CellError Error { get; }

    public CellValue Evaluate(IFormulaContext context) => CellValue.FromError(Error);

    public void CollectPrecedents(HashSet<CellAddress> into)
    {
    }
}

public sealed class NumberNode : IFormulaNode
{
    public NumberNode(double value) => Value = value;

    public double Value { get; }

    public CellValue Evaluate(IFormulaContext context) => CellValue.FromNumber(Value);

    public void CollectPrecedents(HashSet<CellAddress> into)
    {
    }
}

public sealed class TextNode : IFormulaNode
{
    public TextNode(string value) => Value = value;

    public string Value { get; }

    public CellValue Evaluate(IFormulaContext context) => CellValue.FromText(Value);

    public void CollectPrecedents(HashSet<CellAddress> into)
    {
    }
}

public sealed class BooleanNode : IFormulaNode
{
    public BooleanNode(bool value) => Value = value;

    public bool Value { get; }

    public CellValue Evaluate(IFormulaContext context) => CellValue.FromBoolean(Value);

    public void CollectPrecedents(HashSet<CellAddress> into)
    {
    }
}

/// <summary>
/// A reference to one cell, carrying its <c>$</c> anchors so a copy can shift the relative parts and
/// leave the anchored ones alone.
/// </summary>
public sealed class CellRefNode : IFormulaNode
{
    public CellRefNode(int row, int column, bool absoluteRow = false, bool absoluteColumn = false) =>
        Reference = new CellReference(row, column, absoluteRow, absoluteColumn);

    public CellReference Reference { get; }

    public int Row => Reference.Row;

    public int Column => Reference.Column;

    public bool AbsoluteRow => Reference.AbsoluteRow;

    public bool AbsoluteColumn => Reference.AbsoluteColumn;

    public CellValue Evaluate(IFormulaContext context) => context.GetCellValue(Reference.Row, Reference.Column);

    public void CollectPrecedents(HashSet<CellAddress> into) =>
        into.Add(new CellAddress(Reference.Row, Reference.Column));
}

/// <summary>
/// A rectangular range such as A1:B5. A range is only meaningful as a function argument, so
/// evaluating one directly is an error; functions enumerate it instead. Each corner keeps its own
/// anchors, and normalising the corners carries the anchors with them.
/// </summary>
public sealed class RangeRefNode : IFormulaNode
{
    public RangeRefNode(
        int row1,
        int column1,
        int row2,
        int column2,
        bool absoluteRow1 = false,
        bool absoluteColumn1 = false,
        bool absoluteRow2 = false,
        bool absoluteColumn2 = false)
    {
        var rowsOrdered = row1 <= row2;
        var columnsOrdered = column1 <= column2;

        Row1 = rowsOrdered ? row1 : row2;
        Row2 = rowsOrdered ? row2 : row1;
        AbsoluteRow1 = rowsOrdered ? absoluteRow1 : absoluteRow2;
        AbsoluteRow2 = rowsOrdered ? absoluteRow2 : absoluteRow1;

        Column1 = columnsOrdered ? column1 : column2;
        Column2 = columnsOrdered ? column2 : column1;
        AbsoluteColumn1 = columnsOrdered ? absoluteColumn1 : absoluteColumn2;
        AbsoluteColumn2 = columnsOrdered ? absoluteColumn2 : absoluteColumn1;
    }

    public int Row1 { get; }

    public int Row2 { get; }

    public int Column1 { get; }

    public int Column2 { get; }

    public bool AbsoluteRow1 { get; }

    public bool AbsoluteRow2 { get; }

    public bool AbsoluteColumn1 { get; }

    public bool AbsoluteColumn2 { get; }

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
