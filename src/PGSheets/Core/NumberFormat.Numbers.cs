using System.Globalization;
using System.Text;

namespace PGSheets.Core;

public static partial class NumberFormat
{
    /// <summary>A format section split into the parts that matter when it is rendered.</summary>
    private sealed class Pattern
    {
        private readonly List<DateToken>? _dateTokens;

        public Pattern()
        {
        }

        public Pattern(List<DateToken> dateTokens) => _dateTokens = dateTokens;

        public string Prefix { get; init; } = string.Empty;

        public string Suffix { get; init; } = string.Empty;

        public int MinIntegerDigits { get; init; }

        public int MinFractionDigits { get; init; }

        public int MaxFractionDigits { get; init; }

        public bool Grouping { get; init; }

        public bool Percent { get; init; }

        public int Scale { get; init; }

        /// <summary>A section that is nothing but literal text, e.g. <c>"zero"</c>.</summary>
        public bool LiteralOnly { get; init; }

        public bool TryRender(double value, int sectionCount, out string text) =>
            _dateTokens is null
                ? TryRenderNumber(value, sectionCount, out text)
                : TryRenderDate(value, _dateTokens, out text);

        /// <summary>
        /// Lays the digits out according to the placeholders. Returns false when the value cannot be
        /// represented as digits at all, so the caller can fall back to the plain rendering.
        /// </summary>
        private bool TryRenderNumber(double value, int sectionCount, out string text)
        {
            text = string.Empty;

            if (LiteralOnly)
            {
                // The section is a word rather than a number layout: "0.00;0.00;\"zero\"".
                text = Prefix + Suffix;
                return true;
            }

            if (!double.IsFinite(value) || Math.Abs(value) > 7.9e28)
            {
                return false;
            }

            // Decimal keeps the digits exact, which is what makes "0.00" land on the value a reader
            // expects rather than on the binary fraction a hair either side of it.
            var magnitude = (decimal)Math.Abs(value);

            if (Percent)
            {
                magnitude *= 100m;
            }

            for (var index = 0; index < Scale; index++)
            {
                magnitude /= 1000m;
            }

            var rounded = Math.Round(magnitude, MaxFractionDigits, MidpointRounding.AwayFromZero);
            var digits = rounded.ToString("F" + MaxFractionDigits, CultureInfo.InvariantCulture);

            var point = digits.IndexOf('.');
            var integers = point < 0 ? digits : digits[..point];
            var fraction = point < 0 ? string.Empty : digits[(point + 1)..];

            // "#" placeholders drop trailing zeros; "0" placeholders keep them.
            while (fraction.Length > MinFractionDigits && fraction.EndsWith('0'))
            {
                fraction = fraction[..^1];
            }

            var builder = new StringBuilder();

            // One section has to supply its own sign. With two or more, the negative section carries
            // whatever the workbook wants shown — parentheses, for instance.
            if (value < 0d && sectionCount == 1)
            {
                builder.Append('-');
            }

            builder.Append(Prefix);
            builder.Append(GroupInteger(integers));

            if (fraction.Length > 0)
            {
                builder.Append('.');
                builder.Append(fraction);
            }

            builder.Append(Suffix);
            text = builder.ToString();
            return true;
        }

        private string GroupInteger(string digits)
        {
            if (digits.Length < MinIntegerDigits)
            {
                digits = digits.PadLeft(MinIntegerDigits, '0');
            }

            if (!Grouping || digits.Length <= 3)
            {
                return digits;
            }

            var builder = new StringBuilder(digits.Length + (digits.Length / 3));
            var leading = digits.Length % 3;

            for (var index = 0; index < digits.Length; index++)
            {
                if (index > 0 && (index - leading) % 3 == 0)
                {
                    builder.Append(',');
                }

                builder.Append(digits[index]);
            }

            return builder.ToString();
        }
    }

    private static bool TryParse(string section, out Pattern? pattern)
    {
        pattern = null;

        if (IsDatePattern(section))
        {
            if (!TryScanDateTokens(section, out var tokens))
            {
                return false;
            }

            pattern = new Pattern(tokens);
            return true;
        }

        return TryScanNumber(section, out pattern);
    }

