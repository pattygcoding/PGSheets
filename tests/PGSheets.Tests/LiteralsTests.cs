using PGSheets.Core;

namespace PGSheets.Tests;

public class LiteralsTests
{
    [Fact]
    public void Empty_input_is_empty()
    {
        Assert.True(Literals.Parse(string.Empty).IsEmpty);
        Assert.True(Literals.Parse(null!).IsEmpty);
    }

    [Theory]
    [InlineData("1200", 1200d)]
    [InlineData("-3.5", -3.5d)]
    [InlineData("1e3", 1000d)]
    [InlineData("1,234", 1234d)]
    public void Numeric_input_is_parsed_with_the_invariant_culture(string raw, double expected)
    {
        var value = Literals.Parse(raw);

        Assert.Equal(CellValueKind.Number, value.Kind);
        Assert.Equal(expected, value.Number, 9);
    }

    [Theory]
    [InlineData("TRUE")]
    [InlineData("true")]
    [InlineData("True")]
    public void Booleans_are_case_insensitive(string raw)
    {
        var value = Literals.Parse(raw);

        Assert.Equal(CellValueKind.Boolean, value.Kind);
        Assert.True(value.Boolean);
    }

    [Fact]
    public void Everything_else_is_text()
    {
        Assert.Equal(CellValueKind.Text, Literals.Parse("North").Kind);
    }

    [Fact]
    public void A_stray_formula_is_preserved_as_text()
    {
        // Should never reach here through the engine, but must not throw or lose data.
        var value = Literals.Parse("=1+1");

        Assert.Equal(CellValueKind.Text, value.Kind);
        Assert.Equal("=1+1", value.Text);
    }
}
