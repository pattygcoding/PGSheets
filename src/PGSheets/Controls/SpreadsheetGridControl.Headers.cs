using System.Globalization;
using Avalonia;
using Avalonia.Media;

namespace PGSheets.Controls;

public sealed partial class SpreadsheetGridControl
{
    /// <summary>Sticky column header: A, B, C... with the active column highlighted.</summary>
    private void DrawColumnHeader(DrawingContext context, int firstColumn, int lastColumn)
    {
        if (ViewportWidth <= 0d)
        {
            return;
        }

        var range = Selection;

        context.FillRectangle(HeaderBrush, new Rect(_headerWidth, 0d, ViewportWidth, _headerHeight));

        for (var column = firstColumn; column <= lastColumn; column++)
        {
            var x = ColumnX(column);

            if (x + _columnWidth <= _headerWidth)
            {
                continue;
            }

            if (column >= range.Column1 && column <= range.Column2)
            {
                context.FillRectangle(HeaderActiveBrush, new Rect(x, 0d, _columnWidth, _headerHeight));
            }

            var formatted = GetHeaderText(GetColumnLabel(column));
            context.DrawText(
                formatted,
                new Point(x + (_columnWidth - formatted.Width) / 2d, (_headerHeight - formatted.Height) / 2d));

            context.DrawLine(GridPen, new Point(x, 0d), new Point(x, _headerHeight));
        }

        context.DrawLine(GridPen, new Point(_headerWidth, _headerHeight), new Point(Bounds.Width, _headerHeight));
    }

    /// <summary>Sticky row header: 1, 2, 3... with the active row highlighted.</summary>
    private void DrawRowHeader(DrawingContext context, int firstRow, int lastRow)
    {
        if (ViewportHeight <= 0d)
        {
            return;
        }

        var range = Selection;

        context.FillRectangle(HeaderBrush, new Rect(0d, _headerHeight, _headerWidth, ViewportHeight));

        for (var row = firstRow; row <= lastRow; row++)
        {
            var y = RowY(row);

            if (y + _rowHeight <= _headerHeight)
            {
                continue;
            }

            if (row >= range.Row1 && row <= range.Row2)
            {
                context.FillRectangle(HeaderActiveBrush, new Rect(0d, y, _headerWidth, _rowHeight));
            }

            var formatted = GetHeaderText((row + 1).ToString(CultureInfo.InvariantCulture));
            context.DrawText(
                formatted,
                new Point((_headerWidth - formatted.Width) / 2d, y + (_rowHeight - formatted.Height) / 2d));

            context.DrawLine(GridPen, new Point(0d, y), new Point(_headerWidth, y));
        }

        context.DrawLine(GridPen, new Point(_headerWidth, _headerHeight), new Point(_headerWidth, Bounds.Height));
    }

    /// <summary>Small fixed square where the two header strips meet.</summary>
    private void DrawHeaderCorner(DrawingContext context)
    {
        context.FillRectangle(HeaderBrush, new Rect(0d, 0d, _headerWidth, _headerHeight));
        context.DrawLine(GridPen, new Point(0d, _headerHeight), new Point(_headerWidth, _headerHeight));
        context.DrawLine(GridPen, new Point(_headerWidth, 0d), new Point(_headerWidth, _headerHeight));
    }
}
