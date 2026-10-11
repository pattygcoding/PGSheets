using PGSheets.Core;

namespace PGSheets.Tests;

/// <summary>
/// The number-format interpreter. Every case goes through the public entry point, because the
/// guarantee that matters to a caller is the one it makes: a code either renders the value, or the
/// value is rendered plainly — never something in between that means nothing.
/// </summary>
public class NumberFormatTests
{
    private static string Format(double value, string? format) => NumberFormat.Apply(value, format);

    [Theory]
    [InlineData("0", 1.5, "2")]                 // halfway rounds away from zero
    [InlineData("0", 1.4, "1")]
    [InlineData("0", -1.5, "-2")]
    [InlineData("0.00", 1.5, "1.50")]
    [InlineData("0.00", 1.234, "1.23")]
    [InlineData("0.00", 1.005, "1.01")]         // held as a decimal, so this lands exactly
    [InlineData("#,##0", 1234567d, "1,234,567")]
    [InlineData("#,##0", 999d, "999")]
    [InlineData("#,##0.00", 1234.5, "1,234.50")]
    [InlineData("0000", 12d, "0012")]
    [InlineData("0.000", 0d, "0.000")]
    [InlineData("#.##", 1.5, "1.5")]            // "#" drops a trailing zero
    [InlineData("0.0#", 1.25, "1.25")]
    [InlineData("0%", 0.155, "16%")]
    [InlineData("0.0%", 0.155, "15.5%")]
    [InlineData("%0", 0.5, "%50")]              // the sign sits wherever the code puts it
    [InlineData("$#,##0.00", 1234.5678, "$1,234.57")]
    [InlineData("\"$\"#,##0.00", 1234.5678, "$1,234.57")]
    [InlineData("0\" units\"", 5d, "5 units")]
    [InlineData("#,##0,,", 123456789d, "123")]  // a trailing comma divides by a thousand
    [InlineData("[Red]0.00", 1.5, "1.50")]      // a colour does not change the digits
    public void A_number_is_rendered_the_way_the_code_describes(string format, double value, string expected)
    {
        Assert.Equal(expected, Format(value, format));
    }

    [Theory]
    [InlineData("0.00;[Red]-0.00", -1.5, "-1.50")]  // the negative section carries its own sign
    [InlineData("0.00;(0.00)", -1.5, "(1.50)")]
    [InlineData("0.00;(0.00)", 1.5, "1.50")]
    [InlineData("0.00;", -1.5, "")]                 // an empty section hides the value
    [InlineData("0.00;0.00;\"zero\"", 0d, "zero")]
    [InlineData("0.00;0.00;\"zero\"", 5d, "5.00")]
    [InlineData("0.00;0.00;\"zero\"", -5d, "5.00")] // with three sections the sign is the code's job
    public void The_section_that_applies_is_chosen_from_the_sign(string format, double value, string expected)
    {
        Assert.Equal(expected, Format(value, format));
    }

    [Theory]
    [InlineData("yyyy-mm-dd", 45356d, "2024-03-05")]
    [InlineData("yyyy/mm/dd", 45356d, "2024/03/05")]
    [InlineData("dd/mm/yyyy", 45356d, "05/03/2024")]                    // "mm" here is a month
    [InlineData("d mmm yyyy", 45356d, "5 Mar 2024")]
    [InlineData("dddd, d mmmm yyyy", 45356d, "Tuesday, 5 March 2024")]
    [InlineData("ddd d mmm yy", 45356d, "Tue 5 Mar 24")]
    [InlineData("mmm-yy", 45356d, "Mar-24")]
    [InlineData("hh:mm:ss", 45356.5d, "12:00:00")]                      // "mm" here is minutes
    [InlineData("hh:mm", 45356.75d, "18:00")]
    [InlineData("h:mm AM/PM", 45356.5d, "12:00 PM")]
    [InlineData("h:mm AM/PM", 45356d, "12:00 AM")]
    [InlineData("h:mm A/P", 45356.75d, "6:00 P")]
    [InlineData("yyyy-mm-dd hh:mm", 45356.25d, "2024-03-05 06:00")]     // a month, then a minute
    public void A_serial_is_rendered_as_the_date_the_code_describes(string format, double value, string expected)
    {
        Assert.Equal(expected, Format(value, format));
    }

    [Fact]
    public void Milliseconds_come_from_the_fraction_of_a_second()
    {
        var serial = SerialDates.ToSerial(new DateTime(2024, 3, 5, 6, 0, 0, 500));

        Assert.Equal("2024-03-05 06:00:00.500", Format(serial, "yyyy-mm-dd hh:mm:ss.000"));
        Assert.Equal("2024-03-05 06:00:00.5", Format(serial, "yyyy-mm-dd hh:mm:ss.0"));
    }

    [Fact]
    public void Quoted_text_can_contain_date_letters_without_making_it_a_date()
    {
        // "days" contains d, a, y and s, but it is quoted text, not a date pattern.
        Assert.Equal("1.50 days", Format(1.5, "0.00 \"days\""));
        Assert.Equal("1,234 apples", Format(1234d, "#,##0 \"apples\""));
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("   ")]
    [InlineData("General")]
    [InlineData("general")]
    public void No_format_means_the_plain_rendering(string? format)
    {
        Assert.Equal(NumberFormat.ApplyGeneral(1234.5), NumberFormat.Apply(1234.5, format));
    }

    [Theory]
    [InlineData("0.00E+00")]     // scientific notation
    [InlineData("[h]:mm")]       // elapsed time
    [InlineData("@")]            // a text placeholder cannot describe a number
    [InlineData("mmmmm")]        // a month token this interpreter does not know
    [InlineData("yyyyy")]        // nor a five-digit year
    [InlineData("# ?/?")]        // a fraction
    [InlineData("0;0;0;0;0")]    // more sections than a format can have
    public void A_code_it_cannot_represent_falls_back_to_the_plain_rendering(string format)
    {
        var value = 1234.5;

        Assert.Equal(NumberFormat.ApplyGeneral(value), NumberFormat.Apply(value, format));
    }

    [Theory]
    [InlineData(-1d)]
    [InlineData(double.NaN)]
    [InlineData(double.PositiveInfinity)]
    [InlineData(1e30)]
    public void A_serial_it_cannot_turn_into_a_date_falls_back(double value)
    {
        Assert.Equal(NumberFormat.ApplyGeneral(value), Format(value, "yyyy-mm-dd"));
    }

    [Theory]
    [InlineData(double.NaN)]
    [InlineData(double.PositiveInfinity)]
    [InlineData(1e30)]           // far beyond the digits a cell could lay out
    public void A_value_it_cannot_lay_out_as_digits_falls_back(double value)
    {
        Assert.Equal(NumberFormat.ApplyGeneral(value), Format(value, "#,##0.00"));
    }

    [Fact]
    public void A_negative_value_is_still_laid_out_as_digits()
    {
        Assert.Equal("-1.00", Format(-1d, "#,##0.00"));
    }
}
