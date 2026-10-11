using PGSheets.Core;

namespace PGSheets.Tests;

public class CellAddressTests
{
    [Theory]
    [InlineData("A1", 0, 0)]
    [InlineData("B7", 6, 1)]
    [InlineData("AA1", 0, 26)]
    [InlineData("$B$7", 6, 1)]
    [InlineData("b7", 6, 1)]
    [InlineData("XFD1048576", SheetBounds.MaxRows - 1, SheetBounds.MaxColumns - 1)]
    public void Parses_a1_notation(string text, int expectedRow, int expectedColumn)
    {
        Assert.True(CellAddress.TryParseA1(text, out var row, out var column));
        Assert.Equal(expectedRow, row);
        Assert.Equal(expectedColumn, column);
    }

    [Theory]
    [InlineData("")]
    [InlineData(null)]
    [InlineData("A")]
    [InlineData("1")]
    [InlineData("1A")]
    [InlineData("A0")]
    [InlineData("A1B")]
    [InlineData("ABCD1")] // four letters is not a valid column
    [InlineData("A-1")]
    [InlineData("$")]
    [InlineData("$$A1")]
    public void Rejects_malformed_notation(string? text)
    {
        Assert.False(CellAddress.TryParseA1(text, out _, out _));
    }

    [Fact]
    public void Formats_a1_notation()
    {
        Assert.Equal("A1", CellAddress.ToA1(0, 0));
        Assert.Equal("B7", CellAddress.ToA1(6, 1));
        Assert.Equal("AA1", CellAddress.ToA1(0, 26));
    }

    [Fact]
    public void ToString_uses_a1_notation()
    {
        Assert.Equal("C4", new CellAddress(3, 2).ToString());
    }

    [Fact]
    public void Equality_is_by_coordinate()
    {
        Assert.Equal(new CellAddress(1, 2), new CellAddress(1, 2));
        Assert.NotEqual(new CellAddress(1, 2), new CellAddress(2, 1));
        Assert.Equal(new CellAddress(1, 2).GetHashCode(), new CellAddress(1, 2).GetHashCode());
    }

    [Fact]
    public void Negative_coordinates_are_rejected_when_formatting()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => CellAddress.ToColumnName(-1));
        Assert.Throws<ArgumentOutOfRangeException>(() => CellAddress.ToA1(-1, 0));
    }
}
