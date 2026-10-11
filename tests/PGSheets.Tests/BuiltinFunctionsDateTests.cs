namespace PGSheets.Tests;

/// <summary>
/// The date-reading functions. They take a serial number or the ISO text an imported date cell
/// becomes, because both are how a date reaches the engine today.
/// </summary>
public class BuiltinFunctionsDateTests
{
    [Theory]
    [InlineData("=YEAR(45292)", "2024")]
    [InlineData("=MONTH(45292)", "1")]
    [InlineData("=DAY(45292)", "1")]
    [InlineData("=YEAR(45356)", "2024")]
    [InlineData("=MONTH(45356)", "3")]
    [InlineData("=DAY(45356)", "5")]
    [InlineData("=YEAR(\"2024-03-05\")", "2024")]        // what an imported date cell looks like
    [InlineData("=MONTH(\"2024-03-05\")", "3")]
    [InlineData("=DAY(\"2024-03-05\")", "5")]
    [InlineData("=DAY(\"2024-03-05 09:30:00\")", "5")]   // a date-time still has a day
    [InlineData("=YEAR(\"2024-03-05T09:30:00\")", "2024")]
    public void A_date_part_is_read_from_a_serial_or_from_iso_text(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula));
    }

    [Fact]
    public void A_date_part_reads_a_cell_holding_either_shape()
    {
        Assert.Equal(
            "3",
            Sheet.Evaluate("=MONTH(A1)", model => Sheet.Set(model, "A1", "2024-03-05")));

        Assert.Equal(
            "3",
            Sheet.Evaluate("=MONTH(A1)", model => Sheet.Set(model, "A1", "45356")));
    }

    [Theory]
    [InlineData("=WEEKDAY(45356)", "3")]      // 2024-03-05 was a Tuesday, and Sunday is 1
    [InlineData("=WEEKDAY(45292)", "2")]      // 2024-01-01 was a Monday
    [InlineData("=WEEKDAY(45356,2)", "2")]    // Monday is 1
    [InlineData("=WEEKDAY(45356,3)", "1")]    // Monday is 0
    [InlineData("=WEEKDAY(\"2024-03-05\")", "3")]
    public void Weekday_reports_which_day_it_is(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula));
    }

    [Theory]
    [InlineData("=YEAR()", "#VALUE!")]
    [InlineData("=YEAR(\"abc\")", "#VALUE!")]
    [InlineData("=YEAR(\"2024\")", "#VALUE!")]   // only a whole ISO date counts, not a bare year
    [InlineData("=YEAR(\"05/03/2024\")", "#VALUE!")]
    [InlineData("=YEAR(-1)", "#NUM!")]           // before the start of the date system
    [InlineData("=YEAR(1/0)", "#DIV/0!")]        // an error argument comes straight back
    [InlineData("=WEEKDAY(45292,4)", "#NUM!")]   // an unknown week type
    [InlineData("=WEEKDAY(45292,1,2)", "#VALUE!")]
    [InlineData("=MONTH()", "#VALUE!")]
    public void Report_bad_input(string formula, string expected)
    {
        Assert.Equal(expected, Sheet.Evaluate(formula));
    }

    [Fact]
    public void A_blank_cell_reads_as_the_start_of_the_date_system()
    {
        // An empty cell coerces to zero, and serial zero is the day before 1900-01-01.
        Assert.Equal("1899", Sheet.Evaluate("=YEAR(A1)"));
    }
}
