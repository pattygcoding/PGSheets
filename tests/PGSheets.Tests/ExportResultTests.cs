using PGSheets.Export;

namespace PGSheets.Tests;

public class ExportResultTests
{
    [Fact]
    public void Defaults_are_empty()
    {
        var result = new ExportResult();

        Assert.Equal(0, result.Rows);
        Assert.Equal(0, result.Columns);
        Assert.Equal(0, result.CellsWritten);
        Assert.Empty(result.Warnings);
    }

    [Fact]
    public void Warnings_are_capped_the_same_way_import_warnings_are()
    {
        var result = new ExportResult();

        for (var i = 0; i < 150; i++)
        {
            result.AddWarning($"warning {i}");
        }

        Assert.Equal(101, result.Warnings.Count);
        Assert.Equal("Further warnings suppressed.", result.Warnings[^1]);
    }
}
