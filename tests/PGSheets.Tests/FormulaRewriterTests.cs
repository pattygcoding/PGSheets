using PGSheets.Formulas;

namespace PGSheets.Tests;

public class FormulaRewriterTests
{
    [Theory]
    [InlineData("=A1", 1, 0, "=A2")]
    [InlineData("=A1", 0, 1, "=B1")]
    [InlineData("=A1+B2", 2, 3, "=D3+E4")]
    [InlineData("=$A$1", 5, 5, "=$A$1")]
    [InlineData("=A$1", 5, 5, "=F$1")]
    [InlineData("=$A1", 5, 5, "=$A6")]
    [InlineData("=SUM(A1:B2)", 1, 0, "=SUM(A2:B3)")]
    [InlineData("=SUM($A$1:$B$2)", 1, 1, "=SUM($A$1:$B$2)")]
    [InlineData("=A1*2", 3, 0, "=A4*2")]
    [InlineData("=A1", -1, 0, "=#REF!")]
    [InlineData("=A1", 0, -1, "=#REF!")]
    [InlineData("=IF(A1>0,A1,0)", 2, 0, "=IF(A3>0,A3,0)")]
    public void Translate_shifts_relative_references(
        string formula,
        int rowDelta,
        int columnDelta,
        string expected)
    {
        Assert.Equal(expected, FormulaRewriter.Translate(formula, rowDelta, columnDelta));
    }

    [Theory]
    [InlineData("plain text", 1, 1, "plain text")]
    [InlineData("=1+", 1, 1, "=1+")]
    [InlineData("", 1, 1, "")]
    public void Translate_leaves_anything_it_cannot_rewrite_alone(
        string formula,
        int rowDelta,
        int columnDelta,
        string expected)
    {
        Assert.Equal(expected, FormulaRewriter.Translate(formula, rowDelta, columnDelta));
    }

    [Fact]
    public void Translate_by_nothing_is_a_no_op()
    {
        Assert.Equal("=A1+$B$2", FormulaRewriter.Translate("=A1+$B$2", 0, 0));
    }

    [Theory]
    [InlineData("=A5", 3, 1, true, "=A6")]
    [InlineData("=A3", 3, 1, true, "=A3")]
    [InlineData("=$A$5", 3, 1, true, "=$A$6")]
    [InlineData("=A5", 3, 1, false, "=A4")]
    [InlineData("=A4", 3, 1, false, "=#REF!")]
    [InlineData("=A2", 3, 1, false, "=A2")]
    [InlineData("=SUM(A5:A8)", 3, 2, false, "=SUM(#REF!)")]
    [InlineData("=SUM(A5:A8)", 1, 1, false, "=SUM(A4:A7)")]
    [InlineData("=SUM(A1:A3)", 1, 1, false, "=SUM(A1:A2)")]
    [InlineData("=SUM(A1:A10)", 4, 2, false, "=SUM(A1:A8)")]
    public void Adjust_for_rows(string formula, int at, int count, bool inserted, string expected)
    {
        Assert.Equal(expected, FormulaRewriter.AdjustForRows(formula, at, count, inserted));
    }

    [Theory]
    [InlineData("=C1", 1, 1, true, "=D1")]
    [InlineData("=A1", 1, 1, true, "=A1")]
    [InlineData("=B1", 1, 1, false, "=#REF!")]
    [InlineData("=C1", 1, 1, false, "=B1")]
    [InlineData("=$C$1", 1, 1, true, "=$D$1")]
    public void Adjust_for_columns(string formula, int at, int count, bool inserted, string expected)
    {
        Assert.Equal(expected, FormulaRewriter.AdjustForColumns(formula, at, count, inserted));
    }

    [Fact]
    public void Inserting_does_not_protect_anchored_references()
    {
        // Anchors only survive a copy; a spreadsheet still shifts them when a row is inserted.
        Assert.Equal("=$A$9", FormulaRewriter.AdjustForRows("=$A$5", 0, 4, inserted: true));
    }
}
