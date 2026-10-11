using PGSheets.Core;

namespace PGSheets.Export;

/// <summary>
/// A file format writer. Implementations read from an <see cref="IExportSource"/> and never touch
/// the UI, which keeps them unit-testable without an Avalonia window.
/// </summary>
/// <remarks>
/// Unlike <see cref="Import.ISpreadsheetImporter"/> the file name is passed in, because a writer
/// cannot sniff the format it has been asked to produce: ".csv" and ".tsv" share one
/// implementation and differ only by the extension the user chose.
/// </remarks>
public interface ISpreadsheetExporter
{
    /// <summary>Lower-case extensions this exporter claims, e.g. ".csv".</summary>
    IReadOnlyList<string> Extensions { get; }

    bool CanExport(string fileName);

    Task<ExportResult> ExportAsync(
        Stream stream,
        string fileName,
        IExportSource source,
        ExportOptions options,
        CancellationToken cancellationToken = default);
}
