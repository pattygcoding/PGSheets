using System.IO.Compression;
using System.Text;

namespace PGSheets.Tests;

/// <summary>
/// Writes a minimal but valid xlsx (zip + SpreadsheetML parts) entirely with the BCL, so the xlsx
/// tests do not depend on any third-party reader/writer package.
/// </summary>
internal static class XlsxFixtureBuilder
{
    /// <summary>A numeric cell with a date number format applied, on 1-Jan-1900 epoch.</summary>
    public const double DateSerial = 45000d;

    /// <summary>The parts of a valid, minimal workbook. Copy and mutate these to build fixtures.</summary>
    public static IReadOnlyDictionary<string, string> DefaultParts { get; } =
        new Dictionary<string, string>(StringComparer.Ordinal)
        {
            ["[Content_Types].xml"] = ContentTypes,
            ["_rels/.rels"] = RootRelationships,
            ["xl/workbook.xml"] = Workbook,
            ["xl/_rels/workbook.xml.rels"] = WorkbookRelationships,
            ["xl/styles.xml"] = Styles,
            ["xl/sharedStrings.xml"] = SharedStrings,
            ["xl/worksheets/sheet1.xml"] = Worksheet,
        };

    /// <summary>A mutable copy of <see cref="DefaultParts"/> for tests that alter a single part.</summary>
    public static Dictionary<string, string> MutableParts() =>
        new(DefaultParts, StringComparer.Ordinal);

    public static MemoryStream Create() => CreateFromParts(DefaultParts);

    public static MemoryStream CreateFromParts(IEnumerable<KeyValuePair<string, string>> parts)
    {
        var stream = new MemoryStream();

        using (var archive = new ZipArchive(stream, ZipArchiveMode.Create, leaveOpen: true))
        {
            foreach (var part in parts)
            {
                Add(archive, part.Key, part.Value);
            }
        }

        stream.Position = 0;
        return stream;
    }

    private static void Add(ZipArchive archive, string path, string content)
    {
        var entry = archive.CreateEntry(path, CompressionLevel.Optimal);

        using var writer = new StreamWriter(entry.Open(), new UTF8Encoding(false));
        writer.Write(content);
    }

    private const string ContentTypes = """
        <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
          <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
          <Default Extension="xml" ContentType="application/xml"/>
          <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
          <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
          <Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
          <Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>
        </Types>
        """;

    private const string RootRelationships = """
        <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
          <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
        </Relationships>
        """;

    private const string Workbook = """
        <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
                  xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
          <sheets>
            <sheet name="Sheet1" sheetId="1" r:id="rId1"/>
          </sheets>
        </workbook>
        """;

    private const string WorkbookRelationships = """
        <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
          <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
          <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
          <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>
        </Relationships>
        """;

    private const string Styles = """
        <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <cellXfs count="2">
            <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
            <xf numFmtId="14" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>
          </cellXfs>
        </styleSheet>
        """;

    /// <summary>
    /// The same sheet with a workbook-defined number format instead of a built-in id, which is how a
    /// real file carries anything the specification does not fix.
    /// </summary>
    public const string StylesWithCustomFormat = """
        <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <numFmts count="1">
            <numFmt numFmtId="164" formatCode="0.0%"/>
          </numFmts>
          <cellXfs count="2">
            <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
            <xf numFmtId="164" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>
          </cellXfs>
        </styleSheet>
        """;

    private const string SharedStrings = """
        <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="3" uniqueCount="3">
          <si><t>Apples</t></si>
          <si><t>Bananas</t></si>
          <si><r><rPr><b/></rPr><t>Tot</t></r><r><t>al</t></r></si>
        </sst>
        """;

    private const string Worksheet = """
        <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <sheetData>
            <row r="1">
              <c r="A1" t="inlineStr"><is><t>Item</t></is></c>
              <c r="B1" t="inlineStr"><is><t>Amount</t></is></c>
              <c r="C1" s="1"><v>45000</v></c>
            </row>
            <row r="2">
              <c r="A2" t="s"><v>0</v></c>
              <c r="B2"><v>10</v></c>
            </row>
            <row r="3">
              <c r="A3" t="s"><v>1</v></c>
              <c r="B3"><v>20</v></c>
            </row>
            <row r="4">
              <c r="A4" t="s"><v>2</v></c>
              <c r="B4"><f>SUM(B2:B3)</f><v>30</v></c>
            </row>
          </sheetData>
        </worksheet>
        """;

    public const string WorkbookNoSheets = """
        <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
                  xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
          <sheets/>
        </workbook>
        """;

    public const string WorkbookTwoSheets = """
        <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
                  xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
          <sheets>
            <sheet name="Sheet1" sheetId="1" r:id="rId1"/>
            <sheet name="Sheet2" sheetId="2" r:id="rId2"/>
          </sheets>
        </workbook>
        """;

    public const string WorkbookRelationshipsTwoSheets = """
        <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
          <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
          <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/>
          <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
          <Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>
        </Relationships>
        """;

    public const string WorksheetWithUnsupportedFunction = """
        <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <sheetData>
            <row r="1">
              <c r="A1"><f>WEBSERVICE(B1)</f><v>0</v></c>
            </row>
            <row r="2">
              <c r="A2"><f>_xlfn.UNICHAR(65)</f><v>0</v></c>
            </row>
          </sheetData>
        </worksheet>
        """;

    /// <summary>
    /// A worksheet whose formulas only call functions the engine implements, so importing it must
    /// produce no warnings at all.
    /// </summary>
    public const string WorksheetWithFunctions = """
        <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <sheetData>
            <row r="1">
              <c r="A1"><f>VLOOKUP(B1,1,2,0)</f><v>0</v></c>
            </row>
            <row r="2">
              <c r="A2"><f>TEXTJOIN(",",TRUE,B1)</f><v>0</v></c>
            </row>
            <row r="3">
              <c r="A3"><f>IFERROR(A1/0,"none")</f><v>none</v></c>
            </row>
          </sheetData>
        </worksheet>
        """;
}
