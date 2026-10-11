using System.Globalization;
using System.Xml;
using PGSheets.Core;
using PGSheets.Formulas;

namespace PGSheets.Import;

public sealed partial class XlsxImporter
{
    /// <summary>
    /// Streams a worksheet part. The reader walks <c>&lt;row&gt;</c> and <c>&lt;c&gt;</c> elements and
    /// pushes each cell into the sink; nothing larger than one cell is ever materialised.
    /// </summary>
    private static void ParseWorksheet(
        Stream stream,
        IImportSink sink,
        List<string> sharedStrings,
        Dictionary<int, bool> dateStyles,
        ImportResult result,
        bool importFormulas,
        CancellationToken cancellationToken)
    {
        using var reader = XmlReader.Create(stream, XmlSettings);

        var currentRow = -1;
        var nextColumn = 0;
        var maxRow = 0;
        var maxColumn = 0;
        var cells = 0;
        var unknownFunctions = new SortedSet<string>(StringComparer.OrdinalIgnoreCase);

        while (reader.Read())
        {
            if (reader.NodeType != XmlNodeType.Element)
            {
                continue;
            }

            if (reader.LocalName == "row")
            {
                var rowAttribute = reader.GetAttribute("r");

                currentRow = rowAttribute is not null &&
                             int.TryParse(rowAttribute, NumberStyles.Integer, CultureInfo.InvariantCulture, out var rowIndex)
                    ? rowIndex - 1
                    : currentRow + 1;

                nextColumn = 0;
                continue;
            }

            if (reader.LocalName != "c")
            {
                continue;
            }

            cancellationToken.ThrowIfCancellationRequested();

            var reference = reader.GetAttribute("r");
            var type = reader.GetAttribute("t");
            var styleAttribute = reader.GetAttribute("s");

            int row;
            int column;

            if (reference is not null && CellAddress.TryParseA1(reference, out var parsedRow, out var parsedColumn))
            {
                row = parsedRow;
                column = parsedColumn;
            }
            else
            {
                row = currentRow;
                column = nextColumn;
            }

            nextColumn = column + 1;

            string? value = null;
            string? formula = null;
            string? inlineText = null;

            if (!reader.IsEmptyElement)
            {
                using var subtree = reader.ReadSubtree();
                subtree.Read();

                while (subtree.Read())
                {
                    if (subtree.NodeType != XmlNodeType.Element)
                    {
                        continue;
                    }

                    switch (subtree.LocalName)
                    {
                        case "v":
                            value = ReadElementText(subtree);
                            break;
                        case "f":
                            formula = ReadElementText(subtree);
                            break;
                        case "t":
                            inlineText = ReadElementText(subtree);
                            break;
                    }
                }
            }

            var raw = ResolveCellText(type, value, formula, inlineText, styleAttribute, sharedStrings, dateStyles, importFormulas);

            if (importFormulas && formula is { Length: > 0 })
            {
                foreach (var function in ExtractFunctionNames(formula))
                {
                    if (!BuiltinFunctions.IsKnown(function))
                    {
                        unknownFunctions.Add(function);
                    }
                }
            }

            if (row >= 0 && column >= 0 && row < SheetBounds.MaxRows && column < SheetBounds.MaxColumns)
            {
                sink.SetRawInput(row, column, raw);

                if (raw.Length > 0)
                {
                    cells++;
                }

                if (row + 1 > maxRow)
                {
                    maxRow = row + 1;
                }

                if (column + 1 > maxColumn)
                {
                    maxColumn = column + 1;
                }
            }
        }

        result.Rows = maxRow;
        result.Columns = maxColumn;
        result.CellsImported = cells;

        foreach (var function in unknownFunctions)
        {
            result.AddWarning($"Unsupported function '{function}' will evaluate to #NAME?.");
        }
    }
}
