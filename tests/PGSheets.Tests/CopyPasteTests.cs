using PGSheets.Core;
using PGSheets.Export;
using PGSheets.Import;

namespace PGSheets.Tests;

/// <summary>
/// The whole copy/paste chain minus the clipboard itself: a range is serialised to tab-separated
/// text, parsed back into a block, and written at a new location. This is the path Ctrl+C then
/// Ctrl+V takes through the application.
/// </summary>
public class CopyPasteTests
{
    private static string Copy(SpreadsheetModel model, CellRange range, bool values = false) =>
        CsvExporter.Build(
            model,
            new ExportOptions { Range = range, ExportValues = values },
            '\t',
            new ExportResult());

    /// <summary>
    /// Mirrors the host: the shift comes from the region that was copied, because the clipboard text
    /// alone does not say where it came from.
    /// </summary>
    private static (int Written, CellRange Covered) Paste(
        SpreadsheetModel model,
        string text,
        CellRange copiedFrom,
        CellRange target)
    {
        var block = new CellBuffer();
        CsvImporter.Parse(text, block);

        var (rowDelta, columnDelta) = SheetPaste.ShiftFor(copiedFrom, target);

        return SheetPaste.Apply(block, model, target, rowDelta, columnDelta);
    }

    [Fact]
    public void A_block_survives_being_copied_and_pasted()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "one");
        Sheet.Set(model, "B1", "two");
        Sheet.Set(model, "A2", "three");
        Sheet.Set(model, "B2", "=A1");

        var copied = new CellRange(0, 0, 1, 1);
        var (written, covered) = Paste(model, Copy(model, copied), copied, new CellRange(4, 0, 4, 0));

        Assert.Equal(4, written);
        Assert.Equal(new CellRange(4, 0, 5, 1), covered);
        Assert.Equal("one", Sheet.Display(model, "A5"));
        Assert.Equal("two", Sheet.Display(model, "B5"));
        Assert.Equal("three", Sheet.Display(model, "A6"));

        // B2's "=A1" moved down four rows, so it now points at A5.
        Assert.Equal("=A5", model.GetRawInput(5, 1));
    }

    [Fact]
    public void A_pasted_formula_is_shifted_like_a_spreadsheet()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "top");
        Sheet.Set(model, "A6", "six");
        Sheet.Set(model, "B1", "=A1");

        var copied = new CellRange(0, 1, 0, 1);

        // Copy B1 five rows down to B6: the relative reference shifts with it.
        Paste(model, Copy(model, copied), copied, new CellRange(5, 1, 5, 1));

        Assert.Equal("=A6", model.GetRawInput(5, 1));
        Assert.Equal("six", Sheet.Display(model, "B6"));
    }

    [Fact]
    public void An_anchored_reference_does_not_shift_when_pasted()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "anchor target");
        Sheet.Set(model, "B1", "=$A$1");

        var copied = new CellRange(0, 1, 0, 1);

        Paste(model, Copy(model, copied), copied, new CellRange(4, 1, 4, 1));

        Assert.Equal("=$A$1", model.GetRawInput(4, 1));
        Assert.Equal("anchor target", Sheet.Display(model, "B5"));
    }

    [Fact]
    public void A_partly_anchored_reference_shifts_only_the_relative_part()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "target");
        Sheet.Set(model, "B1", "=A$1");

        var copied = new CellRange(0, 1, 0, 1);

        Paste(model, Copy(model, copied), copied, new CellRange(2, 2, 2, 2));

        // Moved down two rows and right one column: the anchored row stays, the column moves.
        Assert.Equal("=B$1", model.GetRawInput(2, 2));
        Assert.Equal("target", Sheet.Display(model, "C3"));
    }

    [Fact]
    public void Copying_a_single_cell_fills_the_selection()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "x");

        var copied = new CellRange(0, 0, 0, 0);

        var (written, covered) = Paste(model, Copy(model, copied), copied, new CellRange(2, 2, 4, 4));

        Assert.Equal(9, written);
        Assert.Equal(new CellRange(2, 2, 4, 4), covered);
        Assert.Equal("x", Sheet.Display(model, "E5"));
    }

    [Fact]
    public void Copying_values_writes_the_computed_result()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "2");
        Sheet.Set(model, "A2", "=A1*3");

        var copied = new CellRange(1, 0, 1, 0);

        Paste(model, Copy(model, copied, values: true), copied, new CellRange(1, 2, 1, 2));

        Assert.Equal("6", model.GetRawInput(1, 2));
        Assert.Equal("6", Sheet.Display(model, "C2"));
    }

    [Fact]
    public void Text_containing_delimiters_and_newlines_survives()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "with\ttab");
        Sheet.Set(model, "A2", "with\nnewline");
        Sheet.Set(model, "A3", "with \"quotes\"");

        var copied = new CellRange(0, 0, 2, 0);

        Paste(model, Copy(model, copied), copied, new CellRange(10, 0, 10, 0));

        Assert.Equal("with\ttab", model.GetRawInput(10, 0));
        Assert.Equal("with\nnewline", model.GetRawInput(11, 0));
        Assert.Equal("with \"quotes\"", model.GetRawInput(12, 0));
    }

    [Fact]
    public void Empty_cells_are_kept_as_blank_fields()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "left");
        Sheet.Set(model, "B1", "right");

        // Copy the 2x2 block: the bottom row exists only as blank fields.
        var copied = new CellRange(0, 0, 1, 1);

        Paste(model, Copy(model, copied), copied, new CellRange(4, 0, 4, 0));

        Assert.Equal("left", Sheet.Display(model, "A5"));
        Assert.Equal("right", Sheet.Display(model, "B5"));
        Assert.Equal(string.Empty, Sheet.Display(model, "A6"));
        Assert.Equal(string.Empty, Sheet.Display(model, "B6"));
    }

    [Fact]
    public void Text_with_no_known_origin_keeps_its_references()
    {
        // Text arriving from another application has no origin, so a spreadsheet has nothing to
        // shift by and the formula is inserted exactly as it was copied.
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "2");

        var block = new CellBuffer();
        CsvImporter.Parse("=A1", block);

        SheetPaste.Apply(block, model, new CellRange(3, 3, 3, 3));

        Assert.Equal("=A1", model.GetRawInput(3, 3));
        Assert.Equal("2", Sheet.Display(model, "D4"));
    }

    [Fact]
    public void Copying_the_same_range_twice_produces_identical_text()
    {
        var model = Sheet.CreateModel();
        Sheet.Set(model, "A1", "a");
        Sheet.Set(model, "B1", "1");

        var range = new CellRange(0, 0, 0, 1);

        Assert.Equal(Copy(model, range), Copy(model, range));
    }
}
