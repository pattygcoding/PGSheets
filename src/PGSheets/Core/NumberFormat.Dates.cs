using System.Globalization;
using System.Text;

namespace PGSheets.Core;

public static partial class NumberFormat
{
    private enum DateTokenKind
    {
        Literal,
        YearFourDigit,
        YearTwoDigit,

        /// <summary>"m" or "mm", which means a month or a minute depending on what is beside it.</summary>
        MonthOrMinute,
        MonthShortName,
        MonthName,
        DayNumber,
        DayShortName,
        DayName,
        Hour,
        Second,

        /// <summary>The digits after a point, which are a fraction of a second.</summary>
        FractionalSecond,
        AmPm,
        ApMarker,
    }

    private readonly record struct DateToken(DateTokenKind Kind, bool Padded, string Literal, int Digits = 0);

    /// <summary>Reads a "yyyy-mm-dd hh:mm" style section into the tokens to emit in order.</summary>
    private static bool TryScanDateTokens(string section, out List<DateToken> tokens)
    {
        tokens = new List<DateToken>();

        for (var index = 0; index < section.Length; index++)
        {
            var ch = section[index];

            if (ch == '"')
            {
                var close = section.IndexOf('"', index + 1);

                if (close < 0)
                {
                    return false;
                }

                tokens.Add(new DateToken(DateTokenKind.Literal, false, section[(index + 1)..close]));
                index = close;
                continue;
            }

            if (ch == '\\' || ch == '_' || ch == '*')
            {
                if (index + 1 >= section.Length)
                {
                    return false;
                }

                if (ch != '*')
                {
                    // "_x" reserves the width of x; one space stands in for it.
                    tokens.Add(new DateToken(DateTokenKind.Literal, false, ch == '_' ? " " : section[index + 1].ToString()));
                }

                index++;
                continue;
            }

            if (ch == '[')
            {
                var close = section.IndexOf(']', index + 1);

                if (close < 0)
                {
                    return false;
                }

                var directive = section[(index + 1)..close].Trim().ToLowerInvariant();

                // "[h]", "[mm]" and friends count elapsed time, which changes what the digits mean.
                if (directive is "h" or "hh" or "m" or "mm" or "s" or "ss")
                {
                    return false;
                }

                index = close;
                continue;
            }

            if (TryReadMeridiem(section, index, out var marker, out var length))
            {
                tokens.Add(new DateToken(marker, false, string.Empty));
                index += length - 1;
                continue;
            }

            if (ch == '.')
            {
                // A point followed by digits is a fraction of a second, as in "hh:mm:ss.000".
                var digits = 0;

                while (index + 1 + digits < section.Length && section[index + 1 + digits] == '0')
                {
                    digits++;
                }

                if (digits > 0)
                {
                    // The point is kept as its own literal so "hh:mm:ss.000" keeps its separator.
                    tokens.Add(new DateToken(DateTokenKind.Literal, false, "."));
                    tokens.Add(new DateToken(DateTokenKind.FractionalSecond, false, string.Empty, digits));
                    index += digits;
                    continue;
                }
            }

            if (!char.IsLetter(ch))
            {
                tokens.Add(new DateToken(DateTokenKind.Literal, false, ch.ToString()));
                continue;
            }

            var run = 1;

            while (index + run < section.Length &&
                   char.ToLowerInvariant(section[index + run]) == char.ToLowerInvariant(ch))
            {
                run++;
            }

            if (!TryMapDateToken(ch, run, out var token))
            {
                return false;
            }

            tokens.Add(token);
            index += run - 1;
        }

        return tokens.Count > 0;
    }

    /// <summary>Maps one run of repeated letters, e.g. "mmmm", to the part it stands for.</summary>
    private static bool TryMapDateToken(char ch, int run, out DateToken token)
    {
        token = default;
        var padded = run >= 2;

        switch (char.ToLowerInvariant(ch))
        {
            case 'y':
                if (run > 4)
                {
                    return false;
                }

                token = new DateToken(
                    run == 2 ? DateTokenKind.YearTwoDigit : DateTokenKind.YearFourDigit,
                    padded,
                    string.Empty);

                return true;

            case 'm':
                token = run switch
                {
                    1 or 2 => new DateToken(DateTokenKind.MonthOrMinute, padded, string.Empty),
                    3 => new DateToken(DateTokenKind.MonthShortName, false, string.Empty),
                    4 => new DateToken(DateTokenKind.MonthName, false, string.Empty),
                    _ => default,
                };

                return run <= 4;

            case 'd':
                token = run switch
                {
                    1 or 2 => new DateToken(DateTokenKind.DayNumber, padded, string.Empty),
                    3 => new DateToken(DateTokenKind.DayShortName, false, string.Empty),
                    4 => new DateToken(DateTokenKind.DayName, false, string.Empty),
                    _ => default,
                };

                return run <= 4;

            case 'h':
                if (run > 2)
                {
                    return false;
                }

                token = new DateToken(DateTokenKind.Hour, padded, string.Empty);
                return true;

            case 's':
                if (run > 2)
                {
                    return false;
                }

                token = new DateToken(DateTokenKind.Second, padded, string.Empty);
                return true;

            default:
                return false;
        }
    }

