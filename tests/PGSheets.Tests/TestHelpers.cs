using PGSheets.Core;
using PGSheets.Formulas;

namespace PGSheets.Tests;

/// <summary>Import sink that records everything it is given, for asserting on importer output.</summary>
internal sealed class RecordingSink : IImportSink
{
    public Dictionary<(int Row, int Column), string> Cells { get; } = new();

    public void SetRawInput(int row, int column, string rawInput) => Cells[(row, column)] = rawInput;

    public string Get(string a1)
    {
        var (row, column) = Sheet.At(a1);
        return Cells.TryGetValue((row, column), out var value) ? value : string.Empty;
    }
}

/// <summary>Small helpers shared by the test classes.</summary>
internal static class Sheet
{
    public static (int Row, int Column) At(string a1)
    {
        Assert.True(CellAddress.TryParseA1(a1, out var row, out var column), $"Unparseable address '{a1}'.");
        return (row, column);
    }

    public static SpreadsheetModel CreateModel() => new(new FormulaEngine());

    public static void Set(SpreadsheetModel model, string a1, string raw)
    {
        var (row, column) = At(a1);
        model.SetRawInput(row, column, raw);
    }

    public static string Display(SpreadsheetModel model, string a1)
    {
        var (row, column) = At(a1);
        return model.GetDisplayValue(row, column);
    }
}
