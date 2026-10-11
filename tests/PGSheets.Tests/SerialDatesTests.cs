using System.Globalization;
using PGSheets.Core;

namespace PGSheets.Tests;

/// <summary>
/// The serial-number date system. A wrong answer here would move every date in every file, so the
/// round trip is checked day by day across two centuries rather than at a handful of points.
/// </summary>
public class SerialDatesTests
{
    private static DateTime Date(string text) =>
        DateTime.ParseExact(text, "yyyy-MM-dd", CultureInfo.InvariantCulture);

    [Theory]
    [InlineData(45292d, "2024-01-01")]
    [InlineData(45356d, "2024-03-05")]
    [InlineData(43831d, "2020-01-01")]
    [InlineData(1d, "1900-01-01")]
    [InlineData(59d, "1900-02-28")]
    [InlineData(61d, "1900-03-01")]
    public void A_serial_becomes_the_date_it_stands_for(double serial, string expected)
    {
        Assert.True(SerialDates.TryToDateTime(serial, out var date));

        Assert.Equal(Date(expected), date);
    }

    [Theory]
    [InlineData("2024-01-01", 45292d)]
    [InlineData("2024-03-05", 45356d)]
    [InlineData("2020-01-01", 43831d)]
    [InlineData("1900-01-01", 1d)]
    [InlineData("1900-02-28", 59d)]
    [InlineData("1900-03-01", 61d)]
    public void A_date_becomes_the_serial_it_stands_for(string text, double expected)
    {
        Assert.Equal(expected, SerialDates.ToSerial(Date(text)));
    }

    [Fact]
    public void The_fraction_holds_the_time_of_day()
    {
        Assert.True(SerialDates.TryToDateTime(45292.5d, out var date));
        Assert.Equal(new DateTime(2024, 1, 1, 12, 0, 0), date);

        Assert.Equal(45292.5d, SerialDates.ToSerial(new DateTime(2024, 1, 1, 12, 0, 0)));
    }

    [Fact]
    public void Every_day_from_1900_to_2100_survives_a_round_trip()
    {
        var firstMismatch = (string?)null;
        var days = 0;

        for (var date = new DateTime(1900, 3, 1); date < new DateTime(2100, 1, 1); date = date.AddDays(1))
        {
            days++;

            if (SerialDates.TryToDateTime(SerialDates.ToSerial(date), out var back) && back == date)
            {
                continue;
            }

            firstMismatch ??= date.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture);
        }

        Assert.True(days > 70000, $"Only {days} days were checked.");
        Assert.True(firstMismatch is null, $"The first mismatch was at {firstMismatch}.");
    }

    [Fact]
    public void The_phantom_leap_day_lands_on_the_first_of_march()
    {
        // Excel counts 1900-02-29 as serial 60. .NET has no such day, so it reads as 1900-03-01 —
        // the same date as serial 61. That one serial is the only place the mapping is not exact.
        Assert.True(SerialDates.TryToDateTime(60d, out var date));
        Assert.Equal(Date("1900-03-01"), date);

        // The day before it is unaffected, which is what matters for real dates.
        Assert.True(SerialDates.TryToDateTime(59d, out var dayBefore));
        Assert.Equal(Date("1900-02-28"), dayBefore);
    }

    [Theory]
    [InlineData(-1d)]
    [InlineData(double.NaN)]
    [InlineData(double.PositiveInfinity)]
    [InlineData(double.NegativeInfinity)]
    public void A_serial_that_is_not_a_date_is_refused(double serial)
    {
        Assert.False(SerialDates.TryToDateTime(serial, out _));
    }
}
