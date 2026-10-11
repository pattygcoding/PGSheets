namespace PGSheets.Tests;

/// <summary>VLOOKUP, HLOOKUP, INDEX, MATCH and CHOOSE.</summary>
public class BuiltinFunctionsLookupTests
{
    private static void Table(Core.SpreadsheetModel model)
    {
        Sheet.Set(model, "A1", "1");
        Sheet.Set(model, "B1", "one");
        Sheet.Set(model, "A2", "2");
        Sheet.Set(model, "B2", "two");
        Sheet.Set(model, "A3", "3");
        Sheet.Set(model, "B3", "three");
        Sheet.Set(model, "A4", "4");
        Sheet.Set(model, "B4", "four");

        Sheet.Set(model, "C1", "apple");
        Sheet.Set(model, "D1", "5");
        Sheet.Set(model, "C2", "banana");
        Sheet.Set(model, "D2", "6");
    }

    [Theory]
    [InlineData("=VLOOKUP(2,A1:B4,2,FALSE)", "two")]
    [InlineData("=VLOOKUP(2,A1:B4,2)", "two")]           // an exact hit needs no approximation
    [InlineData("=VLOOKUP(3.5,A1:B4,2)", "three")]       // approximate: the last key at or below
    [InlineData("=VLOOKUP(5,A1:B4,2,FALSE)", "#N/A")]
    [InlineData("=VLOOKUP(5,A1:B4,2)", "four")]          // approximate finds the top of the table
    [InlineData("=VLOOKUP(0,A1:B4,2)", "#N/A")]          // below every key
    [InlineData("=VLOOKUP(2,A1:B4,1,FALSE)", "2")]
    [InlineData("=VLOOKUP(\"banana\",C1:D2,2,FALSE)", "6")]
    [InlineData("=VLOOKUP(\"BANANA\",C1:D2,2,FALSE)", "6")] // text keys ignore case
    public void Vlookup_finds_the_row(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula, Table));
    }

    [Theory]
    [InlineData("=VLOOKUP(2,A1:B4,3)", "#REF!")]         // there is no third column
    [InlineData("=VLOOKUP(2,A1:B4,0)", "#REF!")]
    [InlineData("=VLOOKUP(2,A1,2)", "#VALUE!")]          // a single cell is not a table
    [InlineData("=VLOOKUP(2,A1:B4,2,TRUE,1)", "#VALUE!")]
    [InlineData("=VLOOKUP(1/0,A1:B4,2)", "#DIV/0!")]
    public void Vlookup_reports_bad_input(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula, Table));
    }

    [Fact]
    public void Hlookup_finds_the_column()
    {
        Action<Core.SpreadsheetModel> seed = model =>
        {
            Sheet.Set(model, "A1", "1");
            Sheet.Set(model, "B1", "2");
            Sheet.Set(model, "C1", "3");
            Sheet.Set(model, "A2", "one");
            Sheet.Set(model, "B2", "two");
            Sheet.Set(model, "C2", "three");
        };

        Assert.Equal("two", Sheet.Evaluate("=HLOOKUP(2,A1:C2,2,FALSE)", seed));
        Assert.Equal("three", Sheet.Evaluate("=HLOOKUP(3,A1:C2,2)", seed));
        Assert.Equal("#N/A", Sheet.Evaluate("=HLOOKUP(9,A1:C2,2,FALSE)", seed));
        Assert.Equal("#REF!", Sheet.Evaluate("=HLOOKUP(1,A1:C2,3)", seed));
    }

    [Theory]
    [InlineData("=MATCH(3,A1:A4,0)", "3")]               // exact, 1-based
    [InlineData("=MATCH(2.5,A1:A4,1)", "2")]             // the last value at or below the key
    [InlineData("=MATCH(9,A1:A4,0)", "#N/A")]
    [InlineData("=MATCH(1,A1:A4,1)", "1")]
    public void Match_returns_a_position(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula, model =>
        {
            for (var index = 1; index <= 4; index++)
            {
                Sheet.Set(model, $"A{index}", index.ToString());
            }
        }));
    }

    [Fact]
    public void Match_works_across_a_row_and_rejects_a_block()
    {
        Action<Core.SpreadsheetModel> seed = model =>
        {
            Sheet.Set(model, "A1", "10");
            Sheet.Set(model, "B1", "20");
            Sheet.Set(model, "C1", "30");
        };

        Assert.Equal("2", Sheet.Evaluate("=MATCH(20,A1:C1,0)", seed));

        // A position only exists along one dimension, so a block has no single answer.
        Assert.Equal("#N/A", Sheet.Evaluate("=MATCH(2,A1:B4,0)", Table));
    }

    [Theory]
    [InlineData("=INDEX(A1:B4,2,2)", "two")]
    [InlineData("=INDEX(A1:B4,4,1)", "4")]
    [InlineData("=INDEX(A1:B4,1,1)", "1")]
    [InlineData("=INDEX(A1:B4,5,1)", "#REF!")]
    [InlineData("=INDEX(A1:B4,1,3)", "#REF!")]
    public void Index_picks_a_cell(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula, Table));
    }

    [Fact]
    public void Index_of_a_single_row_reads_one_index_as_the_column()
    {
        Action<Core.SpreadsheetModel> seed = model =>
        {
            Sheet.Set(model, "A1", "first");
            Sheet.Set(model, "B1", "second");
            Sheet.Set(model, "C1", "third");
        };

        Assert.Equal("second", Sheet.Evaluate("=INDEX(A1:C1,2)", seed));
    }

    [Fact]
    public void Index_and_match_combine_the_way_a_lookup_table_needs()
    {
        // The classic pair: MATCH finds the row, INDEX reads a different column from it.
        Assert.Equal("three", Sheet.Evaluate("=INDEX(B1:B4,MATCH(3,A1:A4,0))", Table));
    }

    [Theory]
    [InlineData("=CHOOSE(2,\"a\",\"b\",\"c\")", "b")]
    [InlineData("=CHOOSE(1,\"a\",\"b\")", "a")]
    [InlineData("=CHOOSE(3,\"a\",\"b\")", "#VALUE!")]
    [InlineData("=CHOOSE(0,\"a\")", "#VALUE!")]
    [InlineData("=CHOOSE(1)", "#VALUE!")]
    public void Choose_picks_by_position(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula));
    }
}
