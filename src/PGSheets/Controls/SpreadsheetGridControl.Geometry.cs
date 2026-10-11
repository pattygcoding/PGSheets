using System.Globalization;
using Avalonia;
using Avalonia.Media;
using PGSheets.Core;

namespace PGSheets.Controls;

public sealed partial class SpreadsheetGridControl
{
    /// <summary>Left edge (control coordinates) of a column's content area.</summary>
    private double ColumnX(int column) => _headerWidth + column * _columnWidth - _scrollX;

    /// <summary>Top edge (control coordinates) of a row's content area.</summary>
    private double RowY(int row) => _headerHeight + row * _rowHeight - _scrollY;

    /// <summary>Maps a pointer position to a cell. False over the headers or outside the sheet.</summary>
    public bool TryHitTest(Point point, out int row, out int column)
    {
        row = -1;
        column = -1;

        if (point.X < _headerWidth || point.Y < _headerHeight)
        {
            return false;
        }

        var columnIndex = (int)((point.X - _headerWidth + _scrollX) / _columnWidth);
        var rowIndex = (int)((point.Y - _headerHeight + _scrollY) / _rowHeight);

        if (columnIndex < 0 || columnIndex >= _columnCount || rowIndex < 0 || rowIndex >= _rowCount)
        {
            return false;
        }

        row = rowIndex;
        column = columnIndex;
        return true;
    }

    /// <summary>
    /// The on-screen rectangle of a cell, in control coordinates. The host uses this to position the
    /// floating editor exactly over the active cell.
    /// </summary>
    public Rect GetCellBounds(int row, int column) =>
        new(ColumnX(column), RowY(row), _columnWidth, _rowHeight);

    /// <summary>Excel-style column label ("A", "Z", "AA"), cached per column index.</summary>
    private string GetColumnLabel(int column)
    {
        while (_columnLabels.Count <= column)
        {
            _columnLabels.Add(CellAddress.ToColumnName(_columnLabels.Count));
        }

        return _columnLabels[column];
    }

    /// <summary>
    /// Cached <see cref="FormattedText"/> for a cell string. The cache is the biggest allocation win
    /// while scrolling: the same strings recur frame after frame.
    /// </summary>
    private FormattedText GetCellText(string text, bool isError)
    {
        if (_cellTextCache.TryGetValue(text, out var formatted))
        {
            return formatted;
        }

        if (_cellTextCache.Count >= MaxTextCacheEntries)
        {
            _cellTextCache.Clear();
        }

        formatted = new FormattedText(
            text,
            CultureInfo.InvariantCulture,
            FlowDirection.LeftToRight,
            _typeface,
            _fontSize,
            isError ? ErrorTextBrush : TextBrush);

        _cellTextCache[text] = formatted;
        return formatted;
    }

    private FormattedText GetHeaderText(string text)
    {
        if (_headerTextCache.TryGetValue(text, out var formatted))
        {
            return formatted;
        }

        if (_headerTextCache.Count >= MaxTextCacheEntries)
        {
            _headerTextCache.Clear();
        }

        formatted = new FormattedText(
            text,
            CultureInfo.InvariantCulture,
            FlowDirection.LeftToRight,
            _typeface,
            _fontSize - 1d,
            HeaderTextBrush);

        _headerTextCache[text] = formatted;
        return formatted;
    }
}
