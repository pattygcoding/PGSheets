using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;
using Avalonia.Media.Immutable;
using PGSheets.Core;

namespace PGSheets.Controls;

/// <summary>
/// A virtualised spreadsheet surface. No child controls are created for cells: the entire grid is
/// painted in a single <see cref="Render"/> pass and only the cells inside the current viewport are
/// touched, so the cost per frame depends on the window size rather than on the sheet size.
/// </summary>
public sealed partial class SpreadsheetGridControl : Control
{
    // All drawing resources are immutable and shared. Nothing here is allocated per frame.
    private static readonly ImmutablePen GridPen = new(0xFFE3E3E3, 1);
    private static readonly ImmutablePen SelectionPen = new(0xFF1A73E8, 2);
    private static readonly ImmutableSolidColorBrush BackgroundBrush = new(0xFFFFFFFF);
    private static readonly ImmutableSolidColorBrush HeaderBrush = new(0xFFF5F5F5);
    private static readonly ImmutableSolidColorBrush HeaderActiveBrush = new(0xFFD7E7FB);
    private static readonly ImmutableSolidColorBrush SelectionFillBrush = new(0x141A73E8);
    private static readonly ImmutableSolidColorBrush TextBrush = new(0xFF202124);
    private static readonly ImmutableSolidColorBrush HeaderTextBrush = new(0xFF5F6368);
    private static readonly ImmutableSolidColorBrush ErrorTextBrush = new(0xFFC5221F);

    private const double CellPadding = 4d;
    private const int MaxTextCacheEntries = 2048;

    private readonly Dictionary<string, FormattedText> _cellTextCache = new(StringComparer.Ordinal);
    private readonly Dictionary<string, FormattedText> _headerTextCache = new(StringComparer.Ordinal);
    private readonly List<string> _columnLabels = new();

    private readonly Typeface _typeface = Typeface.Default;

    private double _scrollX;
    private double _scrollY;
    private int _activeRow;
    private int _activeColumn;

    private double _columnWidth = 100d;
    private double _rowHeight = 24d;
    private double _headerWidth = 52d;
    private double _headerHeight = 24d;
    private double _fontSize = 13d;
    private int _rowCount = SheetBounds.MaxRows;
    private int _columnCount = SheetBounds.MaxColumns;

    public SpreadsheetGridControl()
    {
        Focusable = true;
        ClipToBounds = true;
    }

    public static readonly StyledProperty<SpreadsheetModel?> ModelProperty =
        AvaloniaProperty.Register<SpreadsheetGridControl, SpreadsheetModel?>(nameof(Model));

    static SpreadsheetGridControl()
    {
        AffectsRender<SpreadsheetGridControl>(ModelProperty);
    }

    /// <summary>The workbook this control renders. Setting it resets the view to A1.</summary>
    public SpreadsheetModel? Model
    {
        get => GetValue(ModelProperty);
        set => SetValue(ModelProperty, value);
    }

    /// <summary>Width of a data column, in device independent pixels.</summary>
    public double ColumnWidth
    {
        get => _columnWidth;
        set => SetLayout(ref _columnWidth, value);
    }

    /// <summary>Height of a data row.</summary>
    public double RowHeight
    {
        get => _rowHeight;
        set => SetLayout(ref _rowHeight, value);
    }

    /// <summary>Width of the sticky row-header strip (row numbers).</summary>
    public double HeaderWidth
    {
        get => _headerWidth;
        set => SetLayout(ref _headerWidth, value);
    }

    /// <summary>Height of the sticky column-header strip (A, B, C...).</summary>
    public double HeaderHeight
    {
        get => _headerHeight;
        set => SetLayout(ref _headerHeight, value);
    }

    /// <summary>Font size used for cell contents.</summary>
    public double FontSize
    {
        get => _fontSize;
        set
        {
            if (value > 0 && Math.Abs(_fontSize - value) > double.Epsilon)
            {
                _fontSize = value;
                _cellTextCache.Clear();
                InvalidateVisual();
            }
        }
    }

    /// <summary>Number of virtual rows. Never materialised.</summary>
    public int RowCount
    {
        get => _rowCount;
        set
        {
            var clamped = Math.Clamp(value, 1, SheetBounds.MaxRows);
            if (clamped != _rowCount)
            {
                _rowCount = clamped;
                RaiseViewportChanged();
                InvalidateVisual();
            }
        }
    }

    /// <summary>Number of virtual columns. Never materialised.</summary>
    public int ColumnCount
    {
        get => _columnCount;
        set
        {
            var clamped = Math.Clamp(value, 1, SheetBounds.MaxColumns);
            if (clamped != _columnCount)
            {
                _columnCount = clamped;
                _columnLabels.Clear();
                RaiseViewportChanged();
                InvalidateVisual();
            }
        }
    }

    private void SetLayout(ref double field, double value)
    {
        if (value <= 0 || Math.Abs(field - value) <= double.Epsilon)
        {
            return;
        }

        field = value;
        ClampScroll();
        RaiseViewportChanged();
        InvalidateVisual();
    }
}
