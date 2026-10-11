namespace PGSheets.Tests;

/// <summary>Statistics: the distribution functions and the multi-range aggregates.</summary>
public class BuiltinFunctionsStatisticsTests
{
    private static void Values(Core.SpreadsheetModel model)
    {
        Sheet.Set(model, "A1", "3");
        Sheet.Set(model, "A2", "1");
        Sheet.Set(model, "A3", "4");
        Sheet.Set(model, "A4", "1");
        Sheet.Set(model, "A5", "5");
    }

    [Theory]
    [InlineData("=MEDIAN(A1:A5)", "3")]              // the middle of 1,1,3,4,5
    [InlineData("=MEDIAN(A1:A4)", "2")]              // the mean of the two middle values
    [InlineData("=LARGE(A1:A5,1)", "5")]
    [InlineData("=LARGE(A1:A5,2)", "4")]
    [InlineData("=SMALL(A1:A5,1)", "1")]
    [InlineData("=SMALL(A1:A5,2)", "1")]
    [InlineData("=MODE(A1:A5)", "1")]                // the only repeated value
    [InlineData("=RANK(4,A1:A5)", "2")]              // descending: only 5 is ahead of it
    [InlineData("=RANK(1,A1:A5)", "4")]              // ties share a rank
    [InlineData("=RANK(4,A1:A5,1)", "4")]            // ascending order
    [InlineData("=COUNTBLANK(A1:A9)", "4")]
    [InlineData("=COUNTBLANK(A1:A5)", "0")]
    [InlineData("=SUMPRODUCT(A1:A5,A1:A5)", "52")]   // 9+1+16+1+25
    public void Return_their_result(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula, Values));
    }

    [Theory]
    [InlineData("=VAR(1,3)", "2")]
    [InlineData("=VAR(1,1,1)", "0")]
    [InlineData("=STDEV(1,1,1)", "0")]
    public void Sample_spread_is_measured_around_the_mean(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula));
    }

    [Theory]
    [InlineData("=MEDIAN(A7:A9)", "#NUM!")]          // nothing numeric to take a middle of
    [InlineData("=LARGE(A1:A5,6)", "#NUM!")]         // past the end of the sorted values
    [InlineData("=SMALL(A1:A5,0)", "#NUM!")]
    [InlineData("=MODE(A1:A1)", "#N/A")]             // no repeated value
    [InlineData("=RANK(9,A1:A5)", "#N/A")]           // the value is not in the list
    [InlineData("=STDEV(1)", "#DIV/0!")]             // one value has no spread
    [InlineData("=VAR(1)", "#DIV/0!")]
    [InlineData("=SUMPRODUCT(A1:A5,A1:A2)", "#VALUE!")] // mismatched shapes
    [InlineData("=SUMPRODUCT()", "#VALUE!")]
    [InlineData("=MEDIAN(\"abc\")", "#NUM!")]
    public void Report_bad_input(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula, Values));
    }

    [Fact]
    public void Median_ignores_text_in_a_range()
    {
        Assert.Equal(
            "2",
            Sheet.Evaluate("=MEDIAN(A1:A3)", model =>
            {
                Sheet.Set(model, "A1", "1");
                Sheet.Set(model, "A2", "text");
                Sheet.Set(model, "A3", "3");
            }));
    }
}