    /// <summary>Reads the placeholders, literals and modifiers of a numeric section.</summary>
    private static bool TryScanNumber(string section, out Pattern? pattern)
    {
        pattern = null;

        var prefix = new StringBuilder();
        var suffix = new StringBuilder();
        var current = prefix;
        var minInteger = 0;
        var minFraction = 0;
        var maxFraction = 0;
        var placeholders = 0;
        var grouping = false;
        var percent = false;
        var scale = 0;
        var seenPoint = false;
        var seenDigits = false;

        for (var index = 0; index < section.Length; index++)
        {
            var ch = section[index];

            switch (ch)
            {
                case '"':
                {
                    var close = section.IndexOf('"', index + 1);

                    if (close < 0)
                    {
                        return false;
                    }

                    current.Append(section, index + 1, close - index - 1);
                    index = close;
                    continue;
                }

                case '\\':
                {
                    if (index + 1 >= section.Length)
                    {
                        return false;
                    }

                    current.Append(section[++index]);
                    continue;
                }

                case '_':
                {
                    // "_x" reserves the width of x; one space is close enough.
                    if (index + 1 >= section.Length)
                    {
                        return false;
                    }

                    current.Append(' ');
                    index++;
                    continue;
                }

                case '*':
                {
                    // "*x" repeats x to fill the cell; the fill itself is not reproduced.
                    if (index + 1 >= section.Length)
                    {
                        return false;
                    }

                    index++;
                    continue;
                }

                case '[':
                {
                    var close = section.IndexOf(']', index + 1);

                    if (close < 0)
                    {
                        return false;
                    }

                    var directive = section[(index + 1)..close].Trim().ToLowerInvariant();

                    // Elapsed time changes what the digits mean, so it is not approximated.
                    if (directive is "h" or "hh" or "m" or "mm" or "s" or "ss")
                    {
                        return false;
                    }

                    index = close;
                    continue;
                }

                case '0':
                case '#':
                case '?':
                    current = suffix;
                    placeholders++;
                    seenDigits = true;

                    if (seenPoint)
                    {
                        maxFraction++;

                        if (ch == '0')
                        {
                            minFraction++;
                        }
                    }
                    else if (ch == '0')
                    {
                        minInteger++;
                    }

                    continue;

                case '.':
                    if (seenPoint)
                    {
                        suffix.Append('.');
                    }

                    seenPoint = true;
                    continue;

                case ',':
                    if (!seenDigits)
                    {
                        prefix.Append(',');
                        continue;
                    }

                    // A comma between digits groups them; a trailing one scales by a thousand.
                    if (IsPlaceholder(Peek(section, index + 1)))
                    {
                        grouping = true;
                    }
                    else
                    {
                        scale++;
                    }

                    continue;

                case '%':
                    percent = true;
                    current.Append('%');
                    continue;

                default:
                    if (char.IsLetter(ch))
                    {
                        // A letter here is scientific notation, "General", or a token this
                        // interpreter does not know. None of them can be approximated safely.
                        return false;
                    }

                    // An unquoted '/' next to a placeholder is a fraction ("# ?/?"), which would
                    // otherwise be laid out as nonsense. A quoted one never reaches here.
                    if (ch == '/' && seenDigits && IsPlaceholder(Peek(section, index + 1)))
                    {
                        return false;
                    }

                    current.Append(ch);
                    continue;
            }
        }

        if (placeholders == 0)
        {
            // A section with no placeholders can still be pure literal text, which is how a workbook
            // writes a word for a value: 0.00;0.00;"zero". A bare word, "General" or "@" cannot
            // describe a number at all, so those fall back to the plain rendering.
            var hasLiteral = section.IndexOf('"') >= 0 || section.IndexOf('\\') >= 0;

            if (!hasLiteral || section.IndexOf('@') >= 0)
            {
                return false;
            }

            pattern = new Pattern
            {
                Prefix = prefix.ToString(),
                Suffix = suffix.ToString(),
                LiteralOnly = true,
            };

            return true;
        }

        pattern = new Pattern
        {
            Prefix = prefix.ToString(),
            Suffix = suffix.ToString(),
            MinIntegerDigits = minInteger,
            MinFractionDigits = minFraction,
            MaxFractionDigits = maxFraction,
            Grouping = grouping,
            Percent = percent,
            Scale = scale,
        };

        return true;
    }

    private static char Peek(string text, int index) => index < text.Length ? text[index] : '\0';

    private static bool IsPlaceholder(char ch) => ch is '0' or '#' or '?';
}
