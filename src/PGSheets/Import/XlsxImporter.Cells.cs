using System.Globalization;
using System.Text;
using System.Xml;
using PGSheets.Core;

namespace PGSheets.Import;

public sealed partial class XlsxImporter
{
    /// <summary>
    /// Converts a raw worksheet cell into the text the model will store, together with the number
    /// format that gives that text its meaning. A date stays the serial the file holds: the format is
    /// what turns it back into a date, exactly as it does in a spreadsheet, which is also what makes
    /// it behave like the number it is.
    /// </summary>
    private static (string Raw, string? Format) ResolveCell(
        string? type,
        string? value,
        string? formula,
        string? inlineText,
        string? styleAttribute,
        List<string> sharedStrings,
        Dictionary<int, string> numberFormats,
        bool importFormulas)
    {
        var format = ReadFormat(styleAttribute, numberFormats);

        if (importFormulas && formula is { Length: > 0 })
        {
            return ("=" + formula, format);
        }

        string raw;

        switch (type)
        {
            case "s":
                raw = value is not null &&
                      int.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out var index) &&
                      index >= 0 &&
                      index < sharedStrings.Count
                    ? sharedStrings[index]
                    : string.Empty;
                break;

            case "inlineStr":
                raw = inlineText ?? value ?? string.Empty;
                break;

            case "b":
                raw = value == "1" ? "TRUE" : "FALSE";
                break;

            default:
                // Numeric cells ("n" or no type) and cached formula strings arrive in <v>.
                raw = value ?? string.Empty;
                break;
        }

        return (raw, format);
    }

    /// <summary>The number format a cell refers to with its "s" attribute, or null when it has none.</summary>
    private static string? ReadFormat(string? styleAttribute, Dictionary<int, string> numberFormats) =>
        styleAttribute is not null &&
        int.TryParse(styleAttribute, NumberStyles.Integer, CultureInfo.InvariantCulture, out var styleIndex) &&
        numberFormats.TryGetValue(styleIndex, out var format)
            ? format
            : null;

    /// <summary>
    /// Reads the concatenated text of the element the reader is positioned on, leaving the reader on
    /// that element's end tag. (ReadElementContentAsString advances onto the following sibling, which
    /// would make the caller's read loop skip the next element.)
    /// </summary>
    private static string ReadElementText(XmlReader reader)
    {
        if (reader.IsEmptyElement)
        {
            return string.Empty;
        }

        var depth = reader.Depth;
        var builder = new StringBuilder();

        while (reader.Read())
        {
            if (reader.NodeType == XmlNodeType.EndElement && reader.Depth == depth)
            {
                break;
            }

            if (reader.NodeType is XmlNodeType.Text or XmlNodeType.CDATA or XmlNodeType.SignificantWhitespace)
            {
                builder.Append(reader.Value);
            }
        }

        return builder.ToString();
    }
    /// <summary>Finds function names in a formula so unsupported ones can be reported up front.</summary>
    private static IEnumerable<string> ExtractFunctionNames(string formula)
    {
        for (var i = 0; i < formula.Length; i++)
        {
            var ch = formula[i];

            if (!char.IsLetter(ch) && ch != '_')
            {
                continue;
            }

            var start = i;

            while (i < formula.Length && (char.IsLetterOrDigit(formula[i]) || formula[i] == '_' || formula[i] == '.'))
            {
                i++;
            }

            var token = formula[start..i];

            if (i >= formula.Length || formula[i] != '(')
            {
                continue;
            }

            // A cell reference is never followed by '('; guard against A1-style tokens anyway.
            if (CellAddress.TryParseA1(token, out _, out _))
            {
                continue;
            }

            yield return NormalizeFunctionName(token);
        }
    }

    private static string NormalizeFunctionName(string token)
    {
        // Excel prefixes functions added after the original specification, e.g. _xlfn.CONCAT.
        foreach (var prefix in new[] { "_xlfn.", "_xlws." })
        {
            if (token.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
            {
                return token[prefix.Length..];
            }
        }

        return token;
    }
}
