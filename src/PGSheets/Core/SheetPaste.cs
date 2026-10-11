using PGSheets.Formulas;

namespace PGSheets.Core;

/// <summary>
/// Writes a rectangular block of pasted cells into the sheet, repeating it to cover the target the
/// way a spreadsheet paste does.
/// </summary>
public static class SheetPaste
{
    /// <summary>
    /// Applies <paramref name="block"/> to the target selection. A single copied cell fills the
    /// whole selection; a larger block is tiled enough times to cover it. Every write happens inside
    /// one bulk update, so a paste is a single undo step.
    /// </summary>
    /// <param name="rowDelta">
    /// How far the block has travelled, used to shift relative references. The clipboard carries
    /// only text, so the caller supplies this from the region it copied.
    /// </param>
    /// <returns>The number of cells written, and the range the paste actually covers.</returns>
    public static (int Written, CellRange Covered) Apply(
        CellBuffer block,
        SpreadsheetModel model,
        CellRange target,
        int rowDelta = 0,
        int columnDelta = 0)
    {
        ArgumentNullException.ThrowIfNull(block);
        ArgumentNullException.ThrowIfNull(model);

        if (!block.TryGetBounds(out var source))
        {
            return (0, target);
        }

        var copiesDown = Math.Max(1, target.RowCount / source.RowCount);
        var copiesRight = Math.Max(1, target.ColumnCount / source.ColumnCount);

        var written = 0;

        model.BeginBulkUpdate();

        try
        {
            for (var down = 0; down < copiesDown; down++)
            {
                for (var right = 0; right < copiesRight; right++)
                {
                    foreach (var (row, column, value) in block.Cells)
                    {
                        var destinationRow = target.Row1 + down * source.RowCount + (row - source.Row1);
                        var destinationColumn = target.Column1 + right * source.ColumnCount + (column - source.Column1);

                        if (destinationRow < 0 ||
                            destinationRow >= SheetBounds.MaxRows ||
                            destinationColumn < 0 ||
                            destinationColumn >= SheetBounds.MaxColumns)
                        {
                            continue;
                        }

                        model.SetRawInput(
                            destinationRow,
                            destinationColumn,
                            Translate(value, rowDelta, columnDelta));

                        written++;
                    }
                }
            }
        }
        finally
        {
            model.EndBulkUpdate();
        }

        var covered = new CellRange(
            target.Row1,
            target.Column1,
            Math.Min(SheetBounds.MaxRows - 1, target.Row1 + copiesDown * source.RowCount - 1),
            Math.Min(SheetBounds.MaxColumns - 1, target.Column1 + copiesRight * source.ColumnCount - 1));

        return (written, covered);
    }

    /// <summary>
    /// How far a block has moved when it is pasted: the distance between the two top-left corners.
    /// A spreadsheet applies exactly this shift to relative references and none to anchored ones.
    /// </summary>
    public static (int RowDelta, int ColumnDelta) ShiftFor(CellRange copiedFrom, CellRange target) =>
        (target.Row1 - copiedFrom.Row1, target.Column1 - copiedFrom.Column1);

    /// <summary>
    /// A pasted formula keeps its meaning relative to where it lands: relative references shift by
    /// the distance moved and anchored ones stay put, which is what a spreadsheet does.
    /// </summary>
    private static string Translate(string raw, int rowDelta, int columnDelta) =>
        raw.Length > 0 && raw[0] == '='
            ? FormulaRewriter.Translate(raw, rowDelta, columnDelta)
            : raw;
}
