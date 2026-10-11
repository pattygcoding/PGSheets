namespace PGSheets.Tests;

/// <summary>
/// The built-in function library. Each case is asserted through the model so the whole
/// parse -&gt; evaluate -&gt; display path is exercised.
/// </summary>
public class BuiltinFunctionsTests
{
    private static string Evaluate(string formula, Action<Core.SpreadsheetModel>? seed = null)
    {
        var model = Sheet.CreateModel();
        seed?.Invoke(model);
        Sheet.Set(model, "Z1", formula);

        return Sheet.Display(model, "Z1");
    }

    [Theory]
    [InlineData("=ABS(-5)", "5")]
    [InlineData("=ABS(5)", "5")]
    [InlineData("=SQRT(9)", "3")]
    [InlineData("=POWER(2,10)", "1024")]
    [InlineData("=ROUND(3.14159,2)", "3.14")]
    [InlineData("=ROUND(2.5,0)", "3")]
    [InlineData("=ROUND(2.4)", "2")]
    public void Math_functions_return_their_result(string formula, string expected)
    {
        Assert.Equal(expected, Evaluate(formula));
    }

    [Theory]
    [InlineData("=SQRT(-1)", "#NUM!")]     // no real root
    [InlineData("=ABS(\"abc\")", "#VALUE!")]  // non-numeric argument
    [InlineData("=ABS(1,2)", "#VALUE!")]     // wrong arity
    [InlineData("=POWER(2)", "#VALUE!")]     // wrong arity
    [InlineData("=ROUND()", "#VALUE!")]      // wrong arity
    public void Math_functions_report_bad_input(string formula, string expected)
    {
        Assert.Equal(expected, Evaluate(formula));
    }

    [Fact]
    public void Concat_joins_text_numbers_and_ranges()
    {
        Assert.Equal("ab1", Evaluate("=CONCAT(\"a\",\"b\",1)"));

        Assert.Equal(
            "ab",
            Evaluate("=CONCAT(A1:A2)", model =>
            {
                Sheet.Set(model, "A1", "a");
                Sheet.Set(model, "A2", "b");
            }));
    }

    [Fact]
    public void Counta_counts_non_empty_cells()
    {
        Assert.Equal(
            "2",
            Evaluate("=COUNTA(A1:A3)", model =>
            {
                Sheet.Set(model, "A1", "1");
                Sheet.Set(model, "A3", "text");
            }));
    }

    [Fact]
    public void Average_of_text_only_reports_division_by_zero()
    {
        Assert.Equal(
            "#DIV/0!",
            Evaluate("=AVERAGE(A1:A2)", model =>
            {
                Sheet.Set(model, "A1", "x");
                Sheet.Set(model, "A2", "y");
            }));
    }

    [Fact]
    public void Min_and_max_of_text_only_are_zero()
    {
        Action<Core.SpreadsheetModel> seed = model =>
        {
            Sheet.Set(model, "A1", "x");
            Sheet.Set(model, "A2", "y");
        };

        Assert.Equal("0", Evaluate("=MIN(A1:A2)", seed));
        Assert.Equal("0", Evaluate("=MAX(A1:A2)", seed));
    }

    [Fact]
    public void If_with_too_few_arguments_is_an_error()
    {
        Assert.Equal("#VALUE!", Evaluate("=IF(TRUE)"));
    }

    [Fact]
    public void If_without_a_false_branch_returns_false()
    {
        Assert.Equal("FALSE", Evaluate("=IF(FALSE,1)"));
    }

    [Fact]
    public void Aggregates_propagate_an_error_from_a_range()
    {
        Assert.Equal(
            "#DIV/0!",
            Evaluate("=SUM(A1:A2)", model =>
            {
                Sheet.Set(model, "A1", "=1/0");
                Sheet.Set(model, "A2", "1");
            }));
    }

    [Fact]
    public void A_registered_function_replaces_the_previous_definition()
    {
        Formulas.BuiltinFunctions.Register("ALWAYS", (_, _) => Core.CellValue.FromNumber(1d));
        Assert.Equal("1", Evaluate("=ALWAYS()"));

        Formulas.BuiltinFunctions.Register("ALWAYS", (_, _) => Core.CellValue.FromNumber(2d));
        Assert.Equal("2", Evaluate("=ALWAYS()"));
    }

    [Fact]
    public void The_registry_exposes_its_function_names()
    {
        Assert.Contains("SUM", Formulas.BuiltinFunctions.Names);
        Assert.True(Formulas.BuiltinFunctions.IsKnown("sum"));
        Assert.False(Formulas.BuiltinFunctions.IsKnown("NOPE"));
    }
}
