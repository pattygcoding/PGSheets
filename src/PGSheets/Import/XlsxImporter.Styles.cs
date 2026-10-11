using System.IO.Compression;
using System.Xml.Linq;
using PGSheets.Ooxml;

namespace PGSheets.Import;

public sealed partial class XlsxImporter
{
    /// <summary>
    /// The number format ids the SpreadsheetML specification fixes. A workbook normally writes its own
    /// codes into <c>numFmts</c> instead, but a minimal file refers to these ids.
    /// </summary>
    private static readonly Dictionary<int, string> BuiltInNumberFormats = new()
    {
        [1] = "0",
        [2] = "0.00",
        [3] = "#,##0",
        [4] = "#,##0.00",
        [9] = "0%",
        [10] = "0.00%",
        [11] = "0.00E+00",
        [12] = "# ?/?",
        [13] = "# ??/??",
        [14] = "mm-dd-yy",
        [15] = "d-mmm-yy",
        [16] = "d-mmm",
        [17] = "mmm-yy",
        [18] = "h:mm AM/PM",
        [19] = "h:mm:ss AM/PM",
        [20] = "h:mm",
        [21] = "h:mm:ss",
        [22] = "m/d/yy h:mm",
        [37] = "#,##0 ;(#,##0)",
        [38] = "#,##0 ;[Red](#,##0)",
        [39] = "#,##0.00;(#,##0.00)",
        [40] = "#,##0.00;[Red](#,##0.00)",
        [45] = "mm:ss",
        [46] = "[h]:mm:ss",
        [47] = "mmss.0",
        [48] = "##0.0E+0",
        [49] = "@",
    };

    /// <summary>
    /// Reads the number format of every cell style, keyed by the index a cell refers to with its
    /// <c>s</c> attribute. Both halves of the style table matter: <c>numFmts</c> holds the codes the
    /// workbook defined, and <c>cellXfs</c> is what maps a cell to one of them or to a built-in id.
    /// </summary>
    private static Dictionary<int, string> ReadNumberFormats(ZipArchive archive)
    {
        var formats = new Dictionary<int, string>();

        var entry = archive.GetEntry("xl/styles.xml");
        if (entry is null)
        {
            return formats;
        }

        using var stream = entry.Open();
        var document = XDocument.Load(stream);

        if (document.Root is null)
        {
            return formats;
        }

        var ns = XNamespace.Get(SpreadsheetMl.MainNamespace);
        var custom = new Dictionary<int, string>();

        if (document.Root.Element(ns + "numFmts") is { } numberFormats)
        {
            foreach (var format in numberFormats.Elements(ns + "numFmt"))
            {
                var id = (string?)format.Attribute("numFmtId");
                var code = (string?)format.Attribute("formatCode");

                if (id is not null && code is not null && int.TryParse(id, out var numberFormatId))
                {
                    custom[numberFormatId] = code;
                }
            }
        }

        if (document.Root.Element(ns + "cellXfs") is { } cellFormats)
        {
            var index = 0;

            foreach (var cellFormat in cellFormats.Elements(ns + "xf"))
            {
                var attribute = (string?)cellFormat.Attribute("numFmtId");

                if (attribute is not null &&
                    int.TryParse(attribute, out var numberFormatId) &&
                    ResolveFormatCode(numberFormatId, custom) is { } code)
                {
                    formats[index] = code;
                }

                index++;
            }
        }

        return formats;
    }

    /// <summary>
    /// The format code behind a numFmtId: the workbook's own code when the file carries one, else one
    /// of the ids the specification fixes. An unrecognised id is left alone rather than guessed at,
    /// so the cell keeps the plain rendering instead of being given a wrong one.
    /// </summary>
    private static string? ResolveFormatCode(int numberFormatId, Dictionary<int, string> custom)
    {
        if (custom.TryGetValue(numberFormatId, out var code))
        {
            return code;
        }

        return BuiltInNumberFormats.TryGetValue(numberFormatId, out var builtIn) ? builtIn : null;
    }
}
