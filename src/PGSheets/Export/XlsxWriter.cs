using System.IO.Compression;
using System.Xml;
using PGSheets.Core;
using PGSheets.Ooxml;

namespace PGSheets.Export;

/// <summary>
/// Minimal, dependency-free writer for the SpreadsheetML (xlsx) format. An xlsx file is a zip
/// archive of XML parts; this writer emits the six parts a workbook needs and nothing else.
/// </summary>
/// <remarks>
/// Cells use inline strings, so no shared string table is required. Formulas are written as
/// <c>&lt;f&gt;</c> elements, and results that are errors are deliberately left uncached so the
/// reading application recalculates them instead of inheriting this engine's limitations.
/// </remarks>
public sealed partial class XlsxWriter : ISpreadsheetExporter
{
    private const string ContentTypesPart = "[Content_Types].xml";
    private const string RootRelationshipsPart = "_rels/.rels";
    private const string WorkbookPart = "xl/workbook.xml";
    private const string WorkbookRelationshipsPart = "xl/_rels/workbook.xml.rels";
    private const string StylesPart = "xl/styles.xml";
    private const string WorksheetPart = "xl/worksheets/sheet1.xml";

    private const string OfficeDocumentRelationship =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";

    private const string WorksheetRelationship =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";

    private const string StylesRelationship =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";

    /// <summary>
    /// The first number format id the specification leaves to a workbook. Everything below it is
    /// fixed by the specification, so custom codes are numbered from here up.
    /// </summary>
    private const int FirstCustomFormatId = 164;

    /// <summary>
    /// The distinct number formats used by the cells being written, in the order they are first seen.
    /// Style index 0 is always General, which is what a cell with no format refers to.
    /// </summary>
    private sealed class NumberFormatTable
    {
        private readonly Dictionary<string, int> _indices = new(StringComparer.Ordinal);
        private readonly List<string> _codes = new();

        public IReadOnlyList<string> Codes => _codes;

        /// <summary>The style index for a format code; 0, the default entry, when there is none.</summary>
        public int IndexOf(string? code)
        {
            if (string.IsNullOrEmpty(code))
            {
                return 0;
            }

            if (_indices.TryGetValue(code, out var index))
            {
                return index;
            }

            index = _codes.Count + 1;
            _codes.Add(code);
            _indices[code] = index;

            return index;
        }

        /// <summary>The numFmtId written for the format at a style index.</summary>
        public int NumberFormatIdAt(int styleIndex) => FirstCustomFormatId + styleIndex - 1;
    }

    public IReadOnlyList<string> Extensions { get; } = new[] { ".xlsx" };

    public bool CanExport(string fileName)
    {
        var extension = Path.GetExtension(fileName);
        return Extensions.Any(e => string.Equals(e, extension, StringComparison.OrdinalIgnoreCase));
    }

    public Task<ExportResult> ExportAsync(
        Stream stream,
        string fileName,
        IExportSource source,
        ExportOptions options,
        CancellationToken cancellationToken = default) =>
        // Same reasoning as the importer: a large sheet must not block the UI thread.
        Task.Run(() => Write(stream, source, options, cancellationToken), cancellationToken);

    private static ExportResult Write(
        Stream stream,
        IExportSource source,
        ExportOptions options,
        CancellationToken cancellationToken)
    {
        var result = new ExportResult();
        var sheetName = SpreadsheetMl.SanitizeSheetName(options.SheetName);

        // The style table has to know every format before either part is written, so the formats are
        // collected in one pass over the cell keys first.
        var numberFormats = CollectNumberFormats(source, options);

        using (var archive = new ZipArchive(stream, ZipArchiveMode.Create, leaveOpen: true))
        {
            // [Content_Types].xml is written first; the format does not require it, but some
            // readers expect it at the front of the archive.
            WritePart(archive, ContentTypesPart, WriteContentTypes);
            WritePart(archive, RootRelationshipsPart, WriteRootRelationships);
            WritePart(archive, WorkbookPart, writer => WriteWorkbook(writer, sheetName));
            WritePart(archive, WorkbookRelationshipsPart, WriteWorkbookRelationships);
            WritePart(archive, StylesPart, writer => WriteStyles(writer, numberFormats));
            WritePart(
                archive,
                WorksheetPart,
                writer => WriteWorksheet(writer, source, options, numberFormats, result, cancellationToken));
        }

        if (!string.Equals(options.SheetName, sheetName, StringComparison.Ordinal))
        {
            result.AddWarning($"Worksheet name '{options.SheetName}' is not valid in xlsx; written as '{sheetName}'.");
        }

        return result;
    }

    /// <summary>Creates one zip entry and hands an <see cref="XmlWriter"/> to the part's writer.</summary>
    private static void WritePart(ZipArchive archive, string path, Action<XmlWriter> write)
    {
        var entry = archive.CreateEntry(path, CompressionLevel.Optimal);

        using var stream = entry.Open();
        using var writer = XmlWriter.Create(stream, SpreadsheetMl.WriterSettings);

        write(writer);
        writer.Flush();
    }

    /// <summary>
    /// Collects the number formats the cells being written use, so the style table can name them and
    /// the worksheet can refer to them by index.
    /// </summary>
    private static NumberFormatTable CollectNumberFormats(IExportSource source, ExportOptions options)
    {
        var table = new NumberFormatTable();

        foreach (var (row, column) in source.PopulatedCells)
        {
            if (options.Range is { } range && !range.Contains(row, column))
            {
                continue;
            }

            table.IndexOf(source.GetNumberFormat(row, column));
        }

        return table;
    }
}
