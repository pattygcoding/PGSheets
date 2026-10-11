using System.Xml.Linq;

namespace PGSheets.Import;

public sealed partial class XlsxImporter
{
    /// <summary>
    /// Builds a map of cell-style index to "is a date" by combining the built-in date formats with
    /// any custom number formats that look like dates.
    /// </summary>
    private static Dictionary<int, bool> ReadDateStyles(System.IO.Compression.ZipArchive archive)
    {
        var styles = new Dictionary<int, bool>();

        var entry = archive.GetEntry("xl/styles.xml");
        if (entry is null)
        {
            return styles;
        }

        using var stream = entry.Open();
        var document = XDocument.Load(stream);

        if (document.Root is null)
        {
            return styles;
        }

        var ns = XNamespace.Get(MainNamespace);

        // Built-in date/time number formats defined by the SpreadsheetML specification.
        var builtInDateFormats = new HashSet<int> { 14, 15, 16, 17, 18, 19, 20, 21, 22, 45, 46, 47 };
        var customDateFormats = new HashSet<int>();

        var numberFormats = document.Root.Element(ns + "numFmts");
        if (numberFormats is not null)
        {
            foreach (var format in numberFormats.Elements(ns + "numFmt"))
            {
                var idAttribute = (string?)format.Attribute("numFmtId");
                var code = (string?)format.Attribute("formatCode");

                if (idAttribute is not null &&
                    code is not null &&
                    int.TryParse(idAttribute, out var id) &&
                    LooksLikeDateFormat(code))
                {
                    customDateFormats.Add(id);
                }
            }
        }

        var cellFormats = document.Root.Element(ns + "cellXfs");
        if (cellFormats is not null)
        {
            var index = 0;

            foreach (var cellFormat in cellFormats.Elements(ns + "xf"))
            {
                var numberFormatAttribute = (string?)cellFormat.Attribute("numFmtId");

                if (numberFormatAttribute is not null && int.TryParse(numberFormatAttribute, out var numberFormatId))
                {
                    styles[index] = builtInDateFormats.Contains(numberFormatId) ||
                                    customDateFormats.Contains(numberFormatId);
                }

                index++;
            }
        }

        return styles;
    }

    /// <summary>
    /// Heuristic: a format code containing any of y/d/h/s/m (outside letters used by number formats)
    /// is treated as a date/time format.
    /// </summary>
    private static bool LooksLikeDateFormat(string formatCode)
    {
        foreach (var ch in formatCode)
        {
            switch (char.ToLowerInvariant(ch))
            {
                case 'y':
                case 'd':
                case 'h':
                case 's':
                case 'm':
                    return true;
            }
        }

        return false;
    }
}
