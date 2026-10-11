using PGSheets.Core;
using PGSheets.Export;
using PGSheets.Import;

namespace PGSheets.Tests;

/// <summary>
/// Open, edit, save, reopen. These are the fidelity guard: they prove the writer and the reader
/// agree with each other. They cannot prove that another application accepts the file, which is a
/// manual check.
/// </summary>
public class RoundTripTests
{
    /// <summary>
    /// A sheet covering every cell kind the model can hold, plus awkward text and formats. The CSV
    /// round trip asks for no formats, because a delimited file has nowhere to put one.
    /// </summary>
    private static SpreadsheetModel CreateRichModel(bool withFormats = true)
    {
        var model = Sheet.CreateModel();

        Sheet.Set(model, "A1", "Name");
        Sheet.Set(model, "B1", "Amount");
        Sheet.Set(model, "C1", "Doubled");

        Sheet.Set(model, "A2", "North & <West>");
        Sheet.Set(model, "B2", "1200");
        Sheet.Set(model, "C2", "=B2*2");

        Sheet.Set(model, "A3", "Ünïcödé 😀");
        Sheet.Set(model, "B3", "0.1");

        Sheet.Set(model, "A4", "TRUE");
        Sheet.Set(model, "B4", "   padded   ");
        Sheet.Set(model, "C4", "=SUM(B2:B3)");

        if (withFormats)
        {
            // Number formats travel with the values they describe, so a round trip has to keep both.
            model.SetNumberFormat(1, 1, "$#,##0.00");
            model.SetNumberFormat(3, 1, "0.0%");
            Sheet.Set(model, "A5", "45356");
            model.SetNumberFormat(4, 0, "yyyy-mm-dd");
            Sheet.Set(model, "B5", "0.155");
        }

        return model;
    }

    private static void AssertSameContent(SpreadsheetModel expected, SpreadsheetModel actual)
    {
        var expectedCells = expected.PopulatedCells.ToList();

        Assert.Equal(expectedCells.Count, actual.PopulatedCells.Count());

        foreach (var (row, column) in expectedCells)
        {
            Assert.Equal(expected.GetRawInput(row, column), actual.GetRawInput(row, column));
            Assert.Equal(expected.GetDisplayValue(row, column), actual.GetDisplayValue(row, column));
            Assert.Equal(expected.GetNumberFormat(row, column), actual.GetNumberFormat(row, column));
        }
    }

    private static async Task<byte[]> ExportXlsxAsync(SpreadsheetModel model, string sheetName = "Sheet1")
    {
        using var stream = new MemoryStream();

        await new XlsxWriter().ExportAsync(
            stream,
            "book.xlsx",
            model,
            new ExportOptions { SheetName = sheetName });

        return stream.ToArray();
    }

    private static async Task<SpreadsheetModel> ImportXlsxAsync(byte[] bytes)
    {
        using var stream = new MemoryStream(bytes);

        var model = Sheet.CreateModel();
        await new XlsxImporter().ImportAsync(stream, model);

        return model;
    }

    [Fact]
    public async Task Xlsx_survives_a_save_and_reload()
    {
        var original = CreateRichModel();

        var reloaded = await ImportXlsxAsync(await ExportXlsxAsync(original));

        AssertSameContent(original, reloaded);
    }

    [Fact]
    public async Task A_date_from_a_real_workbook_survives_a_save_and_reload()
    {
        // The fixture holds a serial with the built-in "mm-dd-yy" format, which is how a real file
        // carries a date. It used to arrive as text, which threw away both the number and the format.
        SpreadsheetModel loaded;

        await using (var stream = XlsxFixtureBuilder.Create())
        {
            loaded = Sheet.CreateModel();
            await new XlsxImporter().ImportAsync(stream, loaded);
        }

        Assert.Equal("45000", loaded.GetRawInput(0, 2));
        Assert.Equal("mm-dd-yy", loaded.GetNumberFormat(0, 2));
        Assert.Equal("03-15-23", loaded.GetDisplayValue(0, 2));

        var reloaded = await ImportXlsxAsync(await ExportXlsxAsync(loaded));

        Assert.Equal("45000", reloaded.GetRawInput(0, 2));
        Assert.Equal("mm-dd-yy", reloaded.GetNumberFormat(0, 2));
        Assert.Equal("03-15-23", reloaded.GetDisplayValue(0, 2));

        // The value is still a number, so a date can be added to like one.
        Sheet.Set(reloaded, "D1", "=C1+1");
        reloaded.SetNumberFormat(0, 3, "yyyy-mm-dd");

        Assert.Equal("2023-03-16", reloaded.GetDisplayValue(0, 3));
    }

    [Fact]
    public async Task Csv_survives_a_save_and_reload()
    {
        // Delimited text carries values, not formats, so this model has none to lose.
        var original = CreateRichModel(withFormats: false);

        using var stream = new MemoryStream();
        await new CsvExporter().ExportAsync(stream, "book.csv", original, new ExportOptions());
        stream.Position = 0;

        var reloaded = Sheet.CreateModel();
        await new CsvImporter().ImportAsync(stream, reloaded);

        AssertSameContent(original, reloaded);
    }

    [Fact]
    public async Task Re_exporting_an_imported_workbook_is_stable()
    {
        // Saving what we just loaded must not drift: the second load must equal the first.
        var firstLoad = await ImportXlsxAsync(await ExportXlsxAsync(CreateRichModel()));
        var secondLoad = await ImportXlsxAsync(await ExportXlsxAsync(firstLoad));

        AssertSameContent(firstLoad, secondLoad);
    }

    [Fact]
    public async Task The_worksheet_name_round_trips()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "x");

        using var stream = new MemoryStream(await ExportXlsxAsync(model, "Quarter 1"));

        var result = await new XlsxImporter().ImportAsync(stream, Sheet.CreateModel());

        Assert.Equal("Quarter 1", result.SheetName);
    }

    [Fact]
    public async Task Numeric_looking_literals_are_normalised_to_numbers()
    {
        // Documented behaviour, matching spreadsheets: "007" is the number 7, so the leading zeros
        // are not part of the value and are not preserved by a round trip.
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "007");

        var reloaded = await ImportXlsxAsync(await ExportXlsxAsync(model));

        Assert.Equal("7", reloaded.GetRawInput(0, 0));
    }

    [Fact]
    public async Task An_empty_workbook_round_trips()
    {
        var reloaded = await ImportXlsxAsync(await ExportXlsxAsync(Sheet.CreateModel()));

        Assert.Empty(reloaded.PopulatedCells);
    }

    [Fact]
    public async Task The_facade_round_trips_a_workbook_of_our_own_making()
    {
        // Guards the packaging itself: the parts the importer looks for must be exactly the parts
        // the writer emits, including the relationship that names the worksheet.
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "value");

        var reloaded = Sheet.CreateModel();

        using var stream = new MemoryStream(await ExportXlsxAsync(model));
        var result = await SheetImporter.ImportAsync(stream, "book.xlsx", reloaded);

        Assert.Equal("Sheet1", result.SheetName);
        Assert.Equal("value", reloaded.GetRawInput(0, 0));
        Assert.Equal(1, result.Rows);
        Assert.Empty(result.Warnings);
    }
}
