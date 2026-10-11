using PGSheets.Core;

namespace PGSheets.Import;

/// <summary>
/// A file format reader. Implementations push raw input into an <see cref="IImportSink"/> and never
/// touch the UI, which keeps them unit-testable without an Avalonia window.
/// </summary>
public interface ISpreadsheetImporter
{
    /// <summary>Lower-case extensions this importer claims, e.g. ".csv".</summary>
    IReadOnlyList<string> Extensions { get; }

    bool CanImport(string fileName);

    Task<ImportResult> ImportAsync(Stream stream, IImportSink sink, CancellationToken cancellationToken = default);
}
