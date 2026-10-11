using System.IO.Compression;
using System.Text;
using System.Xml.Linq;
using PGSheets.Core;
using PGSheets.Export;

namespace PGSheets.Tests;

/// <summary>
/// Structural checks on the hand-rolled xlsx writer. These prove the parts exist and are
/// well-formed; whether another application accepts the file is a manual check.
/// </summary>
public class XlsxWriterTests
{
    private static readonly XNamespace Ns = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

    private static async Task<byte[]> WriteAsync(
        SpreadsheetModel model,
        string sheetName = "Sheet1",
        bool values = false)
    {
        using var stream = new MemoryStream();

        await new XlsxWriter().ExportAsync(
            stream,
            "book.xlsx",
            model,
            new ExportOptions { SheetName = sheetName, ExportValues = values });

        return stream.ToArray();
    }

    private static string[] PartNames(byte[] bytes)
    {
        using var archive = new ZipArchive(new MemoryStream(bytes), ZipArchiveMode.Read);
        return archive.Entries.Select(entry => entry.FullName).ToArray();
    }

    private static string ReadPart(byte[] bytes, string path)
    {
        using var archive = new ZipArchive(new MemoryStream(bytes), ZipArchiveMode.Read);

        var entry = archive.GetEntry(path);
        Assert.NotNull(entry);

        using var stream = entry.Open();
        using var reader = new StreamReader(stream, Encoding.UTF8);

        return reader.ReadToEnd();
    }

    private static XDocument ParsePart(byte[] bytes, string path) => XDocument.Parse(ReadPart(bytes, path));

    private static string CellText(byte[] bytes) =>
        ParsePart(bytes, "xl/worksheets/sheet1.xml").Descendants(Ns + "t").Single().Value;

