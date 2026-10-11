using Avalonia;
using PGSheets.Core;

namespace PGSheets.Controls;

public sealed partial class SpreadsheetGridControl
{
    /// <summary>Horizontal scroll position in pixels.</summary>
    public double ScrollOffsetX => _scrollX;

    /// <summary>Vertical scroll position in pixels.</summary>
    public double ScrollOffsetY => _scrollY;

    /// <summary>Total content width, used to size the horizontal scroll bar.</summary>
    public double ExtentWidth => _columnCount * _columnWidth;

    /// <summary>Total content height, used to size the vertical scroll bar.</summary>
    public double ExtentHeight => _rowCount * _rowHeight;

    /// <summary>Width of the scrollable body (everything right of the row header).</summary>
    public double ViewportWidth => Math.Max(0d, Bounds.Width - _headerWidth);

    /// <summary>Height of the scrollable body (everything below the column header).</summary>
    public double ViewportHeight => Math.Max(0d, Bounds.Height - _headerHeight);

    public int ActiveRow => _activeRow;

    public int ActiveColumn => _activeColumn;

    /// <summary>
    /// The selected block, always containing the active cell. Normalised, so the anchor may be
    /// either corner; a single cell when nothing has been extended.
    /// </summary>
    public CellRange Selection => new(_anchorRow, _anchorColumn, _activeRow, _activeColumn);

    /// <summary>Raised whenever the active cell or the selected range changes.</summary>
    public event EventHandler<CellSelectionChangedEventArgs>? SelectionChanged;

    /// <summary>Raised when the host should show the in-place editor for a cell.</summary>
    public event EventHandler<EditRequestedEventArgs>? EditRequested;

