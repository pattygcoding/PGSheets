using PGSheets.Core;

namespace PGSheets.Tests;

public class CellRangeTests
{
    [Fact]
    public void Normalises_any_two_corners()
    {
        var range = new CellRange(4, 3, 1, 0);

        Assert.Equal(1, range.Row1);
        Assert.Equal(4, range.Row2);
        Assert.Equal(0, range.Column1);
        Assert.Equal(3, range.Column2);
    }

    [Fact]
    public void Reports_its_shape()
    {
        var range = new CellRange(1, 1, 3, 4);

        Assert.Equal(3, range.RowCount);
        Assert.Equal(4, range.ColumnCount);
        Assert.Equal(12, range.CellCount);
        Assert.False(range.IsSingleCell);
    }

    [Fact]
    public void A_single_cell_is_its_own_range()
    {
        var range = new CellRange(2, 2, 2, 2);

        Assert.True(range.IsSingleCell);
        Assert.Equal(1, range.CellCount);
    }

    [Fact]
    public void Contains_only_cells_inside_it()
    {
        var range = new CellRange(1, 1, 2, 2);

        Assert.True(range.Contains(1, 1));
        Assert.True(range.Contains(2, 2));
        Assert.False(range.Contains(0, 1));
        Assert.False(range.Contains(3, 1));
        Assert.False(range.Contains(1, 3));
    }

    [Fact]
    public void Enumerate_visits_every_cell_row_by_row()
    {
        var cells = new CellRange(0, 0, 1, 1).Enumerate().ToArray();

        Assert.Equal(4, cells.Length);
        Assert.Equal(new CellAddress(0, 0), cells[0]);
        Assert.Equal(new CellAddress(0, 1), cells[1]);
        Assert.Equal(new CellAddress(1, 0), cells[2]);
        Assert.Equal(new CellAddress(1, 1), cells[3]);
    }

    [Fact]
    public void Offset_shifts_the_whole_range()
    {
        Assert.Equal(new CellRange(11, 6, 12, 7), new CellRange(1, 1, 2, 2).Offset(10, 5));
    }

    [Theory]
    [InlineData(0, 0, 0, 0, "A1")]
    [InlineData(1, 1, 3, 4, "B2:E4")]
    [InlineData(3, 4, 1, 1, "B2:E4")]
    public void Formats_as_a1_notation(int row1, int column1, int row2, int column2, string expected)
    {
        Assert.Equal(expected, new CellRange(row1, column1, row2, column2).ToString());
    }

    [Fact]
    public void Equality_is_by_corners()
    {
        Assert.Equal(new CellRange(1, 1, 2, 2), new CellRange(2, 2, 1, 1));
        Assert.NotEqual(new CellRange(1, 1, 2, 2), new CellRange(1, 1, 2, 3));
    }
}
