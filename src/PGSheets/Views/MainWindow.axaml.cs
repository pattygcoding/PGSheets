using System.Globalization;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Input.Platform;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
using PGSheets.Controls;
using PGSheets.Core;
using PGSheets.Export;
using PGSheets.Formulas;
using PGSheets.Import;

namespace PGSheets.Views;

/// <summary>
/// Hosts the grid, the scroll bars and the single reusable in-place editor.
/// All editor positioning is derived from <see cref="SpreadsheetGridControl.GetCellBounds"/>.
/// </summary>
public partial class MainWindow : Window
{
    /// <summary>
    /// Above this many populated cells the selection summary is skipped: the scan runs on every
    /// selection change, including each step of a drag, so it has to stay cheaper than rendering.
    /// </summary>
    private const int MaxSummaryCells = 20_000;

    private readonly FormulaEngine _engine;
    private readonly SpreadsheetModel _model;

    private bool _syncingScroll;
    private bool _editing;
    private int _editRow;
    private int _editColumn;

    private IStorageFile? _currentFile;
    private string? _currentFileName;
    private string _sheetName = "Sheet1";

    // How many worksheets the file on screen came from. Saving writes one, so anything above one
    // means the original must not be overwritten in place.
    private int _loadedWorksheetCount = 1;

    // Dirty tracking drives the title bar and the close prompt. Loading a file and seeding the
    // sample sheet both raise Changed, so they mute it rather than counting as user edits.
    private bool _isDirty;
    private bool _suppressDirty;
    private bool _forceClose;

    // The region and text of the last copy made in this app. The clipboard carries only text, so
    // remembering these is what lets a paste shift relative references the way a spreadsheet does.
    private CellRange? _copiedRange;
    private string? _copiedText;

    public MainWindow()
    {
        InitializeComponent();

        _engine = new FormulaEngine();
        _model = new SpreadsheetModel(_engine);

        SheetGrid.Model = _model;

        SheetGrid.SelectionChanged += (_, _) => UpdateActiveCellUi();
        SheetGrid.EditRequested += OnEditRequested;
        SheetGrid.ViewportChanged += OnViewportChanged;

        _model.Changed += OnModelChanged;

        CellEditor.KeyDown += OnEditorKeyDown;
        CellEditor.LostFocus += (_, _) => CommitEdit();

        FormulaBar.KeyDown += OnFormulaBarKeyDown;

        VScroll.ValueChanged += (_, e) => OnScrollBarChanged(horizontal: false, e.NewValue);
        HScroll.ValueChanged += (_, e) => OnScrollBarChanged(horizontal: true, e.NewValue);

        _model.Undo.Changed += (_, _) => UpdateUndoState();

        // Seeding the sample sheet is not a user edit, so neither dirty tracking nor undo sees it.
        _suppressDirty = true;

        using (_model.SuspendUndoHistory())
        {
            SeedSampleData();
        }

        _suppressDirty = false;

        UpdateActiveCellUi();
        UpdateTitle();
        UpdateUndoState();

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

        var selection = SheetGrid.Selection;

        // The name box shows the whole block, as a spreadsheet does for a range.
        NameBox.Text = selection.IsSingleCell ? CellAddress.ToA1(row, column) : selection.ToString();

        if (!_editing)
        {
            FormulaBar.Text = _model.GetRawInput(row, column);
        }

        UpdateSelectionSummary(selection);
    }

