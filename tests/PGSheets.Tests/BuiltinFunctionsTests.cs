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

    [Theory]
    [InlineData("SUM")]
    [InlineData("IF")]
    [InlineData("IFERROR")]
    [InlineData("VLOOKUP")]
    [InlineData("INDEX")]
    [InlineData("MATCH")]
    [InlineData("COUNTIF")]
    [InlineData("SUMIF")]
    [InlineData("LEFT")]
    [InlineData("TEXTJOIN")]
    [InlineData("MEDIAN")]
    [InlineData("STDEV")]
    [InlineData("ROUND")]
    [InlineData("CEILING")]
    [InlineData("CHOOSE")]
    [InlineData("ISNA")]
    public void The_library_covers_the_functions_a_real_workbook_reaches_for(string name)
    {
        Assert.True(Formulas.BuiltinFunctions.IsKnown(name), $"'{name}' is missing from the registry.");
    }

    [Fact]
    public void Every_built_in_function_survives_bad_input_without_throwing()
    {
        // One sweep over the whole shipped library. Being called with no arguments is the input
        // every built-in has to cope with, and none of them may throw: a bad argument is #VALUE!,
        // not an exception escaping into the recalculation of an unrelated cell.
        foreach (var name in BuiltInLibrary.Names)
        {
            Assert.True(
                Formulas.BuiltinFunctions.TryGet(name, out var function),
                $"'{name}' is in the registry but cannot be looked up.");

            var first = function(Array.Empty<Formulas.IFormulaNode>(), new EmptyContext());
            var second = function(Array.Empty<Formulas.IFormulaNode>(), new EmptyContext());

            // A worksheet function holds no hidden state, so the same call must give the same answer.
            Assert.True(first.Equals(second), $"'{name}' is not deterministic.");
        }
    }

    [Fact]
    public void The_sweep_covers_the_whole_library()
    {
        // A guard on the test above: if the snapshot were empty it would pass while testing nothing.
        Assert.True(BuiltInLibrary.Names.Length >= 78, $"Only {BuiltInLibrary.Names.Length} functions were swept.");
        Assert.Contains("SUM", BuiltInLibrary.Names);
        Assert.Contains("VLOOKUP", BuiltInLibrary.Names);
    }

    /// <summary>A context in which every cell is empty.</summary>
    private sealed class EmptyContext : Formulas.IFormulaContext
    {
        public Core.CellValue GetCellValue(int row, int column) => Core.CellValue.Empty;
    }
}
