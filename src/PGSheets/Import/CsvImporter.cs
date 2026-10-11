using System.Text;
using PGSheets.Core;

namespace PGSheets.Import;

/// <summary>
/// Reads delimited text (RFC 4180). Handles quoted fields, escaped quotes ("") and embedded
/// newlines, strips a UTF-8 BOM and auto-detects the delimiter from the first line.
/// Implemented on the BCL so no CSV package is required.
/// </summary>
public sealed class CsvImporter : ISpreadsheetImporter
{
    private static readonly char[] DelimiterCandidates = { ',', ';', '\t', '|' };

    public IReadOnlyList<string> Extensions { get; } = new[] { ".csv", ".tsv", ".txt" };

    public bool CanImport(string fileName)
    {
        var extension = Path.GetExtension(fileName);
        return Extensions.Any(e => string.Equals(e, extension, StringComparison.OrdinalIgnoreCase));
    }

    public async Task<ImportResult> ImportAsync(Stream stream, IImportSink sink, CancellationToken cancellationToken = default)
    {
        using var reader = new StreamReader(
            stream,
            Encoding.UTF8,
            detectEncodingFromByteOrderMarks: true,
            bufferSize: 16 * 1024,
            leaveOpen: true);

        var text = await reader.ReadToEndAsync(cancellationToken).ConfigureAwait(false);
        return Parse(text, sink);
    }

    /// <summary>Parses delimited text. Exposed for direct testing.</summary>
    public static ImportResult Parse(string text, IImportSink sink)
    {
        var result = new ImportResult();

        if (string.IsNullOrEmpty(text))
        {
            return result;
        }

        if (text[0] == '\uFEFF')
        {
            text = text[1..];
        }

        var delimiter = DetectDelimiter(text);

        var row = 0;
        var column = 0;
        var maxColumn = 0;
        var rows = 0;
        var cells = 0;
        var index = 0;
        var fieldStarted = false;
        var inQuotes = false;
        var field = new StringBuilder();

        while (index < text.Length)
        {
            var ch = text[index];

            if (inQuotes)
            {
                if (ch == '"')
                {
                    if (index + 1 < text.Length && text[index + 1] == '"')
                    {
                        field.Append('"');
                        index += 2;
                        continue;
                    }

                    inQuotes = false;
                    index++;
                    continue;
                }

                field.Append(ch);
                index++;
                continue;
            }

            if (ch == '"' && !fieldStarted)
            {
                inQuotes = true;
                fieldStarted = true;
                index++;
                continue;
            }

            if (ch == delimiter)
            {
                EmitField(field, row, ref column, ref cells, sink);
                fieldStarted = false;
                index++;
                continue;
            }

            if (ch is '\r' or '\n')
            {
                EmitField(field, row, ref column, ref cells, sink);
                fieldStarted = false;

                maxColumn = Math.Max(maxColumn, column);
                column = 0;
                rows++;

                if (ch == '\r' && index + 1 < text.Length && text[index + 1] == '\n')
                {
                    index++;
                }

                index++;
                row++;
                continue;
            }

            field.Append(ch);
            fieldStarted = true;
            index++;
        }

        // Flush a trailing record that was not terminated by a newline.
        if (field.Length > 0 || column > 0 || inQuotes)
        {
            EmitField(field, row, ref column, ref cells, sink);
            maxColumn = Math.Max(maxColumn, column);
            rows++;
        }

        result.Rows = rows;
        result.Columns = maxColumn;
        result.CellsImported = cells;
        return result;
    }

    private static void EmitField(
        StringBuilder field,
        int row,
        ref int column,
        ref int cells,
        IImportSink sink)
    {
        var value = field.ToString();
        field.Clear();

        if (row < SheetBounds.MaxRows && column < SheetBounds.MaxColumns)
        {
            sink.SetRawInput(row, column, value);

            if (value.Length > 0)
            {
                cells++;
            }
        }

        column++;
    }

    private static char DetectDelimiter(string text)
    {
        var end = text.IndexOfAny(new[] { '\r', '\n' });
        if (end < 0)
        {
            end = text.Length;
        }

        var best = ',';
        var bestCount = 0;

        foreach (var candidate in DelimiterCandidates)
        {
            var count = 0;

            for (var i = 0; i < end; i++)
            {
                if (text[i] == candidate)
                {
                    count++;
                }
            }

            if (count > bestCount)
            {
                bestCount = count;
                best = candidate;
            }
        }

        return best;
    }
}
