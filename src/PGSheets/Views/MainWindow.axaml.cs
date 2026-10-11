using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
using PGSheets.Controls;
using PGSheets.Core;
using PGSheets.Formulas;
using PGSheets.Import;

namespace PGSheets.Views;

/// <summary>
/// Hosts the grid, the scroll bars and the single reusable in-place editor.
/// All editor positioning is derived from <see cref="SpreadsheetGridControl.GetCellBounds"/>.
/// </summary>
public partial class MainWindow : Window
{
    private readonly FormulaEngine _engine;
    private readonly SpreadsheetModel _model;

    private bool _syncingScroll;
    private bool _editing;
    private int _editRow;
    private int _editColumn;

    public MainWindow()
    {
        InitializeComponent();

        _engine = new FormulaEngine();
        _model = new SpreadsheetModel(_engine);

        SheetGrid.Model = _model;

        SheetGrid.SelectionChanged += (_, _) => UpdateActiveCellUi();
        SheetGrid.EditRequested += OnEditRequested;
        SheetGrid.ViewportChanged += OnViewportChanged;

        CellEditor.KeyDown += OnEditorKeyDown;
        CellEditor.LostFocus += (_, _) => CommitEdit();

        FormulaBar.KeyDown += OnFormulaBarKeyDown;

        VScroll.ValueChanged += (_, e) => OnScrollBarChanged(horizontal: false, e.NewValue);
        HScroll.ValueChanged += (_, e) => OnScrollBarChanged(horizontal: true, e.NewValue);

        SeedSampleData();
        UpdateActiveCellUi();

        Loaded += (_, _) =>
        {
            UpdateScrollBars();
            SheetGrid.Focus();
        };
    }

    // ---- Sample content so the window is not empty before a file is opened -------------------

    private void SeedSampleData()
    {
        _model.BeginBulkUpdate();

        _model.SetRawInput(0, 0, "Region");
        _model.SetRawInput(0, 1, "Q1");
        _model.SetRawInput(0, 2, "Q2");
        _model.SetRawInput(0, 3, "Total");

        _model.SetRawInput(1, 0, "North");
        _model.SetRawInput(1, 1, "1200");
        _model.SetRawInput(1, 2, "1450");
        _model.SetRawInput(1, 3, "=B2+C2");

        _model.SetRawInput(2, 0, "South");
        _model.SetRawInput(2, 1, "980");
        _model.SetRawInput(2, 2, "1100");
        _model.SetRawInput(2, 3, "=B3+C3");

        _model.SetRawInput(3, 0, "East");
        _model.SetRawInput(3, 1, "1600");
        _model.SetRawInput(3, 2, "1300");
        _model.SetRawInput(3, 3, "=B4+C4");

        _model.SetRawInput(5, 0, "Totals");
        _model.SetRawInput(5, 1, "=SUM(B2:B4)");
        _model.SetRawInput(5, 2, "=SUM(C2:C4)");
        _model.SetRawInput(5, 3, "=SUM(D2:D4)");

        _model.SetRawInput(7, 0, "Edit any cell, or use File > Open to import .xlsx / .csv.");

        _model.EndBulkUpdate();
    }

    // ---- Selection / formula bar --------------------------------------------------------------

    private void UpdateActiveCellUi()
    {
        var row = SheetGrid.ActiveRow;
        var column = SheetGrid.ActiveColumn;

        NameBox.Text = CellAddress.ToA1(row, column);

        if (!_editing)
        {
            FormulaBar.Text = _model.GetRawInput(row, column);
        }
    }

    private void OnFormulaBarKeyDown(object? sender, KeyEventArgs e)
    {
        if (e.Key == Key.Enter)
        {
            _model.SetRawInput(SheetGrid.ActiveRow, SheetGrid.ActiveColumn, FormulaBar.Text ?? string.Empty);
            SheetGrid.Focus();
            UpdateActiveCellUi();
            e.Handled = true;
        }
        else if (e.Key == Key.Escape)
        {
            UpdateActiveCellUi();
            e.Handled = true;
        }
    }

    // ---- Scroll bar synchronisation -----------------------------------------------------------

    private void OnViewportChanged(object? sender, EventArgs e)
    {
        UpdateScrollBars();
        UpdateEditorPosition();
    }

    private void OnScrollBarChanged(bool horizontal, double value)
    {
        if (_syncingScroll)
        {
            return;
        }

        if (horizontal)
        {
            SheetGrid.SetScrollOffset(value, SheetGrid.ScrollOffsetY);
        }
        else
        {
            SheetGrid.SetScrollOffset(SheetGrid.ScrollOffsetX, value);
        }
    }

