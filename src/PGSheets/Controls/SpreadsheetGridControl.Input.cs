using Avalonia;
using Avalonia.Input;
using PGSheets.Core;

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

        // Column header strip: select the whole column.
        if (point.Y < _headerHeight && point.X >= _headerWidth)
        {
            if (TryHitTest(new Point(point.X, _headerHeight + 1d), out _, out var headerColumn))
            {
                SelectRange(new CellRange(0, headerColumn, _rowCount - 1, headerColumn));
                Focus();
                e.Handled = true;
            }

            return;
        }

        // Row header strip: select the whole row.
        if (point.X < _headerWidth && point.Y >= _headerHeight)
        {
            if (TryHitTest(new Point(_headerWidth + 1d, point.Y), out var headerRow, out _))
            {
                SelectRange(new CellRange(headerRow, 0, headerRow, _columnCount - 1));
                Focus();
                e.Handled = true;
            }

            return;
        }

        if (!TryHitTest(point, out var row, out var column))
        {
            return;
        }

        // Shift+click grows the existing selection instead of starting a new one.
        if (e.KeyModifiers.HasFlag(KeyModifiers.Shift))
        {
            ExtendSelectionTo(row, column);
        }
        else
        {
            SetActiveCell(row, column);
        }

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

        // Dragging extends the selection rather than moving it.
        if (TryHitTest(e.GetPosition(this), out var row, out var column))
        {
            ExtendSelectionTo(row, column);
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

        var control = e.KeyModifiers.HasFlag(KeyModifiers.Control);
        var shift = e.KeyModifiers.HasFlag(KeyModifiers.Shift);
        var alt = e.KeyModifiers.HasFlag(KeyModifiers.Alt);

        // Ctrl+A is a selection concern, so the grid keeps it. Every other accelerator combination
        // (Ctrl+S, Ctrl+O, Ctrl+C...) belongs to the host and is left to bubble.
        if (control && !alt && !shift && e.Key == Key.A)
        {
            SelectUsedRange();
            e.Handled = true;
            return;
        }

        if (control && !alt && e.Key is Key.Home or Key.End)
        {
            var atStart = e.Key == Key.Home;

            if (shift)
            {
                ExtendSelectionTo(atStart ? 0 : _rowCount - 1, atStart ? 0 : _columnCount - 1);
            }
            else
            {
                SetActiveCell(atStart ? 0 : _rowCount - 1, atStart ? 0 : _columnCount - 1);
            }

            e.Handled = true;
            return;
        }

        if (control || alt)
        {
            return;
        }

        var pageRows = Math.Max(1, (int)(ViewportHeight / _rowHeight) - 1);
        var pageColumns = Math.Max(1, (int)(ViewportWidth / _columnWidth) - 1);

        switch (e.Key)
        {
            case Key.Left:
                Move(shift, 0, -1);
                break;
            case Key.Right:
                Move(shift, 0, 1);
                break;
            case Key.Up:
                Move(shift, -1, 0);
                break;
            case Key.Down:
                Move(shift, 1, 0);
                break;
            case Key.PageUp:
                Move(shift, -pageRows, 0);
                break;
            case Key.PageDown:
                Move(shift, pageRows, 0);
                break;
            case Key.Home:
                MoveToRowStart(shift);
                break;
            case Key.End:
                MoveToRowEnd(shift);
                break;
            case Key.Enter:
                MoveActiveBy(1, 0);
                break;
            case Key.Tab:
                MoveActiveBy(0, shift ? -1 : 1);
                break;
            case Key.F2:
                RequestEdit(_activeRow, _activeColumn, seedText: null);
                break;
            case Key.Delete:
            case Key.Back:
                ClearSelection();
                break;
            default:
                return;
        }

        e.Handled = true;
    }

    private void Move(bool extend, int rowDelta, int columnDelta)
    {
        if (extend)
        {
            ExtendActiveBy(rowDelta, columnDelta);
        }
        else
        {
            MoveActiveBy(rowDelta, columnDelta);
        }
    }

    private void MoveToRowStart(bool extend)
    {
        if (extend)
        {
            ExtendSelectionTo(_activeRow, 0);
        }
        else
        {
            SetActiveCell(_activeRow, 0);
        }
    }

    private void MoveToRowEnd(bool extend)
    {
        if (extend)
        {
            ExtendSelectionTo(_activeRow, _columnCount - 1);
        }
        else
        {
            SetActiveCell(_activeRow, _columnCount - 1);
        }
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

    /// <summary>
    /// Empties every populated cell in the selection. The whole thing is one undo step, and only
    /// cells that exist are touched, so clearing a whole column stays cheap.
    /// </summary>
    public void ClearSelection()
    {
        var model = Model;

        if (model is null)
        {
            return;
        }

        var range = Selection;

        if (range.IsSingleCell)
        {
            CommitEdit(range.Row1, range.Column1, string.Empty);
            return;
        }

        // Gather first: mutating the dictionary while enumerating it is not allowed.
        var targets = new List<(int Row, int Column)>();

        foreach (var (row, column) in model.PopulatedCells)
        {
            if (range.Contains(row, column))
            {
                targets.Add((row, column));
            }
        }

        if (targets.Count == 0)
        {
            return;
        }

        model.BeginBulkUpdate();

        try
        {
            foreach (var (row, column) in targets)
            {
                model.SetRawInput(row, column, string.Empty);
            }
        }
        finally
        {
            model.EndBulkUpdate();
        }

        InvalidateVisual();
    }

    private void RequestEdit(int row, int column, string? seedText) =>
        EditRequested?.Invoke(this, new EditRequestedEventArgs(row, column, seedText));
}
