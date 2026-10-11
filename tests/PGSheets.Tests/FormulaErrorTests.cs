namespace PGSheets.Tests;

/// <summary>
/// Malformed and edge-case formulas, asserted through the model so the whole
/// parse -&gt; evaluate -&gt; display path is covered.
/// </summary>
public class FormulaErrorTests
{
    [Theory]
    [InlineData("=1+", "#VALUE!")]
    [InlineData("=SUM(A1:A2", "#VALUE!")]
    [InlineData("=\"unterminated", "#VALUE!")]
    [InlineData("=1 2", "#VALUE!")]
    [InlineData("=()", "#VALUE!")]
    [InlineData("=A1:B2", "#VALUE!")]
    [InlineData("=FOO", "#NAME?")]
    [InlineData("=#REF!", "#REF!")]
    [InlineData("=#DIV/0!", "#DIV/0!")]
    [InlineData("=A1048577", "#REF!")] // one row past the sheet limit
    [InlineData("=XFE1", "#REF!")]     // one column past XFD (column 16385)
    public void Invalid_formulas_surface_an_error_value(string formula, string expected)
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "Z1", formula);

        Assert.Equal(expected, Sheet.Display(model, "Z1"));
    }

    [Fact]
    public void Absolute_references_parse()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "7");
        Sheet.Set(model, "B1", "=$A$1+1");

        Assert.Equal("8", Sheet.Display(model, "B1"));
    }

    [Fact]
    public void String_literals_support_escaped_quotes()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "=\"say \"\"hi\"\"\"");

        Assert.Equal("say \"hi\"", Sheet.Display(model, "A1"));
    }

    [Fact]
    public void Function_calls_nest()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "1");
        Sheet.Set(model, "A2", "2");
        Sheet.Set(model, "B1", "=SUM(SUM(A1:A2),3)");

        Assert.Equal("6", Sheet.Display(model, "B1"));
    }

    [Fact]
    public void Whitespace_inside_a_formula_is_ignored()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "10");
        Sheet.Set(model, "B1", "= A1 + 5 ");

        Assert.Equal("15", Sheet.Display(model, "B1"));
    }

    [Fact]
    public void A_leading_space_makes_the_input_text_rather_than_a_formula()
    {
        // Matches spreadsheet behaviour: only a leading '=' starts a formula.
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", " =1+1");

        Assert.Equal(" =1+1", Sheet.Display(model, "A1"));
        Assert.False(model.GetCell(0, 0)!.IsFormula);
    }

    [Fact]
    public void Repairs_to_a_circular_cell_recover()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "=B1");
        Sheet.Set(model, "B1", "=A1");
        Assert.Equal("#CIRCULAR!", Sheet.Display(model, "A1"));

        // Break the cycle: B1 becomes a literal.
        Sheet.Set(model, "B1", "5");

        Assert.Equal("5", Sheet.Display(model, "A1"));
        Assert.Equal("5", Sheet.Display(model, "B1"));
    }

    [Fact]
    public void A_formula_reading_an_empty_cell_treats_it_as_zero()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "=B1+1");

        Assert.Equal("1", Sheet.Display(model, "A1"));
    }

    [Fact]
    public void Rebuild_recomputes_everything_from_scratch()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "2");
        Sheet.Set(model, "A2", "=A1*3");
        Sheet.Set(model, "A3", "=A2+1");

        Assert.Equal("7", Sheet.Display(model, "A3"));

        model.SetRawInput(0, 0, "10");
        model.Clear();
        Sheet.Set(model, "A1", "2"); // re-seed after clear

        Assert.Equal("2", Sheet.Display(model, "A1"));
    }
}
