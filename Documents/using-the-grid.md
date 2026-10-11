# Using the grid

## The parts of the window

| Part | What it shows |
|---|---|
| **Menu bar** | File, Edit, Sheet. Every command is here, including ones without a key. |
| **Name box** | The active cell (`C4`) or, when a range is selected, the range (`B2:D5`). It is a display, not an input — you cannot type a reference into it. |
| **Formula bar** | The **raw input** of the active cell: what was typed, so `=SUM(B2:B4)` shows as `=SUM(B2:B4)` rather than as its result. Editing here and pressing **Enter** writes the cell. |
| **Column and row headers** | `A`, `B`, `C`… across the top, `1`, `2`, `3`… down the side. Clicking one selects that whole column or row. |
| **Grid** | The sheet itself. Every visible cell is painted in one pass, so it stays responsive on large sheets. |
| **Status bar** | Messages on the left; on the right, a Count / Sum / Average of the numbers you have selected. |
| **Scroll bars** | The sheet is 1,048,576 rows by 16,384 columns, so both bars span that whole extent. |

## Moving the active cell

The active cell is the one with the thick border; the cell reference shown in the name box is the
one the formula bar reads and writes.

| Key | Moves |
|---|---|
| **Arrow keys** | One cell |
| **Enter** | Down one row |
| **Tab** / **Shift+Tab** | Right / left one column |
| **Page Up** / **Page Down** | A screenful up / down |
| **Home** / **End** | To the first / last column of the current row |
| **Ctrl+Home** | To `A1` |
| **Ctrl+End** | To the last cell of the sheet |

Holding **Shift** with any of the movement keys *extends the selection* instead of moving the active
cell, so **Shift+Down** grows a selected block downwards.

## Selecting a range

| Gesture | Selects |
|---|---|
| Click a cell | That cell |
| Drag from a cell | The block you drag over, in any direction |
| **Shift+click** | From the active cell to the cell you click |
| **Shift+arrow keys** | The block, one cell at a time |
| Click a **column header** | That whole column |
| Click a **row header** | That whole row |
| **Ctrl+A** | The smallest block containing every populated cell |

A selection is an **anchor plus an active cell**, which is why it can be dragged in any direction and
why a plain click collapses it back to a single cell.

**Ctrl+A selects the used range, not the entire sheet.** On the sample sheet it selects `A1:D8`, the
block that holds everything present. That keeps the common intent — "select my data" — from
selecting 17 billion empty cells. There is no shortcut for selecting the whole sheet; use the
column and row headers instead.

## The status bar summary

While more than one cell is selected, the right-hand side of the status bar reports on the selection:

| Situation | Shows |
|---|---|
| Text and blanks only | `42 cells` (how many cells the selection covers) |
| Numbers present | `Count: 6    Sum: 7630    Average: 1271.66…` |
| A single cell | Nothing |
| A workbook with more than 20,000 populated cells | `120,000 selected` — the scan is skipped so it does not run on every step of a drag |

Count is the number of *numbers*, so text cells in the range are not counted.

## Scrolling

- **Mouse wheel** scrolls three rows, or three columns with a horizontal wheel or trackpad gesture.
- **Scroll bars** drag, click the trough, or use the arrows on the ends.
- Clicking a cell that is partly off-screen does not scroll, and the selection is not clipped by the
  viewport — a range may extend beyond what you can see.

## An empty sheet and a populated one

The grid always shows the same extent; what changes is whether anything is in it. Only populated
cells are stored, so an untitled sheet costs almost nothing, and a cell becomes populated when you
type into it and unpopulated again when you clear it — unless it carries a number format, which is
worth knowing about: a cell with a format and no value still exists, and remembers the format for
whatever you type next.

Next: [editing.md](editing.md) for undo, the clipboard and structural changes, or
[keyboard-shortcuts.md](keyboard-shortcuts.md) for every key in one table.
