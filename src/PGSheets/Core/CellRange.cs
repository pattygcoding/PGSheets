namespace PGSheets.Core;

/// <summary>
/// A rectangular block of cells. Always normalised so <see cref="Row1"/> &lt;= <see cref="Row2"/>
/// and <see cref="Column1"/> &lt;= <see cref="Column2"/>; a single cell has equal corners.
/// </summary>
public readonly record struct CellRange
{
    /// <summary>Creates a normalised range from any two corners, given in any order.</summary>
    public CellRange(int row1, int column1, int row2, int column2)
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

    public int RowCount => Row2 - Row1 + 1;

    public int ColumnCount => Column2 - Column1 + 1;

    public int CellCount => RowCount * ColumnCount;

    public bool IsSingleCell => Row1 == Row2 && Column1 == Column2;

    public bool Contains(int row, int column) =>
        row >= Row1 && row <= Row2 && column >= Column1 && column <= Column2;

    /// <summary>Every cell in the range, row by row.</summary>
    public IEnumerable<CellAddress> Enumerate()
    {
        for (var row = Row1; row <= Row2; row++)
        {
            for (var column = Column1; column <= Column2; column++)
            {
                yield return new CellAddress(row, column);
            }
        }
    }

    /// <summary>Shifts the whole range by a delta, for tiling a paste across a larger selection.</summary>
    public CellRange Offset(int rowDelta, int columnDelta) =>
        new(Row1 + rowDelta, Column1 + columnDelta, Row2 + rowDelta, Column2 + columnDelta);

    /// <summary>"A1" for a single cell, "B2:D5" for a block.</summary>
    public override string ToString() =>
        IsSingleCell
            ? CellAddress.ToA1(Row1, Column1)
            : $"{CellAddress.ToA1(Row1, Column1)}:{CellAddress.ToA1(Row2, Column2)}";
}
