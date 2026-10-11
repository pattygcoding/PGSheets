namespace PGSheets.Core;

/// <summary>
/// Converts between a date and the serial number a spreadsheet stores it as: a count of days from
/// the start of the 1900 date system, with the time of day held in the fraction.
/// </summary>
/// <remarks>
/// Excel keeps 1900-02-29 as serial 60 even though 1900 was not a leap year, so that files written
/// by Lotus 1-2-3 still line up. No such day exists in .NET: serial 60 therefore reads as 1900-03-01,
/// the same date as serial 61. That one ambiguous serial is the only place this codec is not exact;
/// every real date in the system converts both ways.
/// </remarks>
public static class SerialDates
{
    /// <summary>What the 1900 date system counts up from, for dates from March 1900 onwards.</summary>
    private static readonly DateTime Epoch = new(1899, 12, 30, 0, 0, 0, DateTimeKind.Unspecified);

    /// <summary>Where the phantom leap day would sit; earlier serials count from one day later.</summary>
    private static readonly DateTime PhantomDay = new(1900, 3, 1, 0, 0, 0, DateTimeKind.Unspecified);

    /// <summary>The base for the serials below the phantom day, which is why 1 is 1900-01-01.</summary>
    private static readonly DateTime EarlyEpoch = new(1899, 12, 31, 0, 0, 0, DateTimeKind.Unspecified);

    /// <summary>
    /// The last serial the date system can hold. Anything above it would fall outside what a
    /// <see cref="DateTime"/> can represent.
    /// </summary>
    private static readonly double MaxSerial =
        (DateTime.MaxValue - new DateTime(1899, 12, 30, 0, 0, 0, DateTimeKind.Unspecified)).TotalDays;

    /// <summary>
    /// The date a serial number stands for. Returns false for a serial that is negative, is not
    /// finite, or is beyond the end of the calendar — all of which a spreadsheet reports as #NUM!.
    /// </summary>
    public static bool TryToDateTime(double serial, out DateTime date)
    {
        date = default;

        if (!double.IsFinite(serial) || serial < 0d || serial > MaxSerial)
        {
            return false;
        }

        // Counting from a day later below the phantom day is what makes 1 = 1900-01-01 and
        // 59 = 1900-02-28. Serial 60 is the day Excel calls 1900-02-29, one day after the last real
        // date counted that way; with no such date to return it lands on 1900-03-01, which is also
        // what serial 61 is. That single ambiguous day is the price of the phantom leap year.
        date = (Math.Floor(serial) < 61d ? EarlyEpoch : Epoch).AddDays(serial);
        return true;
    }

    /// <summary>The serial number for a date, in the 1900 date system.</summary>
    public static double ToSerial(DateTime date) =>
        (date - (date < PhantomDay ? EarlyEpoch : Epoch)).TotalDays;
}
