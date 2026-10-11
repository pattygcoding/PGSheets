using System.Text;
using PGSheets.Core;
using PGSheets.Export;
using PGSheets.Import;

namespace PGSheets.Tests;

public class CsvExporterTests
{
    private static (string Text, ExportResult Result) Build(
        SpreadsheetModel model,
        char delimiter = ',',
        bool values = false)
    {
        var result = new ExportResult();
        var text = CsvExporter.Build(model, new ExportOptions { ExportValues = values }, delimiter, result);
        return (text, result);
    }

    [Fact]
    public void Writes_the_used_extent_with_crlf_line_endings()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "Item");
        Sheet.Set(model, "B2", "5");

        Assert.Equal("Item,\r\n,5\r\n", Build(model).Text);
    }

    [Fact]
    public void Reports_the_extent_and_the_cell_count()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "a");
        Sheet.Set(model, "C3", "c");

        var (_, result) = Build(model);

        Assert.Equal(3, result.Rows);
        Assert.Equal(3, result.Columns);
        Assert.Equal(2, result.CellsWritten);
    }

    [Fact]
    public void An_empty_sheet_produces_an_empty_file()
    {
        var (text, result) = Build(Sheet.CreateModel());

        Assert.Equal(string.Empty, text);
        Assert.Equal(0, result.Rows);
        Assert.Equal(0, result.Columns);
    }

    [Fact]
    public void Quotes_a_field_containing_the_delimiter()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "a,b");

        Assert.Equal("\"a,b\"\r\n", Build(model).Text);
    }

    [Fact]
    public void Escapes_quotes_by_doubling_them()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "say \"hi\"");

        Assert.Equal("\"say \"\"hi\"\"\"\r\n", Build(model).Text);
    }

    [Fact]
    public void Quotes_a_field_containing_a_line_break()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "line1\nline2");

        Assert.Equal("\"line1\nline2\"\r\n", Build(model).Text);
    }

    [Fact]
    public void Quoting_and_escaping_round_trip_through_the_importer()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "a,b");
        Sheet.Set(model, "A2", "say \"hi\"");
        Sheet.Set(model, "A3", "line1\nline2");
        Sheet.Set(model, "A4", "  padded  ");

        var sink = new RecordingSink();
        CsvImporter.Parse(Build(model).Text, sink);

        Assert.Equal("a,b", sink.Get("A1"));
        Assert.Equal("say \"hi\"", sink.Get("A2"));
        Assert.Equal("line1\nline2", sink.Get("A3"));
        Assert.Equal("  padded  ", sink.Get("A4"));
    }

    [Fact]
    public void Tab_is_the_delimiter_when_requested()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "a");
        Sheet.Set(model, "B1", "b");

        Assert.Equal("a\tb\r\n", Build(model, '\t').Text);
    }

    [Fact]
    public async Task A_tsv_extension_selects_the_tab_delimiter()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "a");
        Sheet.Set(model, "B1", "b");

        using var stream = new MemoryStream();
        await new CsvExporter().ExportAsync(stream, "data.tsv", model, new ExportOptions());

        Assert.Contains("a\tb", Encoding.UTF8.GetString(stream.ToArray()));
    }

    [Fact]
    public async Task Writes_a_utf8_byte_order_mark()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "Ünïcödé");

        using var stream = new MemoryStream();
        await new CsvExporter().ExportAsync(stream, "data.csv", model, new ExportOptions());

        Assert.Equal(new byte[] { 0xEF, 0xBB, 0xBF }, stream.ToArray()[..3]);
    }

    [Fact]
    public void Raw_input_is_written_by_default_so_formulas_survive()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "2");
        Sheet.Set(model, "B1", "=A1*3");

        Assert.Equal("2,=A1*3\r\n", Build(model).Text);
    }

    [Fact]
    public void Value_mode_writes_the_computed_result()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "2");
        Sheet.Set(model, "B1", "=A1*3");

        Assert.Equal("2,6\r\n", Build(model, ',', values: true).Text);
    }

    [Fact]
    public void Value_mode_warns_about_text_that_will_be_read_back_as_a_number()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "=CONCAT(\"0\",\"07\")");

        var (_, result) = Build(model, ',', values: true);

        Assert.Contains(result.Warnings, warning => warning.Contains("look like numbers"));
    }

    [Fact]
    public void Raw_mode_never_warns_about_numbers()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "007");

        Assert.Empty(Build(model).Result.Warnings);
    }

    // ---- Range export (selection and clipboard) -----------------------------------------------

    [Fact]
    public void A_range_writes_just_that_rectangle()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "ignore");
        Sheet.Set(model, "B2", "keep");
        Sheet.Set(model, "C2", "also");

        var result = new ExportResult();
        var text = CsvExporter.Build(model, Range(new CellRange(1, 1, 1, 2)), ',', result);

        Assert.Equal("keep,also\r\n", text);
        Assert.Equal(1, result.Rows);
        Assert.Equal(2, result.Columns);
        Assert.Equal(2, result.CellsWritten);
    }

    [Fact]
    public void A_range_keeps_its_shape_when_cells_are_empty()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "B2", "only");

        var result = new ExportResult();
        var text = CsvExporter.Build(model, Range(new CellRange(1, 1, 2, 2)), ',', result);

        Assert.Equal("only,\r\n,\r\n", text);
        Assert.Equal(2, result.Rows);
        Assert.Equal(2, result.Columns);
        Assert.Equal(1, result.CellsWritten);
    }

    [Fact]
    public void A_range_round_trips_into_a_paste_buffer()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "B2", "a");
        Sheet.Set(model, "C2", "b");

        var text = CsvExporter.Build(model, Range(new CellRange(1, 1, 1, 2)), '\t', new ExportResult());

        var block = new CellBuffer();
        CsvImporter.Parse(text, block);

        Assert.True(block.TryGetBounds(out var bounds));
        Assert.Equal(new CellRange(0, 0, 0, 1), bounds);
        Assert.Equal("a", block.Get(0, 0));
        Assert.Equal("b", block.Get(0, 1));
    }

    [Fact]
    public void A_range_of_an_empty_region_writes_only_delimiters()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "Z9", "elsewhere");

        var result = new ExportResult();
        var text = CsvExporter.Build(model, Range(new CellRange(0, 0, 1, 1)), ',', result);

        Assert.Equal(",\r\n,\r\n", text);
        Assert.Equal(0, result.CellsWritten);
    }

    private static ExportOptions Range(CellRange range) => new() { Range = range };
}
