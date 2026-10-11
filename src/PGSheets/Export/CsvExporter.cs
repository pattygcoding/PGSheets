using System.Globalization;
using System.Text;
using PGSheets.Core;

namespace PGSheets.Export;

/// <summary>
/// Writes delimited text (RFC 4180). Quoting, escaping and line endings are chosen to round-trip
/// back through <see cref="Import.CsvImporter"/>. Implemented on the BCL so no CSV package is
/// required.
/// </summary>
public sealed class CsvExporter : ISpreadsheetExporter
{
    private const char Comma = ',';
    private const char Tab = '\t';

    /// <summary>Line ending used between records. CRLF is what spreadsheet applications expect.</summary>
    private const string RecordSeparator = "\r\n";

    public IReadOnlyList<string> Extensions { get; } = new[] { ".csv", ".tsv", ".txt" };

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
        ExportAsync(stream, fileName, source, options, DelimiterFor(fileName), cancellationToken);

    /// <summary>Writes using an explicit delimiter; used by tests and callers overriding the format.</summary>
    public async Task<ExportResult> ExportAsync(
        Stream stream,
        string fileName,
        IExportSource source,
        ExportOptions options,
        char delimiter,
        CancellationToken cancellationToken = default)
    {
        var result = new ExportResult();
        var text = Build(source, options, delimiter, result);

        if (text.Length == 0)
        {
            return result;
        }

        // A byte-order mark keeps non-ASCII text readable in Excel, which is not UTF-8 native.
        // CsvImporter strips it again on the way back in. GetBytes() does not emit the preamble,
        // so it has to be written separately.
        var encoding = new UTF8Encoding(encoderShouldEmitUTF8Identifier: true);

        await stream.WriteAsync(encoding.GetPreamble(), cancellationToken).ConfigureAwait(false);
        await stream.WriteAsync(encoding.GetBytes(text), cancellationToken).ConfigureAwait(false);

        return result;
    }

    /// <summary>Tab for ".tsv", comma for everything else.</summary>
    private static char DelimiterFor(string fileName) =>
        string.Equals(Path.GetExtension(fileName), ".tsv", StringComparison.OrdinalIgnoreCase) ? Tab : Comma;

    /// <summary>Renders a region as delimited text. Exposed for direct testing and for the clipboard.</summary>
    public static string Build(IExportSource source, ExportOptions options, char delimiter, ExportResult result)
    {
        var range = options.Range;

        int firstRow;
        int firstColumn;
        int lastRow;
        int lastColumn;
        var cells = 0;

        if (range is { } selected)
        {
            // A range keeps its rectangular shape even where cells are empty, so a copied or saved
            // block lines up with what was selected.
            firstRow = selected.Row1;
            firstColumn = selected.Column1;
            lastRow = selected.Row2;
            lastColumn = selected.Column2;

            foreach (var (row, column) in source.PopulatedCells)
            {
                if (selected.Contains(row, column) && source.GetRawInput(row, column).Length > 0)
                {
                    cells++;
                }
            }
        }
        else
        {
            firstRow = 0;
            firstColumn = 0;
            lastRow = -1;
            lastColumn = -1;

            foreach (var (row, column) in source.PopulatedCells)
            {
                if (row > lastRow)
                {
                    lastRow = row;
                }

                if (column > lastColumn)
                {
                    lastColumn = column;
                }

                if (source.GetRawInput(row, column).Length > 0)
                {
                    cells++;
                }
            }
        }

        result.Rows = lastRow - firstRow + 1;
        result.Columns = lastColumn - firstColumn + 1;
        result.CellsWritten = cells;

        if (lastRow < firstRow || lastColumn < firstColumn)
        {
            return string.Empty;
        }

        if (options.ExportValues)
        {
            ReportLossyValues(source, result);
        }

        var builder = new StringBuilder();

        for (var row = firstRow; row <= lastRow; row++)
        {
            for (var column = firstColumn; column <= lastColumn; column++)
            {
                if (column > firstColumn)
                {
                    builder.Append(delimiter);
                }

                AppendField(builder, GetCellText(source, row, column, options), delimiter);
            }

            builder.Append(RecordSeparator);
        }

        return builder.ToString();
    }

    private static string GetCellText(IExportSource source, int row, int column, ExportOptions options)
    {
        var raw = source.GetRawInput(row, column);

        if (!options.ExportValues || raw.Length == 0)
        {
            return raw;
        }

        return source.GetValue(row, column).ToDisplayString();
    }

    /// <summary>
    /// Quotes a field when it contains the delimiter, a quote or a line break, and also when it has
    /// leading or trailing whitespace that an unquoted round trip would silently trim.
    /// </summary>
    private static void AppendField(StringBuilder builder, string text, char delimiter)
    {
        var needsQuotes =
            text.IndexOf(delimiter) >= 0 ||
            text.IndexOf('"') >= 0 ||
            text.IndexOf('\n') >= 0 ||
            text.IndexOf('\r') >= 0 ||
            (text.Length > 0 && (char.IsWhiteSpace(text[0]) || char.IsWhiteSpace(text[^1])));

        if (!needsQuotes)
        {
            builder.Append(text);
            return;
        }

        builder.Append('"');

        foreach (var ch in text)
        {
            if (ch == '"')
            {
                builder.Append('"');
            }

            builder.Append(ch);
        }

        builder.Append('"');
    }

    /// <summary>
    /// In value mode a text result that looks numeric (or starts with '=') is read back as a number
    /// or as a formula. Reporting that is the honest option; silently changing the data is not.
    /// </summary>
    private static void ReportLossyValues(IExportSource source, ExportResult result)
    {
        var numericLooking = 0;
        var formulaLooking = 0;

        foreach (var (row, column) in source.PopulatedCells)
        {
            var value = source.GetValue(row, column);

            if (value.Kind != CellValueKind.Text)
            {
                continue;
            }

            var text = value.Text ?? string.Empty;

            if (text.Length == 0)
            {
                continue;
            }

            if (text[0] == '=')
            {
                formulaLooking++;
            }
            else if (LooksNumeric(text))
            {
                numericLooking++;
            }
        }

        if (numericLooking > 0)
        {
            result.AddWarning($"{numericLooking} text cell(s) look like numbers and will be read back as numbers.");
        }

        if (formulaLooking > 0)
        {
            result.AddWarning($"{formulaLooking} text cell(s) start with '=' and will be read back as formulas.");
        }
    }

    private static bool LooksNumeric(string text) =>
        double.TryParse(
            text,
            NumberStyles.Float | NumberStyles.AllowThousands,
            CultureInfo.InvariantCulture,
            out _);
}
