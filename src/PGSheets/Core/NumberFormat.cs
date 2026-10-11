using System.Globalization;
using System.Text;

namespace PGSheets.Core;

/// <summary>
/// Renders a number the way a workbook's number format says it should be: the "0.00", "#,##0", "0%",
/// "$#,##0.00" and "yyyy-mm-dd" codes a cell carries.
/// </summary>
/// <remarks>
/// The format language is large, so this interprets the parts a workbook actually uses and refuses
/// the rest: a code containing anything it cannot represent falls back to the plain rendering. That
/// way a format can never make a cell harder to read than no format at all. Sections are separated
/// by ';' (positive; negative; zero; text) and the one that applies is chosen from the value's sign.
/// </remarks>
public static partial class NumberFormat
{
    /// <summary>The code an unformatted cell has, and the name of the plain rendering.</summary>
    public const string General = "General";

    /// <summary>
    /// Formats <paramref name="value"/> with <paramref name="format"/>, falling back to the plain
    /// rendering when there is no format or it cannot be represented.
    /// </summary>
    public static string Apply(double value, string? format) =>
        TryApply(value, format, out var text) ? text : ApplyGeneral(value);

    /// <summary>The rendering of a number with no format applied.</summary>
    public static string ApplyGeneral(double value) => value.ToString("G15", CultureInfo.InvariantCulture);

    /// <summary>True when the code means "no formatting", so the plain rendering applies.</summary>
    public static bool IsGeneral(string? format) =>
        string.IsNullOrWhiteSpace(format) ||
        string.Equals(format.Trim(), General, StringComparison.OrdinalIgnoreCase);

    private static bool TryApply(double value, string? format, out string text)
    {
        text = string.Empty;

        if (IsGeneral(format))
        {
            return false;
        }

        var sections = SplitSections(format!);

        if (sections.Count is 0 or > 4)
        {
            return false;
        }

        var section = ChooseSection(sections, value);

        if (section.Trim().Length == 0)
        {
            // An empty section is how a workbook hides a value: "0.00;" shows nothing negative.
            return true;
        }

        return TryParse(section, out var pattern) && pattern!.TryRender(value, sections.Count, out text);
    }

    /// <summary>Splits a code on ';', ignoring separators inside quotes or brackets.</summary>
    private static List<string> SplitSections(string format)
    {
        var sections = new List<string>();
        var builder = new StringBuilder();
        var inQuotes = false;
        var inBrackets = false;

        foreach (var ch in format)
        {
            switch (ch)
            {
                case '"':
                    inQuotes = !inQuotes;
                    builder.Append(ch);
                    break;
                case '[' when !inQuotes:
                    inBrackets = true;
                    builder.Append(ch);
                    break;
                case ']' when !inQuotes:
                    inBrackets = false;
                    builder.Append(ch);
                    break;
                case ';' when !inQuotes && !inBrackets:
                    sections.Add(builder.ToString());
                    builder.Clear();
                    break;
                default:
                    builder.Append(ch);
                    break;
            }
        }

        sections.Add(builder.ToString());
        return sections;
    }

    /// <summary>
    /// Picks the section for a value: one section covers everything, two split negative from the
    /// rest, and three or four add a separate section for zero.
    /// </summary>
    private static string ChooseSection(List<string> sections, double value) => sections.Count switch
    {
        1 => sections[0],
        2 => value < 0d ? sections[1] : sections[0],
        _ => value > 0d ? sections[0] : value < 0d ? sections[1] : sections[2],
    };

    /// <summary>
    /// True when a section is a date pattern. Letters inside quotes or brackets are literal text or
    /// a directive, so "0.00 \"days\"" and "[Red]0.00" are both numbers.
    /// </summary>
    private static bool IsDatePattern(string section)
    {
        var inQuotes = false;

        for (var index = 0; index < section.Length; index++)
        {
            var ch = section[index];

            if (ch == '"')
            {
                inQuotes = !inQuotes;
                continue;
            }

            if (inQuotes)
            {
                continue;
            }

            if (ch == '\\' || ch == '_' || ch == '*')
            {
                // An escaped character, a reserved width or a fill: skip what follows.
                index++;
                continue;
            }

            if (ch == '[')
            {
                var close = section.IndexOf(']', index + 1);
                index = close < 0 ? section.Length : close;
                continue;
            }

            if (ch is 'y' or 'Y' or 'm' or 'M' or 'd' or 'D' or 'h' or 'H' or 's' or 'S')
            {
                return true;
            }
        }

        return false;
    }
}
