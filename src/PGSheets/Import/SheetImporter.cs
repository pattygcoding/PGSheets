using PGSheets.Core;

namespace PGSheets.Import;

/// <summary>
/// Chooses an <see cref="ISpreadsheetImporter"/> for a file name. Register additional formats here
/// (a single line) to grow the import story without touching the UI.
/// </summary>
public static class SheetImporter
{
    private static readonly ISpreadsheetImporter[] Importers =
    {
        new XlsxImporter(),
        new CsvImporter(),
    };

    /// <summary>Returns the importer that claims <paramref name="fileName"/>, or null.</summary>
    public static ISpreadsheetImporter? Resolve(string fileName) =>
        Importers.FirstOrDefault(importer => importer.CanImport(fileName));

    public static bool CanImport(string fileName) => Resolve(fileName) is not null;

    public static async Task<ImportResult> ImportAsync(
        Stream stream,
        string fileName,
        IImportSink sink,
        CancellationToken cancellationToken = default)
    {
        var importer = Resolve(fileName)
            ?? throw new NotSupportedException($"No importer is registered for '{fileName}'.");

        return await importer.ImportAsync(stream, sink, cancellationToken).ConfigureAwait(false);
    }

    /// <summary>File-picker patterns for every registered importer, plus a combined entry.</summary>
    public static IReadOnlyList<string> AllPatterns =>
        Importers.SelectMany(i => i.Extensions).Distinct(StringComparer.OrdinalIgnoreCase).Select(e => "*" + e).ToArray();
}
