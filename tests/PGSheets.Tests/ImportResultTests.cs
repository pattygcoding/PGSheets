using PGSheets.Import;

namespace PGSheets.Tests;

public class ImportResultTests
{
    [Fact]
    public void Warnings_are_capped()
    {
        var result = new ImportResult();

        for (var i = 0; i < 150; i++)
        {
            result.AddWarning($"warning {i}");
        }

        Assert.Equal(101, result.Warnings.Count);
        Assert.Equal("Further warnings suppressed.", result.Warnings[^1]);
    }

    [Fact]
    public void A_handful_of_warnings_are_all_kept()
    {
        var result = new ImportResult();

        result.AddWarning("first");
        result.AddWarning("second");

        Assert.Equal(new[] { "first", "second" }, result.Warnings);
    }

    [Fact]
    public void Defaults_are_empty()
    {
        var result = new ImportResult();

        Assert.Equal(0, result.Rows);
        Assert.Equal(0, result.Columns);
        Assert.Equal(0, result.CellsImported);
        Assert.Null(result.SheetName);
        Assert.Empty(result.Warnings);
    }
}
