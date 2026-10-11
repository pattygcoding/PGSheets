using System.Globalization;
using System.Xml;
using PGSheets.Core;
using PGSheets.Ooxml;

namespace PGSheets.Export;

public sealed partial class XlsxWriter
{
    /// <summary>
    /// Streams the worksheet. Only populated cells are visited, grouped into one <c>&lt;row&gt;</c>
    /// each, so the part is written in a single pass without building a document in memory.
    /// </summary>
    private static void WriteWorksheet(
        XmlWriter writer,
        IExportSource source,
        ExportOptions options,
        NumberFormatTable numberFormats,
        ExportResult result,
        CancellationToken cancellationToken)
    {
        var cells = new List<(int Row, int Column)>(source.PopulatedCells);

        if (options.Range is { } range)
        {
            cells.RemoveAll(cell => !range.Contains(cell.Row, cell.Column));
        }

        cells.Sort(static (left, right) => left.Row != right.Row
            ? left.Row.CompareTo(right.Row)
            : left.Column.CompareTo(right.Column));

        var maxRow = -1;
        var maxColumn = -1;
        var written = 0;

        writer.WriteStartDocument();
        writer.WriteStartElement("worksheet", SpreadsheetMl.MainNamespace);
        writer.WriteStartElement("sheetData", SpreadsheetMl.MainNamespace);

        var index = 0;

        while (index < cells.Count)
        {
            cancellationToken.ThrowIfCancellationRequested();

            var row = cells[index].Row;

            writer.WriteStartElement("row", SpreadsheetMl.MainNamespace);
            writer.WriteAttributeString("r", (row + 1).ToString(CultureInfo.InvariantCulture));

            while (index < cells.Count && cells[index].Row == row)
            {
                cancellationToken.ThrowIfCancellationRequested();

                var column = cells[index].Column;

                WriteCell(
                    writer,
                    row,
                    column,
                    source,
                    numberFormats.IndexOf(source.GetNumberFormat(row, column)));

                written++;

                if (column > maxColumn)
                {
                    maxColumn = column;
                }

                index++;
            }

            writer.WriteEndElement();

            maxRow = row;
        }

        writer.WriteEndElement();
        writer.WriteEndElement();
        writer.WriteEndDocument();

        result.Rows = maxRow + 1;
        result.Columns = maxColumn + 1;
        result.CellsWritten = written;
    }

    private static void WriteCell(XmlWriter writer, int row, int column, IExportSource source, int styleIndex)
    {
        var raw = source.GetRawInput(row, column);
        var reference = CellAddress.ToA1(row, column);

        // A leading '=' is what makes a cell a formula; writing it as text would turn every
        // formula into a string, so it is written as <f> without the sign.
        if (raw.Length > 0 && raw[0] == '=')
        {
            WriteFormulaCell(writer, reference, raw[1..], source.GetValue(row, column), styleIndex);
            return;
        }

        var value = source.GetValue(row, column);

        if (value.Kind == CellValueKind.Empty)
        {
            // A populated cell always evaluates to something; fall back to the literal rules.
            value = Literals.Parse(raw);
        }

        switch (value.Kind)
        {
            case CellValueKind.Number:
                // "R" round-trips the double exactly, so a save/load cycle does not drift.
                WriteValueCell(writer, reference, type: null, value.Number.ToString("R", CultureInfo.InvariantCulture), styleIndex);
                break;

            case CellValueKind.Boolean:
                WriteValueCell(writer, reference, "b", value.Boolean ? "1" : "0", styleIndex);
                break;

            case CellValueKind.Error:
                WriteValueCell(writer, reference, "e", CellValue.ErrorText(value.Error), styleIndex);
                break;

            default:
                WriteInlineStringCell(writer, reference, raw, styleIndex);
                break;
        }
    }

    private static void WriteFormulaCell(
        XmlWriter writer,
        string reference,
        string formula,
        CellValue value,
        int styleIndex)
    {
        writer.WriteStartElement("c", SpreadsheetMl.MainNamespace);
        writer.WriteAttributeString("r", reference);
        WriteStyle(writer, styleIndex);

        switch (value.Kind)
        {
            // Without the type flag a string result would be read back as a number.
            case CellValueKind.Text:
                writer.WriteAttributeString("t", "str");
                break;

            case CellValueKind.Boolean:
                writer.WriteAttributeString("t", "b");
                break;
        }

        writer.WriteStartElement("f", SpreadsheetMl.MainNamespace);
        writer.WriteString(SpreadsheetMl.SanitizeText(formula));
        writer.WriteEndElement();

        // Errors are not cached: the reading application recalculates rather than inheriting this
        // engine's #NAME? results for functions it does not implement.
        if (!value.IsError)
        {
            WriteValueElement(writer, ValueText(value));
        }

        writer.WriteEndElement();
    }

    private static void WriteValueCell(
        XmlWriter writer,
        string reference,
        string? type,
        string text,
        int styleIndex)
    {
        writer.WriteStartElement("c", SpreadsheetMl.MainNamespace);
        writer.WriteAttributeString("r", reference);

        if (type is not null)
        {
            writer.WriteAttributeString("t", type);
        }

        WriteStyle(writer, styleIndex);
        WriteValueElement(writer, text);
        writer.WriteEndElement();
    }

    private static void WriteInlineStringCell(XmlWriter writer, string reference, string text, int styleIndex)
    {
        writer.WriteStartElement("c", SpreadsheetMl.MainNamespace);
        writer.WriteAttributeString("r", reference);
        writer.WriteAttributeString("t", "inlineStr");
        WriteStyle(writer, styleIndex);

        writer.WriteStartElement("is", SpreadsheetMl.MainNamespace);
        writer.WriteStartElement("t", SpreadsheetMl.MainNamespace);

        // Without this, leading and trailing spaces are silently dropped, and text that is
        // entirely whitespace is discarded altogether.
        if (text.Length != text.Trim().Length)
        {
            writer.WriteAttributeString("xml", "space", "http://www.w3.org/XML/1998/namespace", "preserve");
        }

        writer.WriteString(SpreadsheetMl.SanitizeText(text));
        writer.WriteEndElement();
        writer.WriteEndElement();
        writer.WriteEndElement();
    }

    private static void WriteValueElement(XmlWriter writer, string text)
    {
        writer.WriteStartElement("v", SpreadsheetMl.MainNamespace);
        writer.WriteString(SpreadsheetMl.SanitizeText(text));
        writer.WriteEndElement();
    }

    /// <summary>
    /// Writes the style index that ties a cell to its number format. Index 0 is the default entry, so
    /// leaving the attribute off says the same thing while keeping the part smaller.
    /// </summary>
    private static void WriteStyle(XmlWriter writer, int styleIndex)
    {
        if (styleIndex > 0)
        {
            writer.WriteAttributeString("s", styleIndex.ToString(CultureInfo.InvariantCulture));
        }
    }

    private static string ValueText(CellValue value) => value.Kind switch
    {
        CellValueKind.Number => value.Number.ToString("R", CultureInfo.InvariantCulture),
        CellValueKind.Boolean => value.Boolean ? "1" : "0",
        _ => value.ToDisplayString(),
    };
}
