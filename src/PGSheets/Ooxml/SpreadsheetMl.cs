using System.Text;
using System.Xml;

namespace PGSheets.Ooxml;

/// <summary>
/// Shared constants and helpers for the SpreadsheetML (OOXML) container. The reader and the writer
/// must agree exactly on namespace URIs and on how arbitrary text is made XML-safe, so both use
/// this type instead of repeating the values and risking drift.
/// </summary>
public static class SpreadsheetMl
{
    /// <summary>Namespace of the workbook, worksheet and styles parts.</summary>
    public const string MainNamespace = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

    /// <summary>Namespace of the <c>r:id</c> attributes that link a workbook to its parts.</summary>
    public const string RelationshipNamespace =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

    /// <summary>Namespace of <c>[Content_Types].xml</c>.</summary>
    public const string ContentTypesNamespace = "http://schemas.openxmlformats.org/package/2006/content-types";

    /// <summary>Namespace of every <c>.rels</c> part.</summary>
    public const string PackageRelationshipNamespace =
        "http://schemas.openxmlformats.org/package/2006/relationships";

    /// <summary>Longest worksheet name a workbook may contain.</summary>
    public const int MaxSheetNameLength = 31;

    /// <summary>
    /// Reader settings that refuse DTDs and external entities. Workbooks are untrusted input, so
    /// these stop a malicious file from making the reader fetch or expand anything.
    /// </summary>
    public static XmlReaderSettings ReaderSettings => new()
    {
        DtdProcessing = DtdProcessing.Prohibit,
        IgnoreComments = true,
        IgnoreProcessingInstructions = true,
        XmlResolver = null,
    };

    /// <summary>Writer settings producing compact UTF-8 parts with no byte-order mark.</summary>
    public static XmlWriterSettings WriterSettings => new()
    {
        Encoding = new UTF8Encoding(encoderShouldEmitUTF8Identifier: false),
        Indent = false,
        CloseOutput = false,
    };

    /// <summary>
    /// Removes characters XML 1.0 cannot represent: most control characters and unpaired
    /// surrogates. Cell content is arbitrary user data, and one illegal character is enough to make
    /// the whole part unreadable, so text is always filtered before it is written.
    /// Returns the same instance when nothing needs changing.
    /// </summary>
    public static string SanitizeText(string text)
    {
        StringBuilder? builder = null;

        for (var index = 0; index < text.Length; index++)
        {
            var ch = text[index];

            // A valid surrogate pair is a legal XML character, so both halves are kept.
            if (char.IsHighSurrogate(ch) && index + 1 < text.Length && char.IsLowSurrogate(text[index + 1]))
            {
                builder?.Append(ch).Append(text[index + 1]);
                index++;
                continue;
            }

            if (IsLegalXmlChar(ch))
            {
                builder?.Append(ch);
                continue;
            }

            builder ??= new StringBuilder(text.Length).Append(text, 0, index);
        }

        return builder?.ToString() ?? text;
    }

    /// <summary>
    /// Reduces an arbitrary string to a legal worksheet name: at most 31 characters, none of
    /// <c>[]:*?/\</c>, and no characters XML cannot carry. Falls back to "Sheet1" when nothing
    /// usable is left.
    /// </summary>
    public static string SanitizeSheetName(string? name)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            return "Sheet1";
        }

        var builder = new StringBuilder(Math.Min(name.Length, MaxSheetNameLength));

        foreach (var ch in name)
        {
            if (ch is '[' or ']' or ':' or '*' or '?' or '/' or '\\' || !IsLegalXmlChar(ch))
            {
                continue;
            }

            builder.Append(ch);

            if (builder.Length == MaxSheetNameLength)
            {
                break;
            }
        }

        var cleaned = builder.ToString().Trim();
        return cleaned.Length == 0 ? "Sheet1" : cleaned;
    }

    private static bool IsLegalXmlChar(char ch) =>
        ch is '\t' or '\n' or '\r' ||
        (ch >= '\u0020' && ch <= '\uD7FF') ||
        (ch >= '\uE000' && ch <= '\uFFFD');
}
