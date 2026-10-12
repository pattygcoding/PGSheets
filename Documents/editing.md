# Editing

## Typing into a cell

There is exactly one text box for the whole sheet. When you start typing it is moved over the cell
you are editing, so a cell is either showing its value or being edited — never both, and never with
a control per cell.

| To | Do |
|---|---|
| Replace a cell's content | Select it and type. The character you type replaces what was there. |
| Change a cell's content | **F2**, or double-click the cell. |
| Keep the value but move on | **Enter** (down) or **Tab** (right). |
| Throw the edit away | **Escape**. |

A cell stores its **raw input** exactly as typed. `North` is text, `1200` is a number, `TRUE` is a
boolean, and `=B2+C2` is a formula that is stored as the text `=B2+C2`. Nothing rewrites what you
typed on the way in, and saving writes it back verbatim — which is why a formula stays a formula.

## Clearing

**Delete** or **Backspace** clears every populated cell in the selection; **Edit → Clear Contents**
does the same. Only cells that exist are touched, so clearing a whole column of a mostly empty sheet
is fast.

Clearing a cell removes it — except for its number format and its style. A cell you have formatted
keeps that format and style after the value is gone, ready for whatever you type next. That is
deliberate: it is what makes "format the column, then fill it in" work.

## Formatting a cell

The toolbar above the formula bar sets how the selection **looks**, without touching what it holds.

| Button | Effect |
|---|---|
| **B** / *I* / U | Bold, italic, underline |
| Left / Center / Right | Horizontal alignment |
| Wrap | Wraps the text to the column's width |
| Colour | The text colour, and **Default** to clear it |

Two things follow the way a spreadsheet behaves:

- The buttons show the **active cell's** state, and a click sets the whole selection to match what the
  active cell is *not* — so from a mixed selection one click makes everything bold, or everything plain.
- With **no explicit alignment**, the active cell's own content decides: numbers go right, text goes
  left, booleans are centred. That is Excel's "General" alignment, and clicking Left/Center/Right
  overrides it.

A style travels with its cell: it survives an undo, follows the cell through an inserted or deleted row
or column, and is written into a `.xlsx` file and read back. What it does **not** do is fill a wide
selection with empty cells — only cells that already exist are styled, exactly as with a number format.

Wrap has one caveat: rows are a fixed height, so a wrapped cell shows the lines that fit and clips the
rest rather than pushing the row taller.

## Undo and redo

**Ctrl+Z** and **Ctrl+Y** move through the history one step at a time. A step is whatever you would
think of as one action:

- typing a value into a cell,
- pasting a block,
- clearing a range, however many cells it covered,
- applying a number format or a style to a selection,
- inserting or deleting rows or columns.

The history holds the last 200 steps, lives in memory only, and is **not written to the file** —
reopening a document starts a fresh history.

## Cut, copy and paste

The clipboard carries **tab-separated text**, which is exactly the flavour a spreadsheet puts on the
clipboard for a range. That is why a block of cells moves between PGSheets and another spreadsheet in
either direction with no file format involved.

| Gesture | Does |
|---|---|
| **Ctrl+C** | Copies the selection as text. Formulas are copied as formulas. |
| **Ctrl+X** | Copies, then clears the source cells. |
| **Ctrl+V** | Writes the clipboard at the selection. |

Paste fills what you have selected:

- with a **single cell** selected, the copied block is tiled to cover the selection;
- with a **block** selected, the block it came from is repeated to cover it;
- a pasted value containing tabs or newlines survives because the text is quoted as CSV is.

### What happens to references

Copying a formula and pasting it somewhere else **shifts its relative references**, exactly as a
spreadsheet does:

| Copied formula | Pasted three rows down | Result |
|---|---|---|
| `=A1` | `=A4` | The reference follows the move |
| `=$A$1` | `=$A$1` | Anchored parts stay put |
| `=A$1` | `=A$1` | The row is anchored, the column is not |
| `=SUM(A1:B2)` | `=SUM(A4:B5)` | Both corners move |

Tabs and newlines in the copied text are escaped, so a value like `with<tab>tab` survives the round
trip through the clipboard.

**Pasting text from another application inserts formulas verbatim.** A shift needs to know where the
text was copied *from*, and the clipboard does not carry that: a block that arrived from elsewhere
has no origin, so its references are left exactly as written. Copying and pasting *within* PGSheets
does shift them, because the app remembers the region it copied.

## Inserting and deleting rows and columns

Under **Sheet**:

| Command | Effect |
|---|---|
| Insert Rows / Insert Columns | Inserts as many rows or columns as the selection spans, starting at its first row or column |
| Delete Rows / Delete Columns | Removes that many, pulling later cells up or left |

Both are one undo step and **rewrite every formula in the sheet** so that references keep pointing at
the same data:

| Formula | After inserting a row above it | After deleting the row it referred to |
|---|---|---|
| `=A5` | `=A6` | `=#REF!` |
| `=$A$5` | `=$A$6` | `=$A$6` if untouched, `=#REF!` if removed |
| `=SUM(A5:A8)` | `=SUM(A6:A9)` | `=SUM(#REF!)` if a corner was removed |

Anchors do **not** protect a reference here — inserting a row above `$A$5` still moves it to `$A$6`,
which is what a spreadsheet does. Anchors only survive a copy.

Two things worth knowing:

- Inserting is **refused** — with a message in the status bar and nothing changed — if it would push
  a populated cell past the last row or column of the sheet. Losing data silently would be worse than
  refusing.
- A formula the parser cannot read is left exactly as it was, so its references will not follow the
  change. That is a deliberate choice: leaving it untouched is safer than rewriting a formula that
  was not understood.

Next: [formulas.md](formulas.md) to put the engine to work, or
[number-formats.md](number-formats.md) to change how values read.
