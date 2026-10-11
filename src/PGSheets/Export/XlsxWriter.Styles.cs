using System.Globalization;
using System.Xml;
using PGSheets.Ooxml;

namespace PGSheets.Export;

public sealed partial class XlsxWriter
{
    /// <summary>
    /// Writes the style table: a fixed font, fill, border and named style, plus one entry per number
    /// format the workbook actually uses. Excel is strict about this part — every <c>count</c> must
    /// match the number of children, there must be at least two fills, and <c>cellStyleXfs</c> must
    /// contain a plain entry — and getting any of that wrong makes Excel report the file as needing
    /// repair.
    /// </summary>
    private static void WriteStyles(XmlWriter writer, NumberFormatTable numberFormats)
    {
        writer.WriteStartDocument();
        writer.WriteStartElement("styleSheet", SpreadsheetMl.MainNamespace);

        // The specification fixes the order of these children, and numFmts has to come first: a
        // file with them out of order is one Excel offers to repair.
        if (numberFormats.Codes.Count > 0)
        {
            writer.WriteStartElement("numFmts", SpreadsheetMl.MainNamespace);
            writer.WriteAttributeString("count", Count(numberFormats.Codes.Count));

            for (var index = 0; index < numberFormats.Codes.Count; index++)
            {
                writer.WriteStartElement("numFmt", SpreadsheetMl.MainNamespace);
                writer.WriteAttributeString("numFmtId", Count(numberFormats.NumberFormatIdAt(index + 1)));
                writer.WriteAttributeString("formatCode", numberFormats.Codes[index]);
                writer.WriteEndElement();
            }

            writer.WriteEndElement();
        }

        writer.WriteStartElement("fonts", SpreadsheetMl.MainNamespace);
        writer.WriteAttributeString("count", "1");
        writer.WriteStartElement("font", SpreadsheetMl.MainNamespace);
        WriteValued(writer, "sz", "11");
        WriteValued(writer, "name", "Calibri");
        writer.WriteEndElement();
        writer.WriteEndElement();

        writer.WriteStartElement("fills", SpreadsheetMl.MainNamespace);
        writer.WriteAttributeString("count", "2");
        WritePatternFill(writer, "none");
        WritePatternFill(writer, "gray125");
        writer.WriteEndElement();

        writer.WriteStartElement("borders", SpreadsheetMl.MainNamespace);
        writer.WriteAttributeString("count", "1");
        writer.WriteStartElement("border", SpreadsheetMl.MainNamespace);

        foreach (var side in new[] { "left", "right", "top", "bottom", "diagonal" })
        {
            writer.WriteStartElement(side, SpreadsheetMl.MainNamespace);
            writer.WriteEndElement();
        }

        writer.WriteEndElement();
        writer.WriteEndElement();

        writer.WriteStartElement("cellStyleXfs", SpreadsheetMl.MainNamespace);
        writer.WriteAttributeString("count", "1");
        WriteFormat(writer, numFmtId: "0", includeStyleReference: false);
        writer.WriteEndElement();

        writer.WriteStartElement("cellXfs", SpreadsheetMl.MainNamespace);
        writer.WriteAttributeString("count", Count(numberFormats.Codes.Count + 1));

        // Entry 0 is General, which is what a cell with no number format refers to.
        WriteFormat(writer, numFmtId: "0", includeStyleReference: true);

        for (var index = 1; index <= numberFormats.Codes.Count; index++)
        {
            WriteFormat(
                writer,
                numFmtId: Count(numberFormats.NumberFormatIdAt(index)),
                includeStyleReference: true,
                applyNumberFormat: true);
        }

        writer.WriteEndElement();

        writer.WriteStartElement("cellStyles", SpreadsheetMl.MainNamespace);
        writer.WriteAttributeString("count", "1");
        writer.WriteStartElement("cellStyle", SpreadsheetMl.MainNamespace);
        writer.WriteAttributeString("name", "Normal");
        writer.WriteAttributeString("xfId", "0");
        writer.WriteAttributeString("builtinId", "0");
        writer.WriteEndElement();
        writer.WriteEndElement();

        writer.WriteEndElement();
        writer.WriteEndDocument();
    }

    private static string Count(int value) => value.ToString(CultureInfo.InvariantCulture);

    private static void WriteValued(XmlWriter writer, string element, string value)
    {
        writer.WriteStartElement(element, SpreadsheetMl.MainNamespace);
        writer.WriteAttributeString("val", value);
        writer.WriteEndElement();
    }

    private static void WritePatternFill(XmlWriter writer, string patternType)
    {
        writer.WriteStartElement("fill", SpreadsheetMl.MainNamespace);
        writer.WriteStartElement("patternFill", SpreadsheetMl.MainNamespace);
        writer.WriteAttributeString("patternType", patternType);
        writer.WriteEndElement();
        writer.WriteEndElement();
    }

    private static void WriteFormat(
        XmlWriter writer,
        string numFmtId,
        bool includeStyleReference,
        bool applyNumberFormat = false)
    {
        writer.WriteStartElement("xf", SpreadsheetMl.MainNamespace);
        writer.WriteAttributeString("numFmtId", numFmtId);
        writer.WriteAttributeString("fontId", "0");
        writer.WriteAttributeString("fillId", "0");
        writer.WriteAttributeString("borderId", "0");

        if (includeStyleReference)
        {
            writer.WriteAttributeString("xfId", "0");
        }

        if (applyNumberFormat)
        {
            writer.WriteAttributeString("applyNumberFormat", "1");
        }

        writer.WriteEndElement();
    }
}