    /// <summary>Raised when the scroll offset, extent or viewport size changes.</summary>
    public event EventHandler? ViewportChanged;

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);

        if (change.Property == ModelProperty)
        {
            var previous = change.GetOldValue<SpreadsheetModel?>();
            var current = change.GetNewValue<SpreadsheetModel?>();

            if (previous is not null)
            {
                previous.Changed -= OnModelChanged;
            }

            if (current is not null)
            {
                current.Changed += OnModelChanged;
            }

            ResetView();
            RaiseViewportChanged();
            InvalidateVisual();
        }
        else if (change.Property == BoundsProperty)
        {
            // A resize changes what "fully scrolled" means, and changes the scroll bar geometry.
            ClampScroll();
            RaiseViewportChanged();
        }
    }

    private void OnModelChanged(object? sender, EventArgs e) => InvalidateVisual();

    /// <summary>Scrolls back to A1 and collapses the selection onto the first cell.</summary>
    public void ResetView()
    {
        _scrollX = 0d;
        _scrollY = 0d;

        var changed = _activeRow != 0 || _activeColumn != 0 || _anchorRow != 0 || _anchorColumn != 0;
        _activeRow = 0;
        _activeColumn = 0;
        _anchorRow = 0;
        _anchorColumn = 0;

        if (changed)
        {
            SelectionChanged?.Invoke(this, new CellSelectionChangedEventArgs(0, 0, Selection));
        }
    }

    /// <summary>Sets the scroll offset, clamped to the valid range.</summary>
    public void SetScrollOffset(double x, double y)
    {
        var maxX = Math.Max(0d, ExtentWidth - ViewportWidth);
        var maxY = Math.Max(0d, ExtentHeight - ViewportHeight);

        var clampedX = Math.Clamp(x, 0d, maxX);
        var clampedY = Math.Clamp(y, 0d, maxY);

        if (Math.Abs(clampedX - _scrollX) < 0.001d && Math.Abs(clampedY - _scrollY) < 0.001d)
        {
            return;
        }

        _scrollX = clampedX;
        _scrollY = clampedY;

        InvalidateVisual();
        RaiseViewportChanged();
    }

    /// <summary>Moves the active cell by a delta, collapsing the selection onto it.</summary>
    public void MoveActiveBy(int rowDelta, int columnDelta) =>
        SetActiveCell(_activeRow + rowDelta, _activeColumn + columnDelta);

    /// <summary>Moves the active cell by a delta while keeping the anchor, growing the selection.</summary>
    public void ExtendActiveBy(int rowDelta, int columnDelta) =>
        ExtendSelectionTo(_activeRow + rowDelta, _activeColumn + columnDelta);

    /// <summary>Makes a cell active and collapses the selection onto it.</summary>
    public void SetActiveCell(int row, int column) => SetSelection(row, column, row, column);

    /// <summary>Moves the active cell while keeping the anchor, growing or shrinking the selection.</summary>
    public void ExtendSelectionTo(int row, int column) =>
        SetSelection(_anchorRow, _anchorColumn, row, column);

    /// <summary>Selects a whole block.</summary>
    public void SelectRange(CellRange range) =>
        SetSelection(range.Row1, range.Column1, range.Row2, range.Column2);

    /// <summary>
    /// Selects the smallest rectangle covering every populated cell, or A1 when the sheet is empty.
    /// </summary>
    public void SelectUsedRange()
    {
        var model = Model;

        if (model is null)
        {
            return;
        }

        var minRow = int.MaxValue;
        var minColumn = int.MaxValue;
        var maxRow = int.MinValue;
        var maxColumn = int.MinValue;

        foreach (var (row, column) in model.PopulatedCells)
        {
            minRow = Math.Min(minRow, row);
            maxRow = Math.Max(maxRow, row);
            minColumn = Math.Min(minColumn, column);
            maxColumn = Math.Max(maxColumn, column);
        }

        // Starting from A1 would sweep in empty leading rows and columns, so the box is the data's.
        SelectRange(maxRow < 0
            ? new CellRange(0, 0, 0, 0)
            : new CellRange(minRow, minColumn, maxRow, maxColumn));
    }

    private void SetSelection(int anchorRow, int anchorColumn, int activeRow, int activeColumn)
    {
        anchorRow = Math.Clamp(anchorRow, 0, _rowCount - 1);
        anchorColumn = Math.Clamp(anchorColumn, 0, _columnCount - 1);
        activeRow = Math.Clamp(activeRow, 0, _rowCount - 1);
        activeColumn = Math.Clamp(activeColumn, 0, _columnCount - 1);

        if (anchorRow == _anchorRow &&
            anchorColumn == _anchorColumn &&
            activeRow == _activeRow &&
            activeColumn == _activeColumn)
        {
            EnsureCellVisible(activeRow, activeColumn);
            return;
        }

        _anchorRow = anchorRow;
        _anchorColumn = anchorColumn;
        _activeRow = activeRow;
        _activeColumn = activeColumn;

        EnsureCellVisible(activeRow, activeColumn);
        SelectionChanged?.Invoke(this, new CellSelectionChangedEventArgs(activeRow, activeColumn, Selection));
        InvalidateVisual();
    }

    private void ClampScroll()
    {
        var maxX = Math.Max(0d, ExtentWidth - ViewportWidth);
        var maxY = Math.Max(0d, ExtentHeight - ViewportHeight);

        _scrollX = Math.Clamp(_scrollX, 0d, maxX);
        _scrollY = Math.Clamp(_scrollY, 0d, maxY);
    }

    private void EnsureCellVisible(int row, int column)
    {
        var left = column * _columnWidth;
        var right = left + _columnWidth;
        var top = row * _rowHeight;
        var bottom = top + _rowHeight;

        var x = _scrollX;
        var y = _scrollY;

        if (left < x)
        {
            x = left;
        }
        else if (right > x + ViewportWidth)
        {
            x = right - ViewportWidth;
        }

        if (top < y)
        {
            y = top;
        }
        else if (bottom > y + ViewportHeight)
        {
            y = bottom - ViewportHeight;
        }

        SetScrollOffset(x, y);
    }

    private void RaiseViewportChanged() => ViewportChanged?.Invoke(this, EventArgs.Empty);
}
