using PGSheets.Core;

namespace PGSheets.Tests;

public class FormulaEngineTests
{
    [Fact]
    public void Literals_are_typed_by_content()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "1200");
        Sheet.Set(model, "A2", "North");
        Sheet.Set(model, "A3", "TRUE");

        Assert.Equal(CellValueKind.Number, model.GetValue(0, 0).Kind);
        Assert.Equal(CellValueKind.Text, model.GetValue(1, 0).Kind);
        Assert.Equal(CellValueKind.Boolean, model.GetValue(2, 0).Kind);
        Assert.Equal("North", Sheet.Display(model, "A2"));
    }

    [Fact]
    public void Sum_over_a_range()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "1");
        Sheet.Set(model, "A2", "2");
        Sheet.Set(model, "A3", "3");
        Sheet.Set(model, "A5", "=SUM(A1:A3)");

        Assert.Equal("6", Sheet.Display(model, "A5"));
    }

    [Fact]
    public void Sum_ignores_text_and_empty_cells()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "10");
        Sheet.Set(model, "A2", "note");
        Sheet.Set(model, "A4", "=SUM(A1:A3)");

        Assert.Equal("10", Sheet.Display(model, "A4"));
    }

    [Theory]
    [InlineData("=2+3*4", "14")]
    [InlineData("=(2+3)*4", "20")]
    [InlineData("=10-2-3", "5")]
    [InlineData("=2^3^2", "512")]
    [InlineData("=-2^2", "-4")]
    [InlineData("=8/2/2", "2")]
    public void Arithmetic_follows_spreadsheet_precedence(string formula, string expected)
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", formula);

        Assert.Equal(expected, Sheet.Display(model, "A1"));
    }

    [Fact]
    public void Edits_propagate_to_dependants()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "10");
        Sheet.Set(model, "B1", "=A1*2");
        Sheet.Set(model, "C1", "=B1+5");

        Assert.Equal("25", Sheet.Display(model, "C1"));

        Sheet.Set(model, "A1", "20");

        Assert.Equal("40", Sheet.Display(model, "B1"));
        Assert.Equal("45", Sheet.Display(model, "C1"));
    }

    [Fact]
    public void Diamond_dependencies_recalculate()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "3");
        Sheet.Set(model, "B1", "=A1+1");
        Sheet.Set(model, "C1", "=A1+2");
        Sheet.Set(model, "D1", "=B1*C1");

        Assert.Equal("20", Sheet.Display(model, "D1"));

        Sheet.Set(model, "A1", "4");

        Assert.Equal("30", Sheet.Display(model, "D1"));
    }

    [Fact]
    public void Circular_references_are_reported_not_hung()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "=B1");
        Sheet.Set(model, "B1", "=A1");

        Assert.Equal("#CIRCULAR!", Sheet.Display(model, "A1"));
        Assert.Equal("#CIRCULAR!", Sheet.Display(model, "B1"));
    }

    [Fact]
    public void Self_reference_is_circular()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "=A1+1");

        Assert.Equal("#CIRCULAR!", Sheet.Display(model, "A1"));
    }
}
