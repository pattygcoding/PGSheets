# Keyboard shortcuts

Everything below works from the grid. A cell editor, the formula bar and a dialog take their own
keys while they have focus — those are listed separately.

## Files

| Key | Command |
|---|---|
| **Ctrl+N** | New spreadsheet |
| **Ctrl+O** | Open… |
| **Ctrl+S** | Save (asks for a name the first time) |
| **Ctrl+Shift+S** | Save As… — `.xlsx`, `.csv` or `.tsv` |

## Editing

| Key | Command |
|---|---|
| **Ctrl+Z** | Undo |
| **Ctrl+Y** | Redo |
| **Ctrl+X** | Cut the selection |
| **Ctrl+C** | Copy the selection |
| **Ctrl+V** | Paste at the selection |
| **Ctrl+A** | Select the used range |
| **Delete** or **Backspace** | Clear every populated cell in the selection |
| **F2** | Edit the active cell's existing content |
| *any printable character* | Start an edit, replacing the cell's content with what you type |

**Edit → Clear Contents** does the same as **Delete**. **Edit → Select All** does the same as
**Ctrl+A**.

## Moving and selecting

| Key | Moves / selects |
|---|---|
| **Arrows** | One cell |
| **Shift+Arrows** | Extends the selection one cell |
| **Enter** | Down one row |
| **Tab** / **Shift+Tab** | Right / left one column |
| **Page Up** / **Page Down** | A screenful up / down |
| **Home** / **End** | First / last column of the row |
| **Ctrl+Home** | `A1` |
| **Ctrl+End** | The last cell of the sheet |
| **Shift** + any of the above | Extends the selection instead of moving |

## Mouse

| Gesture | Does |
|---|---|
| Click a cell | Move the active cell there |
| Drag | Select a block |
| **Shift+click** | Extend the selection to the cell you click |
| Double-click | Edit that cell |
| Click a column or row header | Select the whole column or row |
| Wheel | Scroll three rows (or columns) |

## While editing a cell

| Key | Does |
|---|---|
| **Enter** | Writes the value, then moves down |
| **Tab** | Writes the value, then moves right |
| **Escape** | Abandons the edit; nothing is written |

## In the formula bar

| Key | Does |
|---|---|
| **Enter** | Writes the value into the active cell |
| **Escape** | Reverts the bar to the cell's stored content |

## Notes

- **Ctrl+A is the grid's**, and it selects the used range rather than the whole sheet.
- Every other **Ctrl** combination belongs to the window, so `Ctrl+S` saves wherever the grid has
  focus.
- **Undo** and **Redo** are greyed out when there is nothing to undo or redo. Cut, Copy and Paste
  stay available and always act on the current selection; if the system clipboard cannot be reached
  they say so in the status bar.
- **Ctrl+C copies formulas as formulas**, not as their results — the same as a spreadsheet. Use the
  copy/paste round trip to duplicate a formula across a column and watch its references follow.

Next: [files.md](files.md) for what opening and saving actually does to your data.