    /// <summary>
    /// Shows an Excel-style aggregate of the numbers in the selection. Only populated cells are
    /// visited, so the cost tracks the data rather than the size of the selection.
    /// </summary>
    private void UpdateSelectionSummary(CellRange selection)
    {
        if (selection.IsSingleCell)
        {
            SelectionText.Text = string.Empty;
            return;
        }

        if (_model.CellCount > MaxSummaryCells)
        {
            SelectionText.Text = $"{selection.CellCount:N0} selected";
            return;
        }

        var numbers = 0;
        var total = 0d;

        foreach (var (row, column) in _model.PopulatedCells)
        {
            if (!selection.Contains(row, column))
            {
                continue;
            }

            var value = _model.GetValue(row, column);

            if (value.Kind == CellValueKind.Number)
            {
                numbers++;
                total += value.Number;
            }
        }

        if (numbers == 0)
        {
            SelectionText.Text = $"{selection.CellCount:N0} cells";
            return;
        }

        SelectionText.Text =
            $"Count: {numbers:N0}    Sum: {total.ToString("G15", CultureInfo.InvariantCulture)}" +
            $"    Average: {(total / numbers).ToString("G15", CultureInfo.InvariantCulture)}";
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

    private async void OnNewClicked(object? sender, RoutedEventArgs e) => await NewDocumentAsync();

    private async void OnOpenClicked(object? sender, RoutedEventArgs e) => await OpenSpreadsheetAsync();

    private async void OnSaveClicked(object? sender, RoutedEventArgs e) => await SaveAsync();

    private async void OnSaveAsClicked(object? sender, RoutedEventArgs e) => await SaveAsAsync();

    private void OnExitClicked(object? sender, RoutedEventArgs e) => Close();

    protected override void OnKeyDown(KeyEventArgs e)
    {
        if (e.KeyModifiers.HasFlag(KeyModifiers.Control))
        {
            // While the in-place editor is open these belong to the TextBox, so the event is left
            // to the normal handling instead of being turned into a grid action.
            if (_editing && e.Key is Key.C or Key.X or Key.V or Key.Z or Key.Y)
            {
                base.OnKeyDown(e);
                return;
            }

            switch (e.Key)
            {
                case Key.N:
                    _ = NewDocumentAsync();
                    e.Handled = true;
                    return;

                case Key.O:
                    _ = OpenSpreadsheetAsync();
                    e.Handled = true;
                    return;

                // Ctrl+Shift+S must be matched before plain Ctrl+S.
                case Key.S when e.KeyModifiers.HasFlag(KeyModifiers.Shift):
                    _ = SaveAsAsync();
                    e.Handled = true;
                    return;

                case Key.S:
                    _ = SaveAsync();
                    e.Handled = true;
                    return;

                case Key.Z when e.KeyModifiers.HasFlag(KeyModifiers.Shift):
                    Redo();
                    e.Handled = true;
                    return;

                case Key.Z:
                    Undo();
                    e.Handled = true;
                    return;

                case Key.Y:
                    Redo();
                    e.Handled = true;
                    return;

                case Key.C:
                    _ = CopySelectionAsync();
                    e.Handled = true;
                    return;

                case Key.X:
                    _ = CutSelectionAsync();
                    e.Handled = true;
                    return;

                case Key.V:
                    _ = PasteAsync();
                    e.Handled = true;
                    return;
            }
        }

        base.OnKeyDown(e);
    }

    private async Task OpenSpreadsheetAsync()
    {
        if (!await ConfirmUnsavedChangesAsync("opening another file"))
        {
            return;
        }

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
            _suppressDirty = true;
            _model.BeginBulkUpdate();

            // Loading replaces the document, so it must not become an undo step of its own.
            using var undoScope = _model.SuspendUndoHistory();

            try
            {
                // Importing replaces the document. Without this, cells from the previously opened
                // file (and the start-up sample rows) survive underneath the newly loaded data.
                _model.Clear();
                result = await SheetImporter.ImportAsync(stream, file.Name, _model);
            }
            finally
            {
                _model.EndBulkUpdate();
                _suppressDirty = false;
            }

            _currentFile = file;
            _currentFileName = file.Name;
            _sheetName = result.SheetName ?? "Sheet1";
            _loadedWorksheetCount = result.WorksheetCount;
            _isDirty = false;

            SheetGrid.ResetView();
            UpdateScrollBars();
            UpdateActiveCellUi();
            UpdateEditorPosition();
            UpdateTitle();

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

    // ---- Dirty state and document lifecycle ---------------------------------------------------

    private void OnModelChanged(object? sender, EventArgs e)
    {
        if (_suppressDirty || _isDirty)
        {
            return;
        }

        _isDirty = true;
        UpdateTitle();
    }

    private void UpdateTitle()
    {
        var name = _currentFileName ?? "Untitled";
        Title = _isDirty ? $"PGSheets - {name}*" : $"PGSheets - {name}";
    }

    /// <summary>Prompts to save pending edits before an action that would discard them.</summary>
    private async Task<bool> ConfirmUnsavedChangesAsync(string action)
    {
        if (!_isDirty)
        {
            return true;
        }

        var choice = await SavePrompt.ShowAsync(this, $"Save changes before {action}?");

        return choice switch
        {
            SavePromptResult.Save => await SaveAsync(),
            SavePromptResult.Discard => true,
            _ => false,
        };
    }

    private async Task NewDocumentAsync()
    {
        if (!await ConfirmUnsavedChangesAsync("starting a new spreadsheet"))
        {
            return;
        }

        _model.Clear();

        _currentFile = null;
        _currentFileName = null;
        _sheetName = "Sheet1";
        _loadedWorksheetCount = 1;

        // Clear() raises Changed, so the flag is reset after it rather than before.
        _isDirty = false;

        SheetGrid.ResetView();
        UpdateScrollBars();
        UpdateActiveCellUi();
        UpdateTitle();

        StatusText.Text = "New spreadsheet";
    }

    protected override void OnClosing(WindowClosingEventArgs e)
    {
        if (_forceClose || !_isDirty)
        {
            base.OnClosing(e);
            return;
        }

        // Asking is asynchronous, so the close is cancelled and re-issued once the user answers.
        e.Cancel = true;
        _ = ConfirmCloseAsync();
    }

    private async Task ConfirmCloseAsync()
    {
        if (!await ConfirmUnsavedChangesAsync("closing"))
        {
            return;
        }

        _forceClose = true;
        Close();
    }

    // ---- Undo / redo ---------------------------------------------------------------------------

    private void OnUndoClicked(object? sender, RoutedEventArgs e) => Undo();

    private void OnRedoClicked(object? sender, RoutedEventArgs e) => Redo();

    private void Undo()
    {
        CancelEdit();

        if (!_model.Undo.Undo())
        {
            StatusText.Text = "Nothing to undo";
            return;
        }

        UpdateActiveCellUi();
        StatusText.Text = "Undo";
    }

    private void Redo()
    {
        CancelEdit();

        if (!_model.Undo.Redo())
        {
            StatusText.Text = "Nothing to redo";
            return;
        }

        UpdateActiveCellUi();
        StatusText.Text = "Redo";
    }

    private void UpdateUndoState()
    {
        UndoMenuItem.IsEnabled = _model.Undo.CanUndo;
        RedoMenuItem.IsEnabled = _model.Undo.CanRedo;
    }

    // ---- Clipboard ------------------------------------------------------------------------------

    private async void OnCopyClicked(object? sender, RoutedEventArgs e) => await CopySelectionAsync();

    private async void OnCutClicked(object? sender, RoutedEventArgs e) => await CutSelectionAsync();

    private async void OnPasteClicked(object? sender, RoutedEventArgs e) => await PasteAsync();

    private void OnClearClicked(object? sender, RoutedEventArgs e)
    {
        SheetGrid.ClearSelection();
        UpdateActiveCellUi();
        StatusText.Text = "Cleared";
    }

    private void OnSelectAllClicked(object? sender, RoutedEventArgs e)
    {
        SheetGrid.SelectUsedRange();
        SheetGrid.Focus();
    }

    // ---- Number formats --------------------------------------------------------------------------

    /// <summary>
    /// The formats offered by the Sheet menu. The label is the format code itself, which is what a
    /// spreadsheet shows too; the empty code is General.
    /// </summary>
    private static readonly (string Label, string? Code)[] NumberFormats =
    {
        ("General", null),
        ("0", "0"),
        ("0.00", "0.00"),
        ("#,##0", "#,##0"),
        ("#,##0.00", "#,##0.00"),
        ("0%", "0%"),
        ("0.0%", "0.0%"),
        ("$#,##0.00", "$#,##0.00"),
        ("0000", "0000"),
        ("yyyy-mm-dd", "yyyy-mm-dd"),
        ("yyyy-mm-dd hh:mm", "yyyy-mm-dd hh:mm"),
        ("hh:mm:ss", "hh:mm:ss"),
    };

    private void OnNumberFormatClicked(object? sender, RoutedEventArgs e)
    {
        if (sender is not MenuItem { Header: string label })
        {
            return;
        }

        var match = NumberFormats.FirstOrDefault(format => format.Label == label);
        ApplyNumberFormat(match.Code);
    }

    /// <summary>
    /// Applies a number format to the selection as one undo step. Only cells that already exist are
    /// touched: formatting a whole selected column would otherwise create a cell for every row of
    /// the sheet, which a sparse model must not do. A value typed into a formatted cell keeps its
    /// format, so the usual intent — format the column, then fill it in — still works.
    /// </summary>
    private void ApplyNumberFormat(string? code)
    {
        var selection = SheetGrid.Selection;
        var touched = 0;

        _model.BeginBulkUpdate();

        try
        {
            // Snapshot the keys: setting a format can add or remove cells.
            foreach (var (row, column) in _model.PopulatedCells.ToList())
            {
                if (row < selection.Row1 || row > selection.Row2 ||
                    column < selection.Column1 || column > selection.Column2)
                {
                    continue;
                }

                _model.SetNumberFormat(row, column, code);
                touched++;
            }
        }
        finally
        {
            _model.EndBulkUpdate();
        }

        SheetGrid.Focus();

        StatusText.Text = code is null
            ? $"Cleared the number format on {touched} cell(s) in {selection}"
            : $"Applied \"{code}\" to {touched} cell(s) in {selection}";
    }

    // ---- Inserting and deleting rows and columns -------------------------------------------------

    private void OnInsertRowsClicked(object? sender, RoutedEventArgs e) =>
        ApplyStructuralEdit(insert: true, rows: true);

    private void OnInsertColumnsClicked(object? sender, RoutedEventArgs e) =>
        ApplyStructuralEdit(insert: true, rows: false);

    private void OnDeleteRowsClicked(object? sender, RoutedEventArgs e) =>
        ApplyStructuralEdit(insert: false, rows: true);

    private void OnDeleteColumnsClicked(object? sender, RoutedEventArgs e) =>
        ApplyStructuralEdit(insert: false, rows: false);

    /// <summary>
    /// Inserts or removes as many rows or columns as the selection spans, starting at its first row
    /// or column. Every formula in the sheet is adjusted, and the whole thing is one undo step.
    /// </summary>
    private void ApplyStructuralEdit(bool insert, bool rows)
    {
        CancelEdit();

        var selection = SheetGrid.Selection;
        var at = rows ? selection.Row1 : selection.Column1;
        var count = rows ? selection.RowCount : selection.ColumnCount;

        var applied = rows
            ? insert ? _model.InsertRows(at, count) : _model.DeleteRows(at, count)
            : insert ? _model.InsertColumns(at, count) : _model.DeleteColumns(at, count);

        if (!applied)
        {
            StatusText.Text = "That would push data off the sheet";
            return;
        }

        // Leave the same block selected, now covering the fresh rows or columns.
        SheetGrid.SelectRange(rows
            ? new CellRange(at, selection.Column1, at + count - 1, selection.Column2)
            : new CellRange(selection.Row1, at, selection.Row2, at + count - 1));

        SheetGrid.Focus();
        UpdateActiveCellUi();

        var what = rows ? "row(s)" : "column(s)";
        StatusText.Text = insert ? $"Inserted {count} {what}" : $"Deleted {count} {what}";
    }

    private async Task CopySelectionAsync()
    {
        var range = SheetGrid.Selection;
        var text = BuildClipboardText();

        _copiedRange = range;
        _copiedText = text;

        await SetClipboardTextAsync(text);
        StatusText.Text = $"Copied {range}";
    }

    private async Task CutSelectionAsync()
    {
        var range = SheetGrid.Selection;
        var text = BuildClipboardText();

        _copiedRange = range;
        _copiedText = text;

        await SetClipboardTextAsync(text);
        SheetGrid.ClearSelection();
        UpdateActiveCellUi();

        StatusText.Text = $"Cut {range}";
    }

    /// <summary>
    /// Tab-separated <em>raw input</em>, which is the same flavour Excel puts on the clipboard for a
    /// range. Raw input means formulas survive a copy/paste within this app.
    /// </summary>
    private string BuildClipboardText() =>
        // Reusing the delimited writer keeps one implementation of quoting and escaping.
        CsvExporter.Build(
            _model,
            new ExportOptions { Range = SheetGrid.Selection },
            '\t',
            new ExportResult());

    private async Task SetClipboardTextAsync(string text)
    {
        var clipboard = Clipboard;

        if (clipboard is null)
        {
            StatusText.Text = "The clipboard is unavailable";
            return;
        }

        await clipboard.SetTextAsync(text);
    }

    private async Task PasteAsync()
    {
        var clipboard = Clipboard;

        if (clipboard is null)
        {
            return;
        }

        var text = await clipboard.TryGetTextAsync();

        if (string.IsNullOrEmpty(text))
        {
            StatusText.Text = "Clipboard is empty";
            return;
        }

        // The delimited reader already handles quoted fields and delimiter detection.
        var block = new CellBuffer();
        CsvImporter.Parse(text, block);

        if (!block.TryGetBounds(out var source))
        {
            return;
        }

        var target = SheetGrid.Selection;

        // Relative references shift only when the text being pasted is the block this app copied,
        // because only then is its origin known. Text from another application keeps its references.
        var rowDelta = 0;
        var columnDelta = 0;

        if (_copiedRange is { } origin && string.Equals(text, _copiedText, StringComparison.Ordinal))
        {
            (rowDelta, columnDelta) = SheetPaste.ShiftFor(origin, target);
        }

        var (written, covered) = SheetPaste.Apply(block, _model, target, rowDelta, columnDelta);

        // Leave the pasted block selected, the way a spreadsheet does.
        SheetGrid.SelectRange(covered);
        UpdateActiveCellUi();
        StatusText.Text = $"Pasted {written} cell(s)";
    }

    // ---- Saving -------------------------------------------------------------------------------

    /// <summary>
    /// Saves to the current file, falling back to Save As when there is no file yet or when the
    /// current name is a format this app cannot write (an .xlsm workbook, for example).
    /// </summary>
    private async Task<bool> SaveAsync()
    {
        // Saving writes one worksheet, so writing back over a file that held more would destroy the
        // sheets that were never loaded. Send it to Save As instead, which is what an .xlsm save
        // already does: a document this app cannot fully write is never overwritten in place.
        if (!SheetExporter.CanSaveInPlace(_loadedWorksheetCount))
        {
            StatusText.Text =
                $"'{_currentFileName}' has {_loadedWorksheetCount} worksheets, and this app writes " +
                "one - choose a file to save it to.";

            return await SaveAsAsync();
        }

        if (_currentFile is null || _currentFileName is null || !SheetExporter.CanExport(_currentFileName))
        {
            return await SaveAsAsync();
        }

        return await WriteAsync(_currentFile, _currentFileName);
    }

    private async Task<bool> SaveAsAsync()
    {
        var file = await StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Save spreadsheet",
            SuggestedFileName = Path.GetFileNameWithoutExtension(_currentFileName ?? "Book1"),
            DefaultExtension = SheetExporter.DefaultExtension.TrimStart('.'),
            ShowOverwritePrompt = true,
            FileTypeChoices = new[]
            {
                new FilePickerFileType("Excel workbook") { Patterns = new[] { "*.xlsx" } },
                new FilePickerFileType("Delimited text") { Patterns = new[] { "*.csv", "*.tsv" } },
            },
        });

        if (file is null)
        {
            return false;
        }

        // The picker can return a bare name when the chosen type implies the extension.
        var name = SheetExporter.CanExport(file.Name) ? file.Name : file.Name + SheetExporter.DefaultExtension;

        return await WriteAsync(file, name);
    }

