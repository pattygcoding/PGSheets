using PGSheets.Core;
using PGSheets.Formulas;

namespace PGSheets.Tests;

/// <summary>Import sink that records everything it is given, for asserting on importer output.</summary>
internal sealed class RecordingSink : IImportSink
{
    public Dictionary<(int Row, int Column), string> Cells { get; } = new();

    public Dictionary<(int Row, int Column), string?> Formats { get; } = new();

    public void SetRawInput(int row, int column, string rawInput) => Cells[(row, column)] = rawInput;

    public void SetNumberFormat(int row, int column, string? numberFormat) =>
        Formats[(row, column)] = numberFormat;

    public string Get(string a1)
    {
        var (row, column) = Sheet.At(a1);
        return Cells.TryGetValue((row, column), out var value) ? value : string.Empty;
    }

    /// <summary>The number format that came with a cell, or null when it had none.</summary>
    public string? GetFormat(string a1)
    {
        var (row, column) = Sheet.At(a1);
        return Formats.TryGetValue((row, column), out var value) ? value : null;
    }
}

/// <summary>
/// A frozen snapshot of the function library as this project ships it.
/// </summary>
/// <remarks>
/// The registry is global and <c>BuiltinFunctions.Register</c> is a supported extension point, so a
/// host — and, here, another test that registers its own function — adds to the same dictionary.
/// Capturing the names in a module initializer fixes the built-in set before any test body runs, so
/// a sweep over "every function" tests the shipped library rather than whatever else has arrived.
/// </remarks>
internal static class BuiltInLibrary
{
    private static string[]? _names;

    /// <summary>The names that were registered before any test started.</summary>
    public static string[] Names => _names ??= Formulas.BuiltinFunctions.Names.ToArray();

    [System.Runtime.CompilerServices.ModuleInitializer]
    internal static void Capture() => _ = Names;
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

    /// <summary>
    /// Evaluates a formula in Z1 of an optionally seeded model and returns what the grid would
    /// show. Z1 is used because no test data lands there, so a formula can never observe itself.
    /// </summary>
    public static string Evaluate(string formula, Action<SpreadsheetModel>? seed = null)
    {
        var model = CreateModel();
        seed?.Invoke(model);
        Set(model, "Z1", formula);

        return Display(model, "Z1");
    }
}
