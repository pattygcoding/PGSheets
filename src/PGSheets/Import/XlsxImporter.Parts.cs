using System.Globalization;
using System.IO.Compression;
using System.Text;
using System.Xml;
using System.Xml.Linq;

namespace PGSheets.Import;

public sealed partial class XlsxImporter
{
    private static List<(string Name, string Path)> ReadWorkbookSheets(ZipArchive archive, ImportResult result)
    {
        var sheets = new List<(string Name, string Path)>();

        var relationships = ReadRelationships(archive, "xl/_rels/workbook.xml.rels", "xl/");

        var workbookEntry = archive.GetEntry("xl/workbook.xml");
        if (workbookEntry is null)
        {
            result.AddWarning("xl/workbook.xml is missing from the archive.");
            return sheets;
        }

        using var stream = workbookEntry.Open();
        var document = XDocument.Load(stream);

        var main = XNamespace.Get(MainNamespace);
        var relationshipsNs = XNamespace.Get(RelationshipNamespace);

        foreach (var sheet in document.Descendants(main + "sheet"))
        {
            var name = (string?)sheet.Attribute("name") ?? "Sheet1";
            var relationshipId = (string?)sheet.Attribute(relationshipsNs + "id");

            var path = relationshipId is not null && relationships.TryGetValue(relationshipId, out var target)
                ? target
                : "xl/worksheets/sheet1.xml";

            sheets.Add((name, path));
        }

        return sheets;
    }

    private static Dictionary<string, string> ReadRelationships(ZipArchive archive, string partPath, string baseDirectory)
    {
        var map = new Dictionary<string, string>(StringComparer.Ordinal);

        var entry = archive.GetEntry(partPath);
        if (entry is null)
        {
            return map;
        }

        using var stream = entry.Open();
        var document = XDocument.Load(stream);

        if (document.Root is null)
        {
            return map;
        }

        foreach (var relationship in document.Root.Elements())
        {
            var id = (string?)relationship.Attribute("Id");
            var target = (string?)relationship.Attribute("Target");

            if (id is null || target is null)
            {
                continue;
            }

            map[id] = target.StartsWith('/') ? target[1..] : baseDirectory + target;
        }

        return map;
    }

    /// <summary>Reads the shared string table with a streaming reader so large workbooks stay cheap.</summary>
    private static List<string> ReadSharedStrings(ZipArchive archive)
    {
        var strings = new List<string>();

        var entry = archive.GetEntry("xl/sharedStrings.xml");
        if (entry is null)
        {
            return strings;
        }

        using var stream = entry.Open();
        using var reader = XmlReader.Create(stream, XmlSettings);

        while (reader.Read())
        {
            if (reader.NodeType == XmlNodeType.Element && reader.LocalName == "si" && !reader.IsEmptyElement)
            {
                using var subtree = reader.ReadSubtree();
                subtree.Read();
                strings.Add(ReadTextRuns(subtree));
            }
        }

        return strings;
    }

    /// <summary>Concatenates every &lt;t&gt; run inside a shared string (rich text is flattened).</summary>
    private static string ReadTextRuns(XmlReader reader)
    {
        var builder = new StringBuilder();

        while (reader.Read())
        {
            if (reader.NodeType == XmlNodeType.Element && reader.LocalName == "t" && !reader.IsEmptyElement)
            {
                builder.Append(ReadElementText(reader));
            }
        }

        return builder.ToString();
    }

    internal static string FormatDateSerial(double serial)
    {
        if (serial <= 0d)
        {
            return serial.ToString(CultureInfo.InvariantCulture);
        }

        try
        {
            var date = DateTime.FromOADate(serial);
            return date.TimeOfDay == TimeSpan.Zero
                ? date.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture)
                : date.ToString("yyyy-MM-dd HH:mm", CultureInfo.InvariantCulture);
        }
        catch (ArgumentException)
        {
            return serial.ToString(CultureInfo.InvariantCulture);
        }
    }
}