    [Fact]
    public async Task Writes_every_part_a_workbook_needs()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "Item");

        var names = PartNames(await WriteAsync(model));

        Assert.Contains("[Content_Types].xml", names);
        Assert.Contains("_rels/.rels", names);
        Assert.Contains("xl/workbook.xml", names);
        Assert.Contains("xl/_rels/workbook.xml.rels", names);
        Assert.Contains("xl/styles.xml", names);
        Assert.Contains("xl/worksheets/sheet1.xml", names);
    }

    [Fact]
    public async Task Every_part_is_well_formed_xml()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "Item");
        Sheet.Set(model, "B1", "1200");
        Sheet.Set(model, "C1", "=B1*2");

        var bytes = await WriteAsync(model);

        foreach (var name in PartNames(bytes))
        {
            Assert.NotNull(ParsePart(bytes, name).Root);
        }
    }

    [Fact]
    public async Task Style_counts_match_their_children()
    {
        // Excel refuses a workbook whose style table is internally inconsistent, so this guards
        // the single most likely cause of a "needs repair" prompt.
        var styles = ParsePart(await WriteAsync(Sheet.CreateModel()), "xl/styles.xml");

        foreach (var group in new[] { "fonts", "fills", "borders", "cellStyleXfs", "cellXfs", "cellStyles" })
        {
            var element = styles.Root!.Element(Ns + group);

            Assert.NotNull(element);
            Assert.Equal(element.Elements().Count().ToString(), (string?)element.Attribute("count"));
        }
    }

    [Fact]
    public async Task The_style_table_has_the_two_fills_excel_requires()
    {
        var styles = ParsePart(await WriteAsync(Sheet.CreateModel()), "xl/styles.xml");

        Assert.Equal(2, styles.Root!.Element(Ns + "fills")!.Elements().Count());
    }

    [Fact]
    public async Task Literals_are_written_with_the_type_that_matches_them()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "1200");
        Sheet.Set(model, "A2", "TRUE");
        Sheet.Set(model, "A3", "text");

        var sheet = ReadPart(await WriteAsync(model), "xl/worksheets/sheet1.xml");

        Assert.Contains("<v>1200</v>", sheet);
        Assert.Contains("t=\"b\"", sheet);
        Assert.Contains("t=\"inlineStr\"", sheet);
    }

    [Fact]
    public async Task Markup_characters_in_text_survive()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "<a & \"b\">");

        Assert.Equal("<a & \"b\">", CellText(await WriteAsync(model)));
    }

    [Fact]
    public async Task Characters_xml_cannot_represent_are_stripped()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "a\u0001b\u0000c");

        Assert.Equal("abc", CellText(await WriteAsync(model)));
    }

    [Fact]
    public async Task Astral_characters_are_preserved()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "😀 build");

        Assert.Equal("😀 build", CellText(await WriteAsync(model)));
    }

    [Fact]
    public async Task Whitespace_is_flagged_for_preservation()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "  padded  ");
        Sheet.Set(model, "A2", "   ");

        var sheet = ReadPart(await WriteAsync(model), "xl/worksheets/sheet1.xml");

        Assert.Contains("xml:space=\"preserve\"", sheet);
    }

    [Fact]
    public async Task Formulas_are_written_without_the_equals_sign()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "2");
        Sheet.Set(model, "B1", "=A1*3");

        var sheet = ReadPart(await WriteAsync(model), "xl/worksheets/sheet1.xml");

        Assert.Contains("<f>A1*3</f>", sheet);
        Assert.Contains("<v>6</v>", sheet);
    }

    [Fact]
    public async Task Error_results_are_not_cached_so_the_reader_recalculates()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "=1/0");

        var sheet = ReadPart(await WriteAsync(model), "xl/worksheets/sheet1.xml");

        Assert.Contains("<f>1/0</f>", sheet);
        Assert.DoesNotContain("<v>", sheet);
    }

    [Fact]
    public async Task Unsupported_functions_keep_their_formula_text()
    {
        // The local engine reports #NAME?, but the formula must survive so an application that
        // does implement the function can evaluate it properly.
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "=VLOOKUP(B1,1,2,0)");

        var sheet = ReadPart(await WriteAsync(model), "xl/worksheets/sheet1.xml");

        Assert.Contains("<f>VLOOKUP(B1,1,2,0)</f>", sheet);
        Assert.DoesNotContain("<v>", sheet);
    }

    [Fact]
    public async Task Value_mode_still_writes_the_formula()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "2");
        Sheet.Set(model, "B1", "=A1*3");

        var sheet = ReadPart(await WriteAsync(model, values: true), "xl/worksheets/sheet1.xml");

        Assert.Contains("<f>A1*3</f>", sheet);
    }

    [Fact]
    public async Task The_worksheet_name_is_sanitised()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "x");

        var workbook = ParsePart(
            await WriteAsync(model, "a/b:c*d[e]f?g\\h and a very long tail beyond thirty one chars"),
            "xl/workbook.xml");

        var name = (string?)workbook.Descendants(Ns + "sheet").Single().Attribute("name");

        Assert.NotNull(name);
        Assert.StartsWith("abcdefgh", name);
        Assert.True(name!.Length <= 31, $"'{name}' is {name.Length} characters.");

        foreach (var illegal in new[] { '/', ':', '*', '[', ']', '?', '\\' })
        {
            Assert.DoesNotContain(illegal, name);
        }
    }

    [Fact]
    public async Task Renaming_a_sheet_is_reported()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "x");

        using var stream = new MemoryStream();
        var result = await new XlsxWriter().ExportAsync(
            stream,
            "book.xlsx",
            model,
            new ExportOptions { SheetName = "bad/name" });

        Assert.Contains(result.Warnings, warning => warning.Contains("not valid in xlsx"));
    }

    [Fact]
    public async Task A_valid_sheet_name_is_not_reported()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "x");

        using var stream = new MemoryStream();
        var result = await new XlsxWriter().ExportAsync(
            stream,
            "book.xlsx",
            model,
            new ExportOptions { SheetName = "Quarter 1" });

        Assert.Empty(result.Warnings);
    }

    [Fact]
    public async Task An_empty_sheet_still_produces_a_readable_workbook()
    {
        var bytes = await WriteAsync(Sheet.CreateModel());

        Assert.Contains("<sheetData", ReadPart(bytes, "xl/worksheets/sheet1.xml"));
    }

    [Fact]
    public async Task The_result_describes_the_written_extent()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "a");
        Sheet.Set(model, "B2", "b");

        using var stream = new MemoryStream();
        var result = await new XlsxWriter().ExportAsync(stream, "book.xlsx", model, new ExportOptions());

        Assert.Equal(2, result.Rows);
        Assert.Equal(2, result.Columns);
        Assert.Equal(2, result.CellsWritten);
    }

    [Fact]
    public async Task Rows_are_written_in_ascending_order()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A5", "last");
        Sheet.Set(model, "A2", "first");

        var sheet = ParsePart(await WriteAsync(model), "xl/worksheets/sheet1.xml");

        var rows = sheet.Descendants(Ns + "row").Select(row => (string?)row.Attribute("r")).ToArray();

        Assert.Equal(new[] { "2", "5" }, rows);
    }

    // ---- Packaging checks: the mechanical reasons a reader rejects a workbook ------------------

    [Fact]
    public async Task Every_part_is_declared_in_the_content_types()
    {
        // A part with no content type is a packaging error, and readers refuse the file.
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "x");

        var bytes = await WriteAsync(model);
        var types = ParsePart(bytes, "[Content_Types].xml");
        XNamespace ct = "http://schemas.openxmlformats.org/package/2006/content-types";

        var overrides = types.Root!
            .Elements(ct + "Override")
            .Select(element => ((string?)element.Attribute("PartName"))?.TrimStart('/'))
            .ToHashSet(StringComparer.OrdinalIgnoreCase);

        var defaults = types.Root!
            .Elements(ct + "Default")
            .Select(element => "." + (string?)element.Attribute("Extension"))
            .ToHashSet(StringComparer.OrdinalIgnoreCase);

        foreach (var name in PartNames(bytes))
        {
            if (name == "[Content_Types].xml")
            {
                continue;
            }

            Assert.True(
                overrides.Contains(name) || defaults.Contains(Path.GetExtension(name)),
                $"'{name}' has no content type declaration.");
        }
    }

    [Fact]
    public async Task Workbook_relationships_resolve_to_real_parts()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "x");

        var bytes = await WriteAsync(model);
        var names = PartNames(bytes);

        var relationships = ParsePart(bytes, "xl/_rels/workbook.xml.rels");
        XNamespace pr = "http://schemas.openxmlformats.org/package/2006/relationships";

        foreach (var relationship in relationships.Root!.Elements(pr + "Relationship"))
        {
            var target = (string?)relationship.Attribute("Target");

            Assert.NotNull(target);
            Assert.Contains("xl/" + target, names);
        }
    }

    [Fact]
    public async Task The_workbook_points_at_a_relationship_that_exists()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "x");

        var bytes = await WriteAsync(model);

        XNamespace r = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
        XNamespace pr = "http://schemas.openxmlformats.org/package/2006/relationships";

        var workbook = ParsePart(bytes, "xl/workbook.xml");
        var id = (string?)workbook.Descendants(Ns + "sheet").Single().Attribute(r + "id");

        Assert.NotNull(id);

        var relationships = ParsePart(bytes, "xl/_rels/workbook.xml.rels");
        var ids = relationships.Root!.Elements(pr + "Relationship").Select(element => (string?)element.Attribute("Id"));

        Assert.Contains(id, ids);
    }

    [Fact]
    public async Task Entry_names_use_forward_slashes()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "x");

        foreach (var name in PartNames(await WriteAsync(model)))
        {
            Assert.DoesNotContain('\\', name);
        }
    }

    [Fact]
    public async Task A_number_format_becomes_a_style_the_workbook_can_reference()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "0.155");
        model.SetNumberFormat(0, 0, "0.0%");
        Sheet.Set(model, "B1", "plain");

        var bytes = await WriteAsync(model);

        var styles = XDocument.Parse(ReadPart(bytes, "xl/styles.xml"));

        var numberFormats = styles.Root!.Element(Ns + "numFmts")!;
        Assert.Equal("1", (string?)numberFormats.Attribute("count"));

        var format = numberFormats.Elements(Ns + "numFmt").Single();
        Assert.Equal("164", (string?)format.Attribute("numFmtId"));
        Assert.Equal("0.0%", (string?)format.Attribute("formatCode"));

        // Entry 0 is General and entry 1 is the format, so the counts have to agree.
        var cellFormats = styles.Root.Element(Ns + "cellXfs")!;
        Assert.Equal("2", (string?)cellFormats.Attribute("count"));
        Assert.Equal("164", (string?)cellFormats.Elements(Ns + "xf").Last().Attribute("numFmtId"));

        var sheet = XDocument.Parse(ReadPart(bytes, "xl/worksheets/sheet1.xml"));
        var cells = sheet.Descendants(Ns + "c").ToList();

        Assert.Equal("1", (string?)cells.Single(cell => (string?)cell.Attribute("r") == "A1").Attribute("s"));

        // A cell with no format leaves the attribute off, which means the default entry.
        Assert.Null((string?)cells.Single(cell => (string?)cell.Attribute("r") == "B1").Attribute("s"));
    }

    [Fact]
    public async Task One_entry_is_written_per_distinct_format()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "1");
        model.SetNumberFormat(0, 0, "0.00");
        Sheet.Set(model, "A2", "2");
        model.SetNumberFormat(1, 0, "0.00");
        Sheet.Set(model, "A3", "0.5");
        model.SetNumberFormat(2, 0, "0%");

        var bytes = await WriteAsync(model);
        var styles = XDocument.Parse(ReadPart(bytes, "xl/styles.xml"));

        // Two distinct codes, so two entries plus the default: the same format is not repeated.
        Assert.Equal("2", (string?)styles.Root!.Element(Ns + "numFmts")!.Attribute("count"));
        Assert.Equal("3", (string?)styles.Root.Element(Ns + "cellXfs")!.Attribute("count"));
    }

    [Fact]
    public async Task A_workbook_with_no_formats_writes_no_number_format_table()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "1");

        var bytes = await WriteAsync(model);
        var styles = XDocument.Parse(ReadPart(bytes, "xl/styles.xml"));

        // An empty numFmts element is legal but pointless, so it is left out entirely.
        Assert.Null(styles.Root!.Element(Ns + "numFmts"));
        Assert.Equal("1", (string?)styles.Root.Element(Ns + "cellXfs")!.Attribute("count"));
    }

    [Fact]
    public async Task A_format_on_a_formula_cell_is_written_too()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "2");
        Sheet.Set(model, "B1", "=A1*2");
        model.SetNumberFormat(0, 1, "0.00");

        var bytes = await WriteAsync(model);
        var sheet = XDocument.Parse(ReadPart(bytes, "xl/worksheets/sheet1.xml"));

        var cell = sheet.Descendants(Ns + "c").Single(c => (string?)c.Attribute("r") == "B1");

        Assert.Equal("1", (string?)cell.Attribute("s"));
        Assert.Equal("A1*2", cell.Element(Ns + "f")!.Value);
    }

    [Fact]
    public async Task The_style_table_children_are_in_the_order_the_specification_requires()
    {
        // A style table whose children are out of order is one Excel offers to repair, and numFmts
        // has to come first — before fonts.
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "0.5");
        model.SetNumberFormat(0, 0, "0%");

        var bytes = await WriteAsync(model);
        var styles = XDocument.Parse(ReadPart(bytes, "xl/styles.xml"));

        var order = styles.Root!.Elements().Select(element => element.Name.LocalName).ToList();

        Assert.Equal(
            new[] { "numFmts", "fonts", "fills", "borders", "cellStyleXfs", "cellXfs", "cellStyles" },
            order);
    }

    [Fact]
    public async Task Parts_carry_no_byte_order_mark()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "x");

        var bytes = await WriteAsync(model);

        using var archive = new ZipArchive(new MemoryStream(bytes), ZipArchiveMode.Read);

        foreach (var entry in archive.Entries)
        {
            using var stream = entry.Open();
            using var buffer = new MemoryStream();

            await stream.CopyToAsync(buffer);

            var content = buffer.ToArray();
            var hasMark = content.Length >= 3 && content[0] == 0xEF && content[1] == 0xBB && content[2] == 0xBF;

            Assert.False(hasMark, $"'{entry.FullName}' starts with a byte-order mark.");
        }
    }
}
