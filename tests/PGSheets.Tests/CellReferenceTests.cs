using PGSheets.Core;

namespace PGSheets.Tests;

public class CellReferenceTests
{
    [Theory]
    [InlineData(0, 0, false, false, "A1")]
    [InlineData(6, 1, false, false, "B7")]
    [InlineData(6, 1, true, true, "$B$7")]
    [InlineData(6, 1, true, false, "B$7")]
    [InlineData(6, 1, false, true, "$B7")]
    [InlineData(0, 26, false, false, "AA1")]
    public void Formats_with_anchors(
        int row,
        int column,
        bool absoluteRow,
        bool absoluteColumn,
        string expected)
    {
        Assert.Equal(expected, new CellReference(row, column, absoluteRow, absoluteColumn).ToA1());
    }

    [Fact]
    public void Relative_is_unanchored()
    {
        var reference = CellReference.Relative(2, 3);

        Assert.False(reference.AbsoluteRow);
        Assert.False(reference.AbsoluteColumn);
        Assert.Equal("D3", reference.ToA1());
    }

    [Fact]
    public void ToString_uses_a1_notation()
    {
        Assert.Equal("$B$7", new CellReference(6, 1, true, true).ToString());
    }

    [Fact]
    public void Anchored_parsing_reports_the_anchors()
    {
        Assert.True(CellAddress.TryParseA1(
            "$B$7",
            out var row,
            out var column,
            out var absoluteRow,
            out var absoluteColumn));

        Assert.Equal(6, row);
        Assert.Equal(1, column);
        Assert.True(absoluteRow);
        Assert.True(absoluteColumn);
    }

    [Fact]
    public void Partially_anchored_parsing_reports_one_anchor()
    {
        Assert.True(CellAddress.TryParseA1(
            "$B7",
            out _,
            out _,
            out var absoluteRow,
            out var absoluteColumn));

        Assert.False(absoluteRow);
        Assert.True(absoluteColumn);
    }

    [Fact]
    public void Unanchored_parsing_reports_no_anchors()
    {
        Assert.True(CellAddress.TryParseA1("b7", out _, out _, out var absoluteRow, out var absoluteColumn));

        Assert.False(absoluteRow);
        Assert.False(absoluteColumn);
    }

    [Theory]
    [InlineData("$$A1")]
    [InlineData("$")]
    [InlineData("A$")]
    public void Malformed_anchors_are_still_rejected(string text)
    {
        Assert.False(CellAddress.TryParseA1(text, out _, out _, out _, out _));
    }
}
