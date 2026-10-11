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

    /// <summary>Raised whenever the active cell changes.</summary>
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

    /// <summary>Scrolls back to A1 and makes the first cell active.</summary>
    public void ResetView()
    {
        _scrollX = 0d;
        _scrollY = 0d;

        var changed = _activeRow != 0 || _activeColumn != 0;
        _activeRow = 0;
        _activeColumn = 0;

        if (changed)
        {
            SelectionChanged?.Invoke(this, new CellSelectionChangedEventArgs(0, 0));
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

    /// <summary>Moves the active cell by a delta, scrolling it into view.</summary>
    public void MoveActiveBy(int rowDelta, int columnDelta) =>
        SetActiveCell(_activeRow + rowDelta, _activeColumn + columnDelta);

    /// <summary>Makes a cell active, scrolling it fully into view.</summary>
    public void SetActiveCell(int row, int column)
    {
        row = Math.Clamp(row, 0, _rowCount - 1);
        column = Math.Clamp(column, 0, _columnCount - 1);

        if (row == _activeRow && column == _activeColumn)
        {
            EnsureCellVisible(row, column);
            return;
        }

        _activeRow = row;
        _activeColumn = column;

        EnsureCellVisible(row, column);
        SelectionChanged?.Invoke(this, new CellSelectionChangedEventArgs(row, column));
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
