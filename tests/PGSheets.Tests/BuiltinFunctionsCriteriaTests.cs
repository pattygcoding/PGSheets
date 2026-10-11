namespace PGSheets.Tests;

/// <summary>
/// Statistics and the conditional aggregates, including how a criterion string such as "&gt;=10" or
/// "a*" is interpreted.
/// </summary>
public class BuiltinFunctionsCriteriaTests
{
    private static void Numbers(Core.SpreadsheetModel model)
    {
        Sheet.Set(model, "A1", "1");
        Sheet.Set(model, "A2", "2");
        Sheet.Set(model, "A3", "3");
        Sheet.Set(model, "A4", "4");

        Sheet.Set(model, "B1", "10");
        Sheet.Set(model, "B2", "20");
        Sheet.Set(model, "B3", "30");
        Sheet.Set(model, "B4", "40");
    }

    private static void Words(Core.SpreadsheetModel model)
    {
        Sheet.Set(model, "A1", "apple");
        Sheet.Set(model, "A2", "apricot");
        Sheet.Set(model, "A3", "banana");
    }

    [Theory]
    [InlineData("=COUNTIF(A1:A4,\">2\")", "2")]
    [InlineData("=COUNTIF(A1:A4,\">=2\")", "3")]
    [InlineData("=COUNTIF(A1:A4,\"<3\")", "2")]
    [InlineData("=COUNTIF(A1:A4,\"2\")", "1")]
    [InlineData("=COUNTIF(A1:A4,2)", "1")]          // a bare number means equality
    [InlineData("=COUNTIF(A1:A4,\"<>2\")", "3")]
    [InlineData("=COUNTIF(A1:A4,\"=2\")", "1")]
    [InlineData("=COUNTIF(A1:A4,\">999\")", "0")]
    [InlineData("=SUMIF(A1:A4,\">2\")", "7")]
    [InlineData("=SUMIF(A1:A4,\"<=2\")", "3")]
    [InlineData("=SUMIF(A1:A4,\">2\",B1:B4)", "70")]
    [InlineData("=SUMIF(A1:A4,\">2\",B1)", "70")]   // a single anchor cell, walked with A's shape
    [InlineData("=AVERAGEIF(A1:A4,\">2\")", "3.5")]
    public void Conditional_aggregates_count_and_add_what_matches(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula, Numbers));
    }

    [Theory]
    [InlineData("=COUNTIF(A1:A3,\"a*\")", "2")]
    [InlineData("=COUNTIF(A1:A3,\"*an*\")", "1")]
    [InlineData("=COUNTIF(A1:A3,\"apple\")", "1")]
    [InlineData("=COUNTIF(A1:A3,\"APPLE\")", "1")]  // text criteria ignore case
    [InlineData("=COUNTIF(A1:A3,\"?????\")", "1")]  // ? is exactly one character
    [InlineData("=COUNTIF(A1:A3,\"<>apple\")", "2")]
    public void Text_criteria_use_wildcards(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula, Words));
    }

    [Fact]
    public void A_tilde_escapes_a_wildcard_character()
    {
        Assert.Equal(
            "1",
            Sheet.Evaluate("=COUNTIF(A1:A2,\"~*\")", model =>
            {
                Sheet.Set(model, "A1", "*");
                Sheet.Set(model, "A2", "x");
            }));
    }

    [Fact]
    public void An_empty_cell_counts_as_zero_for_a_numeric_criterion()
    {
        // COUNTIF treats a blank as 0, which is why "=0" finds the empty cell.
        Assert.Equal(
            "1",
            Sheet.Evaluate("=COUNTIF(A1:A2,\"=0\")", model => Sheet.Set(model, "A1", "5")));
    }

    [Fact]
    public void Averageif_with_no_match_divides_by_zero()
    {
        Assert.Equal("#DIV/0!", Sheet.Evaluate("=AVERAGEIF(A1:A4,\">9\")", Numbers));
    }

    [Theory]
    [InlineData("=COUNTIF(A1:A4)", "#VALUE!")]       // the criteria are required
    [InlineData("=SUMIF(A1:A4,\">2\",B1:B4,1)", "#VALUE!")]
    [InlineData("=COUNTIF(1,1)", "1")]               // a single value behaves as a one-cell range
    public void Report_bad_input(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula, Numbers));
    }
}
