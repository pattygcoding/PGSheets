namespace PGSheets.Tests;

/// <summary>The numeric half of the function library, beyond the arithmetic originals.</summary>
public class BuiltinFunctionsMathTests
{
    [Theory]
    [InlineData("=INT(2.7)", "2")]
    [InlineData("=INT(-2.1)", "-3")]            // towards negative infinity, not towards zero
    [InlineData("=TRUNC(2.9)", "2")]
    [InlineData("=TRUNC(-2.9)", "-2")]          // towards zero
    [InlineData("=TRUNC(2.987,2)", "2.98")]
    [InlineData("=ROUNDUP(2.1,0)", "3")]
    [InlineData("=ROUNDUP(-2.1,0)", "-3")]      // away from zero
    [InlineData("=ROUNDUP(1.234,2)", "1.24")]
    [InlineData("=ROUNDDOWN(2.9,0)", "2")]
    [InlineData("=ROUNDDOWN(-2.9,0)", "-2")]    // towards zero
    [InlineData("=ROUNDDOWN(1.239,2)", "1.23")]
    [InlineData("=MOD(7,3)", "1")]
    [InlineData("=MOD(-3,5)", "2")]             // follows the divisor's sign, unlike C#'s '%'
    [InlineData("=MOD(3,-5)", "-2")]
    [InlineData("=SIGN(-4)", "-1")]
    [InlineData("=SIGN(0)", "0")]
    [InlineData("=SIGN(9)", "1")]
    [InlineData("=CEILING(2.3)", "3")]
    [InlineData("=CEILING(2.3,0.5)", "2.5")]
    [InlineData("=CEILING(-2.3,1)", "-2")]      // up is towards positive infinity
    [InlineData("=FLOOR(2.7)", "2")]
    [InlineData("=FLOOR(2.7,0.5)", "2.5")]
    [InlineData("=FLOOR(-2.3,1)", "-3")]        // down is towards negative infinity
    [InlineData("=PI()", "3.14159265358979")]
    [InlineData("=EXP(0)", "1")]
    [InlineData("=LN(1)", "0")]
    [InlineData("=LOG10(1000)", "3")]
    [InlineData("=LOG(8,2)", "3")]
    [InlineData("=PRODUCT(2,3,4)", "24")]
    public void Return_their_result(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula));
    }

    [Fact]
    public void Product_reads_a_range_and_ignores_text()
    {
        Assert.Equal(
            "24",
            Sheet.Evaluate("=PRODUCT(A1:A4)", model =>
            {
                Sheet.Set(model, "A1", "2");
                Sheet.Set(model, "A2", "3");
                Sheet.Set(model, "A3", "text");
                Sheet.Set(model, "A4", "4");
            }));
    }

    [Theory]
    [InlineData("=MOD(1,0)", "#DIV/0!")]
    [InlineData("=LN(0)", "#NUM!")]
    [InlineData("=LOG(0)", "#NUM!")]
    [InlineData("=LOG(8,1)", "#NUM!")]          // a base of one has no logarithm
    [InlineData("=INT(\"abc\")", "#VALUE!")]
    [InlineData("=CEILING()", "#VALUE!")]
    [InlineData("=MOD(1)", "#VALUE!")]
    [InlineData("=PI(1)", "#VALUE!")]
    [InlineData("=ROUNDUP()", "#VALUE!")]
    [InlineData("=PRODUCT(\"abc\")", "0")]      // nothing numeric to multiply
    public void Report_bad_input(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula));
    }
}