    private async Task<bool> WriteAsync(IStorageFile file, string fileName)
    {
        var options = new ExportOptions
        {
            SheetName = _sheetName,
            ExportValues = false,
        };

        try
        {
            var localPath = file.TryGetLocalPath();

            ExportResult result;

            if (localPath is not null)
            {
                // A real path truncates reliably, unlike a platform write handle.
                await using var stream = File.Create(localPath);
                result = await SheetExporter.ExportAsync(stream, fileName, _model, options);
            }
            else
            {
                await using var stream = await file.OpenWriteAsync();
                result = await SheetExporter.ExportAsync(stream, fileName, _model, options);
            }

            // Say what a single-worksheet save left behind, while the count still says how many
            // worksheets the file that was opened had. Losing sheets has to be stated, not
            // discovered later.
            if (SheetExporter.DescribeLostWorksheets(_loadedWorksheetCount, _sheetName) is { } lost)
            {
                result.AddWarning(lost);
            }

            _currentFile = file;
            _currentFileName = fileName;
            _isDirty = false;

            // The file on disk now holds exactly one worksheet, so later saves can be in place.
            _loadedWorksheetCount = 1;

            UpdateTitle();

            var summary = $"{fileName} - {result.Rows} x {result.Columns} written ({result.CellsWritten} non-empty)";
            StatusText.Text = result.Warnings.Count > 0 ? $"{summary} - {result.Warnings.Count} warning(s)" : summary;

            foreach (var warning in result.Warnings)
            {
                System.Diagnostics.Debug.WriteLine(warning);
            }

            return true;
        }
        catch (Exception ex)
        {
            StatusText.Text = $"Save failed: {ex.Message}";
            return false;
        }
    }
}
