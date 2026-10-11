namespace PGSheets.Tests;

/// <summary>The text half of the function library.</summary>
public class BuiltinFunctionsTextTests
{
    [Theory]
    [InlineData("=LEN(\"hello\")", "5")]
    [InlineData("=LEN(1234)", "4")]                 // a number is measured through its display form
    [InlineData("=LEN(\"\")", "0")]
    [InlineData("=LEFT(\"hello\",2)", "he")]
    [InlineData("=LEFT(\"hello\")", "h")]           // one character unless told otherwise
    [InlineData("=LEFT(\"hi\",9)", "hi")]           // asking for more than exists is not an error
    [InlineData("=RIGHT(\"hello\",3)", "llo")]
    [InlineData("=RIGHT(\"hello\",0)", "")]
    [InlineData("=MID(\"hello\",2,3)", "ell")]
    [InlineData("=MID(\"hello\",10,2)", "")]
    [InlineData("=UPPER(\"abc\")", "ABC")]
    [InlineData("=LOWER(\"ABC\")", "abc")]
    [InlineData("=UPPER(123)", "123")]
    [InlineData("=PROPER(\"hello world\")", "Hello World")]
    [InlineData("=TRIM(\"  a   b  \")", "a b")]     // ends removed and inner runs collapsed
    [InlineData("=REPT(\"ab\",3)", "ababab")]
    [InlineData("=REPT(\"ab\",0)", "")]
    [InlineData("=SUBSTITUTE(\"a-b-c\",\"-\",\"+\")", "a+b+c")]
    [InlineData("=SUBSTITUTE(\"a-b-c\",\"-\",\"+\",2)", "a-b+c")]
    [InlineData("=SUBSTITUTE(\"a-b-c\",\"-\",\"+\",9)", "a-b-c")] // fewer matches than asked for
    [InlineData("=SUBSTITUTE(\"a\",\"\",\"x\")", "a")]            // nothing to look for
    [InlineData("=FIND(\"l\",\"hello\")", "3")]     // 1-based
    [InlineData("=FIND(\"l\",\"hello\",4)", "4")]   // and case sensitive
    [InlineData("=EXACT(\"a\",\"A\")", "FALSE")]
    [InlineData("=EXACT(\"a\",\"a\")", "TRUE")]
    [InlineData("=VALUE(\"123\")", "123")]
    [InlineData("=VALUE(\"1,200\")", "1200")]
    [InlineData("=CHAR(65)", "A")]
    [InlineData("=CODE(\"A\")", "65")]
    [InlineData("=CONCATENATE(\"a\",\"b\")", "ab")]  // the older name for CONCAT
    public void Return_their_result(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula));
    }

    [Fact]
    public void Textjoin_joins_a_range_and_can_skip_blanks()
    {
        Action<Core.SpreadsheetModel> seed = model =>
        {
            Sheet.Set(model, "A1", "a");
            Sheet.Set(model, "A3", "c");
        };

        Assert.Equal("a,c", Sheet.Evaluate("=TEXTJOIN(\",\",TRUE,A1:A3)", seed));

        // With empties kept, the blank middle cell still contributes its separator.
        Assert.Equal("a,,c", Sheet.Evaluate("=TEXTJOIN(\",\",FALSE,A1:A3)", seed));
        Assert.Equal("a-c", Sheet.Evaluate("=TEXTJOIN(\"-\",TRUE,\"a\",\"c\")", seed));
    }

    [Theory]
    [InlineData("=LEFT(\"abc\",-1)", "#VALUE!")]
    [InlineData("=MID(\"abc\",0,1)", "#VALUE!")]     // start is 1-based
    [InlineData("=MID(\"abc\",1,-1)", "#VALUE!")]
    [InlineData("=FIND(\"a\",\"abc\",0)", "#VALUE!")]
    [InlineData("=FIND(\"z\",\"hello\")", "#VALUE!")] // a miss is an error, not zero
    [InlineData("=REPT(\"a\",-1)", "#VALUE!")]
    [InlineData("=REPT(\"ab\",20000)", "#VALUE!")]    // more than a cell could ever hold
    [InlineData("=CHAR(0)", "#VALUE!")]
    [InlineData("=CODE(\"\")", "#VALUE!")]
    [InlineData("=LEN()", "#VALUE!")]
    [InlineData("=VALUE(\"abc\")", "#VALUE!")]
    [InlineData("=TEXTJOIN(\",\",TRUE)", "#VALUE!")]
    [InlineData("=SUBSTITUTE(\"a\",\"b\",\"c\",0)", "#VALUE!")]
    public void Report_bad_input(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula));
    }

    [Fact]
    public void Text_functions_propagate_an_error_argument()
    {
        Assert.Equal("#DIV/0!", Sheet.Evaluate("=UPPER(1/0)"));
        Assert.Equal("#DIV/0!", Sheet.Evaluate("=LEFT(1/0,1)"));
    }
}
