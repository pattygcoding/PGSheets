using System.Text;
using PGSheets.Import;

namespace PGSheets.Tests;

public class CsvImporterTests
{
    [Fact]
    public void Parses_simple_rows_and_columns()
    {
        var sink = new RecordingSink();
        var result = CsvImporter.Parse("a,b\n1,2", sink);

        Assert.Equal("a", sink.Get("A1"));
        Assert.Equal("b", sink.Get("B1"));
        Assert.Equal("1", sink.Get("A2"));
        Assert.Equal("2", sink.Get("B2"));
        Assert.Equal(2, result.Rows);
        Assert.Equal(2, result.Columns);
    }

    [Fact]
    public void Handles_quoted_fields_with_commas_and_escaped_quotes()
    {
        var sink = new RecordingSink();
        CsvImporter.Parse("\"Smith, John\",\"He said \"\"hi\"\"\"", sink);

        Assert.Equal("Smith, John", sink.Get("A1"));
        Assert.Equal("He said \"hi\"", sink.Get("B1"));
    }

    [Fact]
    public void Handles_newlines_inside_quoted_fields()
    {
        var sink = new RecordingSink();
        CsvImporter.Parse("\"line1\nline2\",x", sink);

        Assert.Equal("line1\nline2", sink.Get("A1"));
        Assert.Equal("x", sink.Get("B1"));
    }

    [Fact]
    public void Detects_a_semicolon_delimiter()
    {
        var sink = new RecordingSink();
        CsvImporter.Parse("a;b;c\n1;2;3", sink);

        Assert.Equal("a", sink.Get("A1"));
        Assert.Equal("c", sink.Get("C1"));
        Assert.Equal("3", sink.Get("C2"));
    }

    [Fact]
    public void Strips_a_byte_order_mark()
    {
        var sink = new RecordingSink();
        CsvImporter.Parse("\uFEFFa,b", sink);

        Assert.Equal("a", sink.Get("A1"));
    }

    [Fact]
    public void A_trailing_newline_does_not_add_a_row()
    {
        var sink = new RecordingSink();
        var result = CsvImporter.Parse("a,b\n", sink);

        Assert.Equal(1, result.Rows);
    }

    [Fact]
    public async Task Imports_into_a_model_and_evaluates_formulas()
    {
        var model = Sheet.CreateModel();
        const string csv = "Item,Amount\nApples,10\nBananas,20\nTotal,=SUM(B2:B3)\n";

        await using var stream = new MemoryStream(Encoding.UTF8.GetBytes(csv));

        var result = await new CsvImporter().ImportAsync(stream, model);

        Assert.Equal("Apples", Sheet.Display(model, "A2"));
        Assert.Equal("30", Sheet.Display(model, "B4"));
        Assert.Null(result.SheetName);
    }

    [Fact]
    public void Recognises_its_extensions()
    {
        var importer = new CsvImporter();

        Assert.True(importer.CanImport("data.csv"));
        Assert.True(importer.CanImport("DATA.CSV"));
        Assert.False(importer.CanImport("book.xlsx"));
    }

    [Fact]
    public void Detects_a_tab_delimiter()
    {
        var sink = new RecordingSink();
        var result = CsvImporter.Parse("a\tb\n1\t2", sink);

        Assert.Equal("b", sink.Get("B1"));
        Assert.Equal("2", sink.Get("B2"));
        Assert.Equal(2, result.Columns);
    }

    [Fact]
    public void Detects_a_pipe_delimiter()
    {
        var sink = new RecordingSink();
        CsvImporter.Parse("a|b|c", sink);

        Assert.Equal("c", sink.Get("C1"));
    }

    [Fact]
    public void Handles_crlf_and_lone_cr_line_endings()
    {
        var crlf = new RecordingSink();
        CsvImporter.Parse("a,b\r\nc,d\r\n", crlf);
        Assert.Equal("c", crlf.Get("A2"));
        Assert.Equal("d", crlf.Get("B2"));

        var cr = new RecordingSink();
        CsvImporter.Parse("a,b\rc,d", cr);
        Assert.Equal("c", cr.Get("A2"));
    }

    [Fact]
    public void Empty_input_produces_no_rows()
    {
        var sink = new RecordingSink();
        var result = CsvImporter.Parse(string.Empty, sink);

        Assert.Equal(0, result.Rows);
        Assert.Equal(0, result.Columns);
        Assert.Empty(sink.Cells);
    }

    [Fact]
    public void Blank_lines_do_not_create_cells()
    {
        var sink = new RecordingSink();
        var result = CsvImporter.Parse("a,b\n\nc,d", sink);

        Assert.Equal(3, result.Rows);
        Assert.Equal("c", sink.Get("A3")); // blank line consumed row 2
    }

    [Fact]
    public void An_empty_quoted_field_is_empty()
    {
        var sink = new RecordingSink();
        CsvImporter.Parse("x,\"\",y", sink);

        Assert.Equal(string.Empty, sink.Get("B1"));
        Assert.Equal("y", sink.Get("C1"));
    }

    [Fact]
    public void A_quoted_field_can_contain_the_delimiter_and_a_newline()
    {
        var sink = new RecordingSink();
        CsvImporter.Parse("\"a,b\nc\",d", sink);

        Assert.Equal("a,b\nc", sink.Get("A1"));
        Assert.Equal("d", sink.Get("B1"));
    }

    [Fact]
    public void Counts_non_empty_cells()
    {
        var sink = new RecordingSink();
        var result = CsvImporter.Parse("a,,c\n,2,", sink);

        Assert.Equal(3, result.CellsImported);
    }

    [Fact]
    public async Task Imports_a_tsv_file_end_to_end()
    {
        var model = Sheet.CreateModel();
        const string tsv = "Item\tAmount\nApples\t10\nTotal\t=SUM(B2:B2)\n";

        await using var stream = new MemoryStream(Encoding.UTF8.GetBytes(tsv));

        await new CsvImporter().ImportAsync(stream, model);

        Assert.Equal("Apples", Sheet.Display(model, "A2"));
        Assert.Equal("10", Sheet.Display(model, "B3"));
    }
}
