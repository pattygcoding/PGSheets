using PGSheets.Import;

namespace PGSheets.Tests;

public class XlsxImporterTests
{
    [Fact]
    public async Task Reads_inline_strings_shared_strings_and_numbers()
    {
        await using var stream = XlsxFixtureBuilder.Create();
        var sink = new RecordingSink();

        var result = await new XlsxImporter().ImportAsync(stream, sink);

        Assert.Equal("Item", sink.Get("A1"));
        Assert.Equal("Apples", sink.Get("A2"));
        Assert.Equal("Bananas", sink.Get("A3"));
        Assert.Equal("Total", sink.Get("A4"));
        Assert.Equal("10", sink.Get("B2"));
        Assert.Equal("20", sink.Get("B3"));

        Assert.Equal("Sheet1", result.SheetName);
        Assert.Equal(4, result.Rows);
        Assert.Equal(3, result.Columns);
        Assert.Empty(result.Warnings);
    }

    [Fact]
    public async Task Imports_formulas_as_formulas_by_default()
    {
        await using var stream = XlsxFixtureBuilder.Create();
        var sink = new RecordingSink();

        await new XlsxImporter().ImportAsync(stream, sink);

        Assert.Equal("=SUM(B2:B3)", sink.Get("B4"));
    }

    [Fact]
    public async Task Can_import_cached_values_instead_of_formulas()
    {
        await using var stream = XlsxFixtureBuilder.Create();
        var sink = new RecordingSink();

        await new XlsxImporter { ImportFormulas = false }.ImportAsync(stream, sink);

        Assert.Equal("30", sink.Get("B4"));
    }

    [Fact]
    public async Task A_date_cell_keeps_its_serial_and_gains_the_format_that_reads_it()
    {
        await using var stream = XlsxFixtureBuilder.Create();
        var sink = new RecordingSink();

        await new XlsxImporter().ImportAsync(stream, sink);

        // The value stays the number the file holds, so it sorts and does arithmetic like a date
        // should, and the format is what makes the grid show 03-15-23. Turning it into text on the
        // way in — which is what this used to do — threw away both.
        Assert.Equal("45000", sink.Get("C1"));
        Assert.Equal("mm-dd-yy", sink.GetFormat("C1"));

        var model = Sheet.CreateModel();
        Sheet.Set(model, "C1", sink.Get("C1"));
        model.SetNumberFormat(0, 2, sink.GetFormat("C1"));

        Assert.Equal("03-15-23", Sheet.Display(model, "C1"));
    }

    [Fact]
    public async Task A_cell_with_no_format_carries_none()
    {
        await using var stream = XlsxFixtureBuilder.Create();
        var sink = new RecordingSink();

        await new XlsxImporter().ImportAsync(stream, sink);

        // Style 0 is General, which is the absence of a format rather than a format.
        Assert.Null(sink.GetFormat("B2"));
        Assert.Null(sink.GetFormat("A1"));
    }

    [Fact]
    public async Task A_custom_number_format_comes_across()
    {
        var parts = XlsxFixtureBuilder.MutableParts();
        parts["xl/styles.xml"] = XlsxFixtureBuilder.StylesWithCustomFormat;

        await using var stream = XlsxFixtureBuilder.CreateFromParts(parts);
        var sink = new RecordingSink();

        await new XlsxImporter().ImportAsync(stream, sink);

        Assert.Equal("0.0%", sink.GetFormat("C1"));

        var model = Sheet.CreateModel();
        Sheet.Set(model, "C1", "0.155");
        model.SetNumberFormat(0, 2, "0.0%");

        Assert.Equal("15.5%", Sheet.Display(model, "C1"));
    }

    [Fact]
    public async Task End_to_end_import_evaluates_the_imported_formula()
    {
        await using var stream = XlsxFixtureBuilder.Create();
        var model = Sheet.CreateModel();

        await new XlsxImporter().ImportAsync(stream, model);

        Assert.Equal("30", Sheet.Display(model, "B4"));
    }

    [Fact]
    public void Facade_resolves_an_importer_by_extension()
    {
        Assert.IsType<XlsxImporter>(SheetImporter.Resolve("book.xlsx"));
        Assert.IsType<XlsxImporter>(SheetImporter.Resolve("BOOK.XLSX"));
        Assert.IsType<CsvImporter>(SheetImporter.Resolve("data.csv"));
        Assert.IsType<CsvImporter>(SheetImporter.Resolve("notes.txt"));
        Assert.Null(SheetImporter.Resolve("photo.png"));

        Assert.Contains("*.xlsx", SheetImporter.AllPatterns);
        Assert.Contains("*.csv", SheetImporter.AllPatterns);
    }

    [Fact]
    public async Task The_facade_rejects_an_unknown_extension()
    {
        var model = Sheet.CreateModel();
        await using var stream = new MemoryStream(new byte[] { 1, 2, 3 });

        await Assert.ThrowsAsync<NotSupportedException>(
            () => SheetImporter.ImportAsync(stream, "photo.png", model));
    }