    private void UpdateScrollBars()
    {
        _syncingScroll = true;

        var viewportHeight = SheetGrid.ViewportHeight;
        var viewportWidth = SheetGrid.ViewportWidth;

        VScroll.Minimum = 0d;
        VScroll.Maximum = Math.Max(0d, SheetGrid.ExtentHeight - viewportHeight);
        VScroll.ViewportSize = viewportHeight;
        VScroll.SmallChange = SheetGrid.RowHeight;
        VScroll.LargeChange = Math.Max(SheetGrid.RowHeight, viewportHeight);
        VScroll.Value = Math.Min(SheetGrid.ScrollOffsetY, VScroll.Maximum);

        HScroll.Minimum = 0d;
        HScroll.Maximum = Math.Max(0d, SheetGrid.ExtentWidth - viewportWidth);
        HScroll.ViewportSize = viewportWidth;
        HScroll.SmallChange = SheetGrid.ColumnWidth;
        HScroll.LargeChange = Math.Max(SheetGrid.ColumnWidth, viewportWidth);
        HScroll.Value = Math.Min(SheetGrid.ScrollOffsetX, HScroll.Maximum);

        _syncingScroll = false;
    }

    // ---- In-place editor ----------------------------------------------------------------------

    private void OnEditRequested(object? sender, EditRequestedEventArgs e) =>
        BeginEdit(e.Row, e.Column, e.SeedText);

    /// <summary>
    /// Shows the single reusable editor over the target cell. The editor always ends up fully
    /// visible because <c>SetActiveCell</c> scrolls the cell into view first.
    /// </summary>
    private void BeginEdit(int row, int column, string? seedText)
    {
        SheetGrid.SetActiveCell(row, column);

        _editRow = row;
        _editColumn = column;
        _editing = true;

        var text = seedText ?? _model.GetRawInput(row, column);
        CellEditor.Text = text;
        CellEditor.CaretIndex = text.Length;

        UpdateEditorPosition();
        CellEditor.IsVisible = true;
        CellEditor.Focus();
    }

    private void UpdateEditorPosition()
    {
        if (!_editing)
        {
            return;
        }

        var bounds = SheetGrid.GetCellBounds(_editRow, _editColumn);
        CellEditor.Margin = new Thickness(bounds.X, bounds.Y, 0d, 0d);
        CellEditor.Width = Math.Max(40d, bounds.Width);
        CellEditor.Height = Math.Max(20d, bounds.Height);
    }

    private void OnEditorKeyDown(object? sender, KeyEventArgs e)
    {
        switch (e.Key)
        {
            case Key.Enter:
                CommitEdit();
                SheetGrid.MoveActiveBy(1, 0);
                e.Handled = true;
                break;

            case Key.Tab:
                CommitEdit();
                SheetGrid.MoveActiveBy(0, 1);
                e.Handled = true;
                break;

            case Key.Escape:
                CancelEdit();
                e.Handled = true;
                break;
        }
    }

    /// <summary>Commits the editor contents to the model, which recalculates dependants.</summary>
    private void CommitEdit()
    {
        if (!_editing)
        {
            return;
        }

        _editing = false;
        var text = CellEditor.Text ?? string.Empty;
        CellEditor.IsVisible = false;

        _model.SetRawInput(_editRow, _editColumn, text);

        SheetGrid.Focus();
        UpdateActiveCellUi();
    }

    private void CancelEdit()
    {
        if (!_editing)
        {
            return;
        }

        _editing = false;
        CellEditor.IsVisible = false;

        SheetGrid.Focus();
        UpdateActiveCellUi();
    }

    // ---- File menu ----------------------------------------------------------------------------

    private async void OnOpenClicked(object? sender, RoutedEventArgs e) =>
        await OpenSpreadsheetAsync();

    private void OnExitClicked(object? sender, RoutedEventArgs e) => Close();

    protected override void OnKeyDown(KeyEventArgs e)
    {
        if (e.Key == Key.O && e.KeyModifiers.HasFlag(KeyModifiers.Control))
        {
            _ = OpenSpreadsheetAsync();
            e.Handled = true;
            return;
        }

        base.OnKeyDown(e);
    }

    private async Task OpenSpreadsheetAsync()
    {
        var files = await StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = "Open spreadsheet",
            AllowMultiple = false,
            FileTypeFilter = new[]
            {
                new FilePickerFileType("Spreadsheets") { Patterns = SheetImporter.AllPatterns.ToArray() },
                new FilePickerFileType("Excel workbook") { Patterns = new[] { "*.xlsx", "*.xlsm" } },
                new FilePickerFileType("Delimited text") { Patterns = new[] { "*.csv", "*.tsv", "*.txt" } },
            },
        });

        if (files.Count == 0)
        {
            return;
        }

        var file = files[0];

        try
        {
            await using var stream = await file.OpenReadAsync();

            ImportResult result;

            // One recalculation for the whole import rather than one per cell.
            _model.BeginBulkUpdate();

            try
            {
                result = await SheetImporter.ImportAsync(stream, file.Name, _model);
            }
            finally
            {
                _model.EndBulkUpdate();
            }

            SheetGrid.ResetView();
            UpdateScrollBars();
            UpdateActiveCellUi();
            UpdateEditorPosition();

            var summary =
                $"{file.Name} - {result.Rows} x {result.Columns} cells imported ({result.CellsImported} non-empty)";

            if (result.Warnings.Count > 0)
            {
                summary += $" - {result.Warnings.Count} warning(s)";

                foreach (var warning in result.Warnings)
                {
                    System.Diagnostics.Debug.WriteLine(warning);
                }
            }

            StatusText.Text = summary;
        }
        catch (Exception ex)
        {
            StatusText.Text = $"Import failed: {ex.Message}";
        }
    }
}
