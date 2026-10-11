using System.Xml;
using PGSheets.Ooxml;

namespace PGSheets.Export;

public sealed partial class XlsxWriter
{
    private static void WriteContentTypes(XmlWriter writer)
    {
        writer.WriteStartDocument();
        writer.WriteStartElement("Types", SpreadsheetMl.ContentTypesNamespace);

        WriteDefaultType(writer, "rels", "application/vnd.openxmlformats-package.relationships+xml");
        WriteDefaultType(writer, "xml", "application/xml");

        WriteOverrideType(
            writer,
            "/xl/workbook.xml",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml");

        WriteOverrideType(
            writer,
            "/xl/worksheets/sheet1.xml",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml");

        WriteOverrideType(
            writer,
            "/xl/styles.xml",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml");

        writer.WriteEndElement();
        writer.WriteEndDocument();
    }

    private static void WriteRootRelationships(XmlWriter writer)
    {
        writer.WriteStartDocument();
        writer.WriteStartElement("Relationships", SpreadsheetMl.PackageRelationshipNamespace);
        WriteRelationship(writer, "rId1", OfficeDocumentRelationship, "xl/workbook.xml");
        writer.WriteEndElement();
        writer.WriteEndDocument();
    }

    private static void WriteWorkbook(XmlWriter writer, string sheetName)
    {
        writer.WriteStartDocument();
        writer.WriteStartElement("workbook", SpreadsheetMl.MainNamespace);

        // The r: prefix has to be declared for the sheet's r:id attribute to resolve.
        writer.WriteAttributeString("xmlns", "r", null, SpreadsheetMl.RelationshipNamespace);

        writer.WriteStartElement("sheets", SpreadsheetMl.MainNamespace);
        writer.WriteStartElement("sheet", SpreadsheetMl.MainNamespace);
        writer.WriteAttributeString("name", sheetName);
        writer.WriteAttributeString("sheetId", "1");
        writer.WriteAttributeString("r", "id", SpreadsheetMl.RelationshipNamespace, "rId1");
        writer.WriteEndElement();
        writer.WriteEndElement();

        writer.WriteEndElement();
        writer.WriteEndDocument();
    }

    private static void WriteWorkbookRelationships(XmlWriter writer)
    {
        writer.WriteStartDocument();
        writer.WriteStartElement("Relationships", SpreadsheetMl.PackageRelationshipNamespace);
        WriteRelationship(writer, "rId1", WorksheetRelationship, "worksheets/sheet1.xml");
        WriteRelationship(writer, "rId2", StylesRelationship, "styles.xml");
        writer.WriteEndElement();
        writer.WriteEndDocument();
    }

    private static void WriteDefaultType(XmlWriter writer, string extension, string contentType)
    {
        writer.WriteStartElement("Default", SpreadsheetMl.ContentTypesNamespace);
        writer.WriteAttributeString("Extension", extension);
        writer.WriteAttributeString("ContentType", contentType);
        writer.WriteEndElement();
    }

    private static void WriteOverrideType(XmlWriter writer, string partName, string contentType)
    {
        writer.WriteStartElement("Override", SpreadsheetMl.ContentTypesNamespace);
        writer.WriteAttributeString("PartName", partName);
        writer.WriteAttributeString("ContentType", contentType);
        writer.WriteEndElement();
    }

    private static void WriteRelationship(XmlWriter writer, string id, string type, string target)
    {
        writer.WriteStartElement("Relationship", SpreadsheetMl.PackageRelationshipNamespace);
        writer.WriteAttributeString("Id", id);
        writer.WriteAttributeString("Type", type);
        writer.WriteAttributeString("Target", target);
        writer.WriteEndElement();
    }
}
