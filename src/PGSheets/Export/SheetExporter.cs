using PGSheets.Core;

namespace PGSheets.Export;

/// <summary>
/// Chooses an <see cref="ISpreadsheetExporter"/> for a file name. Register additional formats here
/// (a single line) to grow the save story without touching the UI.
/// </summary>
public static class SheetExporter
{
    private static readonly ISpreadsheetExporter[] Writers =
    {
        new XlsxWriter(),
        new CsvExporter(),
    };

    /// <summary>Format used when the user has not chosen one yet.</summary>
    public const string DefaultExtension = ".xlsx";

    /// <summary>Returns the exporter that claims <paramref name="fileName"/>, or null.</summary>
    public static ISpreadsheetExporter? Resolve(string fileName) =>
        Writers.FirstOrDefault(writer => writer.CanExport(fileName));

    public static bool CanExport(string fileName) => Resolve(fileName) is not null;

    public static async Task<ExportResult> ExportAsync(
        Stream stream,
        string fileName,
        IExportSource source,
        ExportOptions? options = null,
        CancellationToken cancellationToken = default)
    {
        var writer = Resolve(fileName)
            ?? throw new NotSupportedException($"No exporter is registered for '{fileName}'.");

        return await writer
            .ExportAsync(stream, fileName, source, options ?? new ExportOptions(), cancellationToken)
            .ConfigureAwait(false);
    }

    /// <summary>File-picker patterns for every registered exporter.</summary>
    public static IReadOnlyList<string> AllPatterns =>
        Writers.SelectMany(writer => writer.Extensions)
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .Select(extension => "*" + extension)
            .ToArray();

    /// <summary>
    /// Whether a document loaded from a file may be written back over it.
    /// </summary>
    /// <remarks>
    /// Saving writes a single worksheet and truncates the target, so a workbook that held more than
    /// one worksheet has to go through Save As instead: writing in place would destroy every sheet
    /// that was never loaded. This is the same rule that already sends an <c>.xlsm</c> save to Save
    /// As, because a VBA project cannot be produced — a format this app cannot fully write is never
    /// overwritten in place.
    /// </remarks>
    public static bool CanSaveInPlace(int loadedWorksheetCount) => loadedWorksheetCount <= 1;

    /// <summary>
    /// What a single-worksheet save leaves behind, or null when nothing is lost. The caller surfaces
    /// this as an export warning, so the loss is stated rather than discovered later.
    /// </summary>
    public static string? DescribeLostWorksheets(int loadedWorksheetCount, string writtenSheet) =>
        loadedWorksheetCount <= 1
            ? null
            : $"only the '{writtenSheet}' worksheet was written; the other " +
              $"{loadedWorksheetCount - 1} worksheet(s) in the file that was opened are not preserved.";
}