    [Fact]
    public async Task A_missing_workbook_part_is_reported()
    {
        var parts = XlsxFixtureBuilder.MutableParts();
        parts.Remove("xl/workbook.xml");

        await using var stream = XlsxFixtureBuilder.CreateFromParts(parts);

        var result = await new XlsxImporter().ImportAsync(stream, new RecordingSink());

        Assert.Equal(0, result.Rows);
        Assert.Contains(result.Warnings, warning => warning.Contains("workbook.xml"));
    }

    [Fact]
    public async Task A_missing_worksheet_part_is_reported()
    {
        var parts = XlsxFixtureBuilder.MutableParts();
        parts.Remove("xl/worksheets/sheet1.xml");

        await using var stream = XlsxFixtureBuilder.CreateFromParts(parts);

        var result = await new XlsxImporter().ImportAsync(stream, new RecordingSink());

        Assert.Equal(0, result.Rows);
        Assert.Contains(result.Warnings, warning => warning.Contains("is missing from the archive"));
    }

    [Fact]
    public async Task A_workbook_without_sheets_is_reported()
    {
        var parts = XlsxFixtureBuilder.MutableParts();
        parts["xl/workbook.xml"] = XlsxFixtureBuilder.WorkbookNoSheets;

        await using var stream = XlsxFixtureBuilder.CreateFromParts(parts);

        var result = await new XlsxImporter().ImportAsync(stream, new RecordingSink());

        Assert.Contains(result.Warnings, warning => warning.Contains("does not contain any worksheets"));
    }

    [Fact]
    public async Task Extra_sheets_are_reported_and_the_first_is_imported()
    {
        var parts = XlsxFixtureBuilder.MutableParts();
        parts["xl/workbook.xml"] = XlsxFixtureBuilder.WorkbookTwoSheets;
        parts["xl/_rels/workbook.xml.rels"] = XlsxFixtureBuilder.WorkbookRelationshipsTwoSheets;

        await using var stream = XlsxFixtureBuilder.CreateFromParts(parts);
        var sink = new RecordingSink();

        var result = await new XlsxImporter().ImportAsync(stream, sink);

        Assert.Equal("Sheet1", result.SheetName);
        Assert.Equal("Item", sink.Get("A1"));
        Assert.Contains(result.Warnings, warning => warning.Contains("only 'Sheet1' was imported"));

        // The count is what lets the host refuse to overwrite the file it came from.
        Assert.Equal(2, result.WorksheetCount);
    }

    [Fact]
    public async Task A_single_worksheet_workbook_reports_one_worksheet()
    {
        await using var stream = XlsxFixtureBuilder.Create();
        var sink = new RecordingSink();

        var result = await new XlsxImporter().ImportAsync(stream, sink);

        Assert.Equal(1, result.WorksheetCount);
    }

    [Fact]
    public async Task Unsupported_functions_are_reported_as_warnings()
    {
        var parts = XlsxFixtureBuilder.MutableParts();
        parts["xl/worksheets/sheet1.xml"] = XlsxFixtureBuilder.WorksheetWithUnsupportedFunction;

        await using var stream = XlsxFixtureBuilder.CreateFromParts(parts);
        var sink = new RecordingSink();

        var result = await new XlsxImporter().ImportAsync(stream, sink);

        Assert.Equal("=WEBSERVICE(B1)", sink.Get("A1"));
        Assert.Contains(result.Warnings, warning => warning.Contains("WEBSERVICE"));

        // The _xlfn. prefix is stripped so the reported name is readable.
        Assert.Contains(result.Warnings, warning => warning.Contains("UNICHAR"));
        Assert.DoesNotContain(result.Warnings, warning => warning.Contains("_xlfn"));
    }

    [Fact]
    public async Task Supported_functions_are_not_reported_as_warnings()
    {
        var parts = XlsxFixtureBuilder.MutableParts();
        parts["xl/worksheets/sheet1.xml"] = XlsxFixtureBuilder.WorksheetWithFunctions;

        await using var stream = XlsxFixtureBuilder.CreateFromParts(parts);
        var sink = new RecordingSink();

        var result = await new XlsxImporter().ImportAsync(stream, sink);

        Assert.Equal("=VLOOKUP(B1,1,2,0)", sink.Get("A1"));
        Assert.Equal("=TEXTJOIN(\",\",TRUE,B1)", sink.Get("A2"));
        Assert.Empty(result.Warnings);
    }

    [Fact]
    public async Task Unsupported_functions_are_not_reported_when_importing_cached_values()
    {
        var parts = XlsxFixtureBuilder.MutableParts();
        parts["xl/worksheets/sheet1.xml"] = XlsxFixtureBuilder.WorksheetWithUnsupportedFunction;

        await using var stream = XlsxFixtureBuilder.CreateFromParts(parts);
        var sink = new RecordingSink();

        var result = await new XlsxImporter { ImportFormulas = false }.ImportAsync(stream, sink);

        Assert.Equal("0", sink.Get("A1"));
        Assert.Empty(result.Warnings);
    }
}
