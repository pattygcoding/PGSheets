using Avalonia;
using Avalonia.Media;

namespace PGSheets.Controls;

public sealed partial class SpreadsheetGridControl
{
    /// <summary>
    /// Single paint pass for the whole control. Only the cells intersecting the viewport are visited,
    /// and the loop bounds are derived from <see cref="Control.Bounds"/> and the scroll offsets.
    /// </summary>
    public override void Render(DrawingContext context)
    {
        var width = Bounds.Width;
        var height = Bounds.Height;

        if (width <= 0d || height <= 0d)
        {
            return;
        }

        context.FillRectangle(BackgroundBrush, new Rect(0d, 0d, width, height));

        // Viewport window: the only cells that reach the render path.
        var firstColumn = Math.Max(0, (int)(_scrollX / _columnWidth));
        var firstRow = Math.Max(0, (int)(_scrollY / _rowHeight));
        var lastColumn = Math.Min(_columnCount - 1, firstColumn + (int)(ViewportWidth / _columnWidth) + 1);
        var lastRow = Math.Min(_rowCount - 1, firstRow + (int)(ViewportHeight / _rowHeight) + 1);

        var body = new Rect(_headerWidth, _headerHeight, ViewportWidth, ViewportHeight);

        if (body.Width > 0d && body.Height > 0d)
        {
            using (context.PushClip(body))
            {
                DrawCells(context, firstRow, lastRow, firstColumn, lastColumn);
                DrawGridLines(context, firstRow, lastRow, firstColumn, lastColumn);
                DrawSelectionHighlight(context);
            }
        }

        DrawColumnHeader(context, firstColumn, lastColumn);
        DrawRowHeader(context, firstRow, lastRow);
        DrawHeaderCorner(context);
    }

    private void DrawCells(DrawingContext context, int firstRow, int lastRow, int firstColumn, int lastColumn)
    {
        var model = Model;

        if (model is null)
        {
            return;
        }

        for (var row = firstRow; row <= lastRow; row++)
        {
            var y = RowY(row);

            for (var column = firstColumn; column <= lastColumn; column++)
            {
                if (!model.TryGetCell(row, column, out var cell))
                {
                    continue;
                }

                var text = cell.DisplayValue;

                if (text.Length == 0)
                {
                    continue;
                }

                var x = ColumnX(column);
                var formatted = GetCellText(text, cell.Value.IsError);
                var textY = y + (_rowHeight - formatted.Height) / 2d;
                var textX = x + CellPadding;

                if (formatted.Width > _columnWidth - CellPadding * 2d)
                {
                    // Only cells that actually overflow pay for a clip.
                    using (context.PushClip(new Rect(x, y, _columnWidth, _rowHeight)))
                    {
                        context.DrawText(formatted, new Point(textX, textY));
                    }

                    continue;
                }

                context.DrawText(formatted, new Point(textX, textY));
            }
        }
    }

    private void DrawGridLines(DrawingContext context, int firstRow, int lastRow, int firstColumn, int lastColumn)
    {
        var top = _headerHeight;
        var bottom = Bounds.Height;

        for (var column = firstColumn; column <= lastColumn + 1 && column <= _columnCount; column++)
        {
            var x = ColumnX(column);

            if (x < _headerWidth)
            {
                continue;
            }

            context.DrawLine(GridPen, new Point(x, top), new Point(x, bottom));
        }

        var left = _headerWidth;
        var right = Bounds.Width;

        for (var row = firstRow; row <= lastRow + 1 && row <= _rowCount; row++)
        {
            var y = RowY(row);

            if (y < _headerHeight)
            {
                continue;
            }

            context.DrawLine(GridPen, new Point(left, y), new Point(right, y));
        }
    }

    /// <summary>Shades the selected block and outlines it, with the active cell outlined inside.</summary>
    private void DrawSelectionHighlight(DrawingContext context)
    {
        var range = Selection;

        var rect = new Rect(
            ColumnX(range.Column1),
            RowY(range.Row1),
            range.ColumnCount * _columnWidth,
            range.RowCount * _rowHeight);

        context.FillRectangle(SelectionFillBrush, rect);
        context.DrawRectangle(null, SelectionPen, rect);

        // A big selection would otherwise hide which cell keyboard input will land in.
        if (!range.IsSingleCell)
        {
            context.DrawRectangle(null, SelectionPen, GetCellBounds(_activeRow, _activeColumn));
        }
    }
}
