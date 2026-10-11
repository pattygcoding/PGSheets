using Avalonia;
using Avalonia.Input;

namespace PGSheets.Controls;

public sealed partial class SpreadsheetGridControl
{
    protected override void OnPointerPressed(PointerPressedEventArgs e)
    {
        base.OnPointerPressed(e);

        if (!e.GetCurrentPoint(this).Properties.IsLeftButtonPressed)
        {
            return;
        }

        var point = e.GetPosition(this);

        // Column header strip: make the top cell of that column active.
        if (point.Y < _headerHeight && point.X >= _headerWidth)
        {
            if (TryHitTest(new Point(point.X, _headerHeight + 1d), out _, out var headerColumn))
            {
                SetActiveCell(0, headerColumn);
                Focus();
                e.Handled = true;
            }

            return;
        }

        // Row header strip: make the first cell of that row active.
        if (point.X < _headerWidth && point.Y >= _headerHeight)
        {
            if (TryHitTest(new Point(_headerWidth + 1d, point.Y), out var headerRow, out _))
            {
                SetActiveCell(headerRow, 0);
                Focus();
                e.Handled = true;
            }

            return;
        }

        if (!TryHitTest(point, out var row, out var column))
        {
            return;
        }

        SetActiveCell(row, column);
        Focus();

        if (e.ClickCount >= 2)
        {
            // Double-click edits the cell's existing content.
            RequestEdit(row, column, seedText: null);
        }

        e.Pointer.Capture(this);
        e.Handled = true;
    }

    protected override void OnPointerMoved(PointerEventArgs e)
    {
        base.OnPointerMoved(e);

        if (!ReferenceEquals(e.Pointer.Captured, this))
        {
            return;
        }

        if (!e.GetCurrentPoint(this).Properties.IsLeftButtonPressed)
        {
            return;
        }

        if (TryHitTest(e.GetPosition(this), out var row, out var column))
        {
            SetActiveCell(row, column);
        }
    }

    protected override void OnPointerReleased(PointerReleasedEventArgs e)
    {
        base.OnPointerReleased(e);

        if (ReferenceEquals(e.Pointer.Captured, this))
        {
            e.Pointer.Capture(null);
        }
    }

    protected override void OnPointerWheelChanged(PointerWheelEventArgs e)
    {
        base.OnPointerWheelChanged(e);

        var delta = e.Delta;

        if (delta.X == 0d && delta.Y == 0d)
        {
            return;
        }

        SetScrollOffset(_scrollX - delta.X * 3d * _columnWidth, _scrollY - delta.Y * 3d * _rowHeight);
        e.Handled = true;
    }

    protected override void OnKeyDown(KeyEventArgs e)
    {
        base.OnKeyDown(e);

        // Leave accelerator combinations (Ctrl+C, Ctrl+O...) to the host window.
        if (e.KeyModifiers.HasFlag(KeyModifiers.Control) || e.KeyModifiers.HasFlag(KeyModifiers.Alt))
        {
            return;
        }

        var pageRows = Math.Max(1, (int)(ViewportHeight / _rowHeight) - 1);
        var pageColumns = Math.Max(1, (int)(ViewportWidth / _columnWidth) - 1);

        switch (e.Key)
        {
            case Key.Left:
                MoveActiveBy(0, -1);
                break;
            case Key.Right:
                MoveActiveBy(0, 1);
                break;
            case Key.Up:
                MoveActiveBy(-1, 0);
                break;
            case Key.Down:
                MoveActiveBy(1, 0);
                break;
            case Key.PageUp:
                MoveActiveBy(-pageRows, 0);
                break;
            case Key.PageDown:
                MoveActiveBy(pageRows, 0);
                break;
            case Key.Home:
                SetActiveCell(_activeRow, 0);
                break;
            case Key.End:
                SetActiveCell(_activeRow, _columnCount - 1);
                break;
            case Key.Enter:
                MoveActiveBy(1, 0);
                break;
            case Key.Tab:
                MoveActiveBy(0, 1);
                break;
            case Key.F2:
                RequestEdit(_activeRow, _activeColumn, seedText: null);
                break;
            case Key.Delete:
            case Key.Back:
                ClearActiveCell();
                break;
            default:
                return;
        }

        e.Handled = true;
    }

    /// <summary>Typing a printable character starts an in-place edit seeded with that character.</summary>
    protected override void OnTextInput(TextInputEventArgs e)
    {
        base.OnTextInput(e);

        var text = e.Text;

        if (string.IsNullOrEmpty(text) || char.IsControl(text[0]))
        {
            return;
        }

        RequestEdit(_activeRow, _activeColumn, text);
        e.Handled = true;
    }

    /// <summary>Writes raw input for a cell through the model, which recalculates dependants.</summary>
    public void CommitEdit(int row, int column, string text)
    {
        Model?.SetRawInput(row, column, text ?? string.Empty);
        InvalidateVisual();
    }

    /// <summary>Empties the active cell.</summary>
    public void ClearActiveCell() => CommitEdit(_activeRow, _activeColumn, string.Empty);

    private void RequestEdit(int row, int column, string? seedText) =>
        EditRequested?.Invoke(this, new EditRequestedEventArgs(row, column, seedText));
}
