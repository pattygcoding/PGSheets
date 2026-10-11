namespace PGSheets.Tests;

/// <summary>Logic and type tests, including the conditional functions that must stay lazy.</summary>
public class BuiltinFunctionsLogicalTests
{
    [Theory]
    [InlineData("=IFERROR(1/0,\"safe\")", "safe")]
    [InlineData("=IFERROR(5,\"safe\")", "5")]
    [InlineData("=IFERROR(1/0,1/0)", "#DIV/0!")]     // a failing fallback still reports
    [InlineData("=IFNA(NA(),\"missing\")", "missing")]
    [InlineData("=IFNA(1/0,\"missing\")", "#DIV/0!")] // IFNA catches only #N/A
    [InlineData("=IFNA(7,\"missing\")", "7")]
    [InlineData("=AND(TRUE,TRUE)", "TRUE")]
    [InlineData("=AND(TRUE,FALSE)", "FALSE")]
    [InlineData("=OR(FALSE,TRUE)", "TRUE")]
    [InlineData("=OR(FALSE,FALSE)", "FALSE")]
    [InlineData("=XOR(TRUE,TRUE)", "FALSE")]
    [InlineData("=XOR(TRUE,TRUE,TRUE)", "TRUE")]      // XOR is a parity test
    [InlineData("=NOT(TRUE)", "FALSE")]
    [InlineData("=NOT(0)", "TRUE")]
    [InlineData("=IFS(1>2,\"a\",1<2,\"b\")", "b")]
    [InlineData("=IFS(1>2,\"a\")", "#N/A")]           // nothing matched
    [InlineData("=SWITCH(2,1,\"a\",2,\"b\")", "b")]
    [InlineData("=SWITCH(9,1,\"a\",2,\"b\")", "#N/A")]
    [InlineData("=SWITCH(9,1,\"a\",2,\"b\",\"other\")", "other")]
    [InlineData("=SWITCH(\"B\",\"a\",1,\"b\",2)", "2")] // text comparisons ignore case
    [InlineData("=TRUE()", "TRUE")]
    [InlineData("=FALSE()", "FALSE")]
    [InlineData("=NA()", "#N/A")]
    public void Return_their_result(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula));
    }

    [Fact]
    public void Logic_skips_text_and_blank_cells_in_a_range()
    {
        // A spreadsheet ignores text inside a range rather than counting it as false.
        Assert.Equal(
            "TRUE",
            Sheet.Evaluate("=AND(A1:A3)", model =>
            {
                Sheet.Set(model, "A1", "1");
                Sheet.Set(model, "A2", "text");
                Sheet.Set(model, "A3", "TRUE");
            }));
    }

    [Fact]
    public void Logic_with_nothing_to_test_is_an_error()
    {
        Assert.Equal("#VALUE!", Sheet.Evaluate("=AND()"));
        Assert.Equal(
            "#VALUE!",
            Sheet.Evaluate("=OR(A1:A2)", model =>
            {
                Sheet.Set(model, "A1", "text");
                Sheet.Set(model, "A2", "more text");
            }));
    }

    [Theory]
    [InlineData("=TRUE(1)", "#VALUE!")]
    [InlineData("=IFS(1>2,\"a\",1>3)", "#VALUE!")]   // a condition with no result
    [InlineData("=IFERROR(1)", "#VALUE!")]
    [InlineData("=SWITCH(1,2)", "#VALUE!")]
    [InlineData("=NOT()", "#VALUE!")]
    public void Report_bad_input(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula));
    }

    [Theory]
    [InlineData("=ISNUMBER(1)", "TRUE")]
    [InlineData("=ISNUMBER(\"1\")", "FALSE")]   // text that looks numeric is still text
    [InlineData("=ISTEXT(\"a\")", "TRUE")]
    [InlineData("=ISTEXT(1)", "FALSE")]
    [InlineData("=ISLOGICAL(TRUE)", "TRUE")]
    [InlineData("=ISLOGICAL(1)", "FALSE")]
    [InlineData("=ISERROR(1/0)", "TRUE")]
    [InlineData("=ISERROR(1)", "FALSE")]
    [InlineData("=ISERROR(NA())", "TRUE")]
    [InlineData("=ISNA(NA())", "TRUE")]
    [InlineData("=ISNA(1/0)", "FALSE")]         // only #N/A counts
    [InlineData("=ISEVEN(4)", "TRUE")]
    [InlineData("=ISEVEN(3)", "FALSE")]
    [InlineData("=ISODD(3)", "TRUE")]
    public void Type_tests_answer_true_or_false(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula));
    }

    [Fact]
    public void Isblank_tells_an_empty_cell_from_a_full_one()
    {
        Assert.Equal("TRUE", Sheet.Evaluate("=ISBLANK(A1)"));
        Assert.Equal("FALSE", Sheet.Evaluate("=ISBLANK(A1)", model => Sheet.Set(model, "A1", "1")));
    }

    [Fact]
    public void Type_tests_report_bad_input()
    {
        Assert.Equal("#VALUE!", Sheet.Evaluate("=ISEVEN(\"abc\")"));
        Assert.Equal("#VALUE!", Sheet.Evaluate("=ISBLANK(1,2)"));
    }
}
