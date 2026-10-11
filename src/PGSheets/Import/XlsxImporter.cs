using System.IO.Compression;
using System.Xml;
using PGSheets.Core;

namespace PGSheets.Import;

/// <summary>
/// Minimal, dependency-free reader for the SpreadsheetML (xlsx/xlsm) format. An xlsx file is a zip
/// archive of XML parts; this reader pulls the first worksheet, the shared string table and just
/// enough of the style table to recognise date formats.
/// </summary>
/// <remarks>
/// Deliberate limitations: only the first worksheet is read, and date-formatted numbers are
/// imported as ISO text rather than as serial numbers.
/// </remarks>
public sealed partial class XlsxImporter : ISpreadsheetImporter
{
    public IReadOnlyList<string> Extensions { get; } = new[] { ".xlsx", ".xlsm" };

    /// <summary>
    /// When true (the default) formulas are imported as formulas and re-evaluated by
    /// <see cref="Formulas.FormulaEngine"/>. When false the cached result stored in the file is
    /// imported as a literal instead.
    /// </summary>
    public bool ImportFormulas { get; init; } = true;

    public bool CanImport(string fileName)
    {
        var extension = Path.GetExtension(fileName);
        return Extensions.Any(e => string.Equals(e, extension, StringComparison.OrdinalIgnoreCase));
    }

    public Task<ImportResult> ImportAsync(Stream stream, IImportSink sink, CancellationToken cancellationToken = default) =>
        Task.Run(() => Import(stream, sink, cancellationToken), cancellationToken);

    private ImportResult Import(Stream stream, IImportSink sink, CancellationToken cancellationToken)
    {
        var result = new ImportResult();

        using var archive = new ZipArchive(stream, ZipArchiveMode.Read, leaveOpen: true);

        var sheets = ReadWorkbookSheets(archive, result);
        result.WorksheetCount = sheets.Count;

        if (sheets.Count == 0)
        {
            result.AddWarning("The workbook does not contain any worksheets.");
            return result;
        }

        if (sheets.Count > 1)
        {
            result.AddWarning($"The workbook has {sheets.Count} worksheets; only '{sheets[0].Name}' was imported.");
        }

        var (sheetName, sheetPath) = sheets[0];
        result.SheetName = sheetName;

        var sharedStrings = ReadSharedStrings(archive);
        var numberFormats = ReadNumberFormats(archive);

        var entry = archive.GetEntry(sheetPath);
        if (entry is null)
        {
            result.AddWarning($"Worksheet part '{sheetPath}' is missing from the archive.");
            return result;
        }

        using var sheetStream = entry.Open();
        ParseWorksheet(sheetStream, sink, sharedStrings, numberFormats, result, ImportFormulas, cancellationToken);

        return result;
    }
}
