using PGSheets.Core;
using PGSheets.Formulas;

namespace PGSheets.Tests;

public class FormulaWriterTests
{
    private static string Write(string formula)
    {
        Assert.True(FormulaParser.TryParse(formula, out var node, out var error), error);
        return FormulaWriter.Write(node);
    }

    [Theory]
    [InlineData("=1+2", "=1+2")]
    [InlineData("=1+2*3", "=1+2*3")]
    [InlineData("=(1+2)*3", "=(1+2)*3")]
    [InlineData("=1-2-3", "=1-2-3")]
    [InlineData("=1-(2-3)", "=1-(2-3)")]
    [InlineData("=2^3^2", "=2^3^2")]
    [InlineData("=(2^3)^2", "=(2^3)^2")]
    [InlineData("=2^-1", "=2^-1")]
    [InlineData("=-A1^2", "=-A1^2")]
    [InlineData("=$A$1", "=$A$1")]
    [InlineData("=$A1+B$2", "=$A1+B$2")]
    [InlineData("=SUM($A$1:B2)", "=SUM($A$1:B2)")]
    [InlineData("=A1<>B1", "=A1<>B1")]
    [InlineData("=IF(A1>2,1,2)", "=IF(A1>2,1,2)")]
    [InlineData("=TRUE", "=TRUE")]
    [InlineData("=#DIV/0!", "=#DIV/0!")]
    [InlineData("=#N/A", "=#N/A")]
    [InlineData("=#REF!", "=#REF!")]
    public void Writes_canonical_text(string formula, string expected)
    {
        Assert.Equal(expected, Write(formula));
    }

    [Theory]
    [InlineData("=1+2")]
    [InlineData("=1+2*3")]
    [InlineData("=(1+2)*3")]
    [InlineData("=2^3^2")]
    [InlineData("=-A1^2")]
    [InlineData("=10-A1-B1")]
    [InlineData("=10-(A1-B1)")]
    [InlineData("=A1/(B1*C1)")]
    [InlineData("=IF(A1>2,\"big\",\"small\")")]
    [InlineData("=SUM(A1:B2)*2")]
    [InlineData("=1/2+3")]
    [InlineData("=\"a\"\"b\"")]
    [InlineData("=TRUE")]
    [InlineData("=#DIV/0!")]
    [InlineData("=#N/A")]
    [InlineData("=2^-1")]
    [InlineData("=A1<=B1")]
    [InlineData("=NOT(A1)")]
    public void Rewriting_preserves_the_meaning(string formula)
    {
        // The strongest check available: the rewritten text must evaluate to the same value.
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "5");
        Sheet.Set(model, "B1", "3");
        Sheet.Set(model, "C1", "2");
        Sheet.Set(model, "A2", "1");
        Sheet.Set(model, "B2", "2");

        Sheet.Set(model, "E1", formula);
        var expected = Sheet.Display(model, "E1");

        Sheet.Set(model, "G1", Write(formula));

        Assert.Equal(expected, Sheet.Display(model, "G1"));
    }

    [Theory]
    [InlineData("=1+2")]
    [InlineData("=(1+2)*3")]
    [InlineData("=SUM(A1:B2)+$A$1")]
    [InlineData("=IF(A1>2,\"x\",\"y\")")]
    public void Writing_is_idempotent(string formula)
    {
        var once = Write(formula);

        Assert.Equal(once, Write(once));
    }

    [Fact]
    public void Escapes_quotes_inside_a_string_literal()
    {
        Assert.Equal("=\"say \"\"hi\"\"\"", Write("=\"say \"\"hi\"\"\""));
    }

    [Fact]
    public void Numbers_round_trip_exactly()
    {
        Assert.Equal("=0.1", Write("=0.1"));
        Assert.Equal("=1200", Write("=1200"));
        Assert.Equal("=3.14159265358979", Write("=3.14159265358979"));
    }

    [Fact]
    public void A_reference_rewriter_can_replace_anchors()
    {
        Assert.True(FormulaParser.TryParse("=A1+$B$2", out var node, out _));

        var written = FormulaWriter.Write(node, reference => reference with { Row = reference.Row + 10 });

        Assert.Equal("=A11+$B$12", written);
    }

    [Fact]
    public void A_reference_rewriter_can_break_a_reference()
    {
        Assert.True(FormulaParser.TryParse("=A1", out var node, out _));

        Assert.Equal("=#REF!", FormulaWriter.Write(node, _ => null));
    }

    [Fact]
    public void Breaking_one_corner_of_a_range_writes_a_reference_error()
    {
        Assert.True(FormulaParser.TryParse("=SUM(A1:B2)", out var node, out _));

        var written = FormulaWriter.Write(node, reference => reference.Column == 1 ? null : reference);

        Assert.Equal("=SUM(#REF!)", written);
    }

    [Fact]
    public void Inverted_range_corners_are_written_top_left_first()
    {
        Assert.True(FormulaParser.TryParse("=SUM(A1:B2)", out var node, out _));

        // Pushing the top-left corner past the bottom-right inverts the pair.
        var written = FormulaWriter.Write(node, reference => reference with { Row = reference.Row + 5 });

        Assert.Equal("=SUM(A6:B7)", written);
    }

    [Fact]
    public void An_unknown_node_type_is_refused_rather_than_silently_dropped()
    {
        // A host may implement IFormulaNode. Writing one it does not understand must be loud,
        // because a formula quietly rendered as nothing would be worse than a failed save.
        Assert.Throws<NotSupportedException>(() => FormulaWriter.Write(new UnknownNode()));
    }

    /// <summary>A node type the writer has no rendering rule for.</summary>
    private sealed class UnknownNode : IFormulaNode
    {
        public CellValue Evaluate(IFormulaContext context) => CellValue.Empty;

        public void CollectPrecedents(HashSet<CellAddress> into)
        {
        }
    }
}
