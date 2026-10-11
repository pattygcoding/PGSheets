using PGSheets.Core;
using PGSheets.Formulas;

namespace PGSheets.Tests;

public class FormulaEngineAdvancedTests
{
    [Fact]
    public void Division_by_zero_is_reported()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "=1/0");

        Assert.Equal("#DIV/0!", Sheet.Display(model, "A1"));
    }

    [Fact]
    public void Unknown_function_is_reported()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "=WEBSERVICE(1)");

        Assert.Equal("#NAME?", Sheet.Display(model, "A1"));
    }

    [Fact]
    public void Text_in_arithmetic_is_reported()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "abc");
        Sheet.Set(model, "B1", "=A1+1");

        Assert.Equal("#VALUE!", Sheet.Display(model, "B1"));
    }

    [Fact]
    public void If_only_evaluates_the_taken_branch()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "=IF(TRUE,1,1/0)");
        Sheet.Set(model, "A2", "=IF(FALSE,1/0,2)");

        Assert.Equal("1", Sheet.Display(model, "A1"));
        Assert.Equal("2", Sheet.Display(model, "A2"));
    }

    [Fact]
    public void If_supports_comparisons()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "20");
        Sheet.Set(model, "B1", "=IF(A1>10,\"big\",\"small\")");
        Sheet.Set(model, "B2", "=IF(A1<=10,\"big\",\"small\")");

        Assert.Equal("big", Sheet.Display(model, "B1"));
        Assert.Equal("small", Sheet.Display(model, "B2"));
    }

    [Theory]
    [InlineData("=1<2", "TRUE")]
    [InlineData("=2<=2", "TRUE")]
    [InlineData("=3<>3", "FALSE")]
    [InlineData("=5>=4", "TRUE")]
    public void Comparison_operators_return_booleans(string formula, string expected)
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", formula);

        Assert.Equal(expected, Sheet.Display(model, "A1"));
    }

    [Fact]
    public void Aggregate_functions_are_available()
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "4");
        Sheet.Set(model, "A2", "8");
        Sheet.Set(model, "A3", "6");

        Sheet.Set(model, "C1", "=MIN(A1:A3)");
        Sheet.Set(model, "C2", "=MAX(A1:A3)");
        Sheet.Set(model, "C3", "=AVERAGE(A1:A3)");
        Sheet.Set(model, "C4", "=COUNT(A1:A3)");

        Assert.Equal("4", Sheet.Display(model, "C1"));
        Assert.Equal("8", Sheet.Display(model, "C2"));
        Assert.Equal("6", Sheet.Display(model, "C3"));
        Assert.Equal("3", Sheet.Display(model, "C4"));
    }

    [Fact]
    public void Hosts_can_register_extra_functions()
    {
        FormulaFunction implementation = (arguments, context) =>
        {
            var value = arguments[0].Evaluate(context);
            return CellValue.FromNumber(value.AsNumberOrDefault() * 2d);
        };

        BuiltinFunctions.Register("DOUBLE", implementation);

        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "21");
        Sheet.Set(model, "B1", "=DOUBLE(A1)");

        Assert.Equal("42", Sheet.Display(model, "B1"));
    }
}
