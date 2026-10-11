using PGSheets.Export;

namespace PGSheets.Tests;

public class SheetExporterTests
{
    [Fact]
    public void Resolves_a_writer_by_extension()
    {
        Assert.IsType<XlsxWriter>(SheetExporter.Resolve("book.xlsx"));
        Assert.IsType<XlsxWriter>(SheetExporter.Resolve("BOOK.XLSX"));
        Assert.IsType<CsvExporter>(SheetExporter.Resolve("data.csv"));
        Assert.IsType<CsvExporter>(SheetExporter.Resolve("data.tsv"));
        Assert.IsType<CsvExporter>(SheetExporter.Resolve("notes.txt"));
        Assert.Null(SheetExporter.Resolve("photo.png"));
    }

    [Fact]
    public void Macro_enabled_workbooks_are_readable_but_not_writable()
    {
        // .xlsm imports fine, but vbaProject.bin cannot be written, so Save must fall through to
        // Save As rather than producing a workbook that lies about containing macros.
        Assert.False(SheetExporter.CanExport("book.xlsm"));
    }

    [Theory]
    [InlineData(0, true)]
    [InlineData(1, true)]
    [InlineData(2, false)]
    [InlineData(9, false)]
    public void A_file_may_only_be_overwritten_when_one_worksheet_will_replace_it(int loadedWorksheetCount, bool expected)
    {
        // Saving truncates the target and writes a single worksheet, so overwriting a workbook that
        // held more would destroy the sheets that were never loaded.
        Assert.Equal(expected, SheetExporter.CanSaveInPlace(loadedWorksheetCount));
    }

    [Fact]
    public void Losing_worksheets_is_described_so_it_can_be_reported()
    {
        Assert.Null(SheetExporter.DescribeLostWorksheets(1, "Sheet1"));

        var lost = SheetExporter.DescribeLostWorksheets(3, "Data");

        Assert.NotNull(lost);
        Assert.Contains("Data", lost);
        Assert.Contains("2 worksheet(s)", lost);
    }

    [Fact]
    public async Task The_facade_rejects_an_unknown_extension()
    {
        using var stream = new MemoryStream();

        await Assert.ThrowsAsync<NotSupportedException>(
            () => SheetExporter.ExportAsync(stream, "photo.png", Sheet.CreateModel()));
    }

    [Fact]
    public void All_patterns_list_every_format()
    {
        Assert.Contains("*.xlsx", SheetExporter.AllPatterns);
        Assert.Contains("*.csv", SheetExporter.AllPatterns);
        Assert.Contains("*.tsv", SheetExporter.AllPatterns);
    }

    [Theory]
    [InlineData("book.xlsx", true)]
    [InlineData("book.XLSX", true)]
    [InlineData("data.CSV", true)]
    [InlineData("book.xlsm", false)]
    [InlineData("photo.png", false)]
    public void Can_export_ignores_extension_case(string fileName, bool expected)
    {
        Assert.Equal(expected, SheetExporter.CanExport(fileName));
    }

    [Fact]
    public async Task The_facade_writes_through_the_resolved_writer()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "only");

        using var stream = new MemoryStream();
        var result = await SheetExporter.ExportAsync(stream, "data.csv", model);

        Assert.Equal(1, result.CellsWritten);
        Assert.Contains("only", System.Text.Encoding.UTF8.GetString(stream.ToArray()));
    }
}