    private static bool TryReadMeridiem(string section, int index, out DateTokenKind kind, out int length)
    {
        kind = DateTokenKind.AmPm;

        if (MatchesAt(section, index, "AM/PM"))
        {
            length = 5;
            return true;
        }

        if (MatchesAt(section, index, "A/P"))
        {
            kind = DateTokenKind.ApMarker;
            length = 3;
            return true;
        }

        length = 0;
        return false;
    }

    private static bool MatchesAt(string text, int index, string candidate) =>
        index + candidate.Length <= text.Length &&
        string.Equals(text.Substring(index, candidate.Length), candidate, StringComparison.OrdinalIgnoreCase);

    /// <summary>
    /// Renders a serial number as a date. Anything before the start of the date system cannot be a
    /// date, so it falls back to the plain rendering.
    /// </summary>
    private static bool TryRenderDate(double value, List<DateToken> tokens, out string text)
    {
        text = string.Empty;

        if (!SerialDates.TryToDateTime(value, out var date))
        {
            return false;
        }

        // An AM/PM marker is what makes the hours count a twelve-hour clock.
        var twelveHour = tokens.Any(token => token.Kind is DateTokenKind.AmPm or DateTokenKind.ApMarker);
        var builder = new StringBuilder(tokens.Count * 2);

        for (var index = 0; index < tokens.Count; index++)
        {
            AppendDateToken(builder, tokens, index, date, value, twelveHour);
        }

        text = builder.ToString();
        return true;
    }

    private static void AppendDateToken(
        StringBuilder builder,
        List<DateToken> tokens,
        int index,
        DateTime date,
        double serial,
        bool twelveHour)
    {
        var token = tokens[index];
        var format = CultureInfo.InvariantCulture.DateTimeFormat;

        switch (token.Kind)
        {
            case DateTokenKind.Literal:
                builder.Append(token.Literal);
                return;

            case DateTokenKind.YearFourDigit:
                builder.Append(date.Year.ToString("D4", CultureInfo.InvariantCulture));
                return;

            case DateTokenKind.YearTwoDigit:
                builder.Append((date.Year % 100).ToString("D2", CultureInfo.InvariantCulture));
                return;

            case DateTokenKind.MonthName:
                builder.Append(format.GetMonthName(date.Month));
                return;

            case DateTokenKind.MonthShortName:
                builder.Append(Abbreviate(format.GetMonthName(date.Month)));
                return;

            case DateTokenKind.MonthOrMinute:
                builder.Append(Pad(IsMinute(tokens, index) ? date.Minute : date.Month, token.Padded));
                return;

            case DateTokenKind.DayNumber:
                builder.Append(Pad(date.Day, token.Padded));
                return;

            case DateTokenKind.DayName:
                builder.Append(format.GetDayName(date.DayOfWeek));
                return;

            case DateTokenKind.DayShortName:
                builder.Append(Abbreviate(format.GetDayName(date.DayOfWeek)));
                return;

            case DateTokenKind.Hour:
                // A twelve-hour clock counts 12, 1, 2, ... rather than 0, 1, 2, ...
                builder.Append(Pad(twelveHour ? ((date.Hour + 11) % 12) + 1 : date.Hour, token.Padded));
                return;

            case DateTokenKind.Second:
                builder.Append(Pad(date.Second, token.Padded));
                return;

            case DateTokenKind.FractionalSecond:
            {
                // Read from the fraction of the day rather than from the DateTime: AddDays rounds to
                // the nearest millisecond on the whole serial, which can be a millisecond out.
                var fraction = serial - Math.Floor(serial);
                var whole = fraction * 86400d % 1d;
                var milliseconds = Math.Clamp(
                    (int)Math.Round(whole * 1000d, MidpointRounding.AwayFromZero),
                    0,
                    999);

                var text = milliseconds.ToString("D3", CultureInfo.InvariantCulture);
                builder.Append(text[..Math.Clamp(token.Digits, 1, 3)]);
                return;
            }

            case DateTokenKind.AmPm:
                builder.Append(date.Hour < 12 ? "AM" : "PM");
                return;

            default:
                builder.Append(date.Hour < 12 ? "A" : "P");
                return;
        }
    }

    /// <summary>
    /// True when a run of "m"s stands for minutes: a spreadsheet reads it that way next to an hour
    /// or before seconds, and as a month everywhere else.
    /// </summary>
    private static bool IsMinute(List<DateToken> tokens, int index)
    {
        for (var before = index - 1; before >= 0; before--)
        {
            if (tokens[before].Kind == DateTokenKind.Literal)
            {
                continue;
            }

            if (tokens[before].Kind == DateTokenKind.Hour)
            {
                return true;
            }

            break;
        }

        for (var after = index + 1; after < tokens.Count; after++)
        {
            if (tokens[after].Kind == DateTokenKind.Literal)
            {
                continue;
            }

            return tokens[after].Kind == DateTokenKind.Second;
        }

        return false;
    }

    private static string Pad(int value, bool padded) =>
        padded
            ? value.ToString("D2", CultureInfo.InvariantCulture)
            : value.ToString(CultureInfo.InvariantCulture);

    private static string Abbreviate(string name) => name.Length <= 3 ? name : name[..3];
}
