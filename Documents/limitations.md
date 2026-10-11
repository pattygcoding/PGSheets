# Limitations

This page is the honest list. Some of these are deliberate trade-offs with a reason; others are
simply not built yet. Where a limitation has a reason, the reason is given, because a documented
constraint is more useful than a surprise.

## Not built yet

- **Multiple worksheets.** Only the first sheet of a workbook is read, and only one is written. There
  are no sheet tabs. A workbook with more than one sheet is never saved over in place, so opening one
  and pressing **Ctrl+S** asks for a new file instead of destroying the sheets that were not loaded.
- **Cell formatting.** Fonts, colours, borders, alignment, merged cells and column widths are not
  read, shown or written. The **number format is the one exception**, because a number format changes
  what a value *means*, not just how it looks.
- **Column and row resizing.** Every column is 100 pixels and every row 24.
- **Find and replace.**
- **Sorting and filtering.**
- **Charts, images, comments, named ranges, data validation, conditional formatting.**
- **Printing.**

## Where the engine differs from a full spreadsheet

- **The function library covers the common ground, not the long tail** — 83 functions. There is no
  `XLOOKUP`, `LOOKUP`, `TEXT`, `SEARCH` or `TEXTBEFORE`, and no array or spill semantics:
  `SUMPRODUCT` needs equally sized ranges. A function that is missing evaluates to `#NAME?`, and
  importing a file lists the missing names as warnings.
- **No `DATE`, `TODAY` or `NOW`.** A formula cannot give its own result a number format, so a
  produced date would display as a bare serial unless the cell had already been date-formatted by
  hand. This app does not guess a format, so it does not offer the function.
- **Pasting text from another application does not shift references.** A shift needs to know where the
  text was copied from, and the clipboard does not carry that. Copy and paste *within* PGSheets does
  shift them, because the app remembers the region it copied.
- **A formula the parser cannot read is never rewritten.** It is left exactly as it was, which is
  safer than guessing — but it means its references will not follow a copy or an inserted row.
- **Deleting a reference leaves `#REF!`,** and a range with one deleted corner collapses to a bare
  `#REF!` rather than to a partial range.
- **A lookup reports a miss as `#N/A`,** and nothing else produces that code: `IFNA` only ever catches
  a lookup, and `ISNA` is only ever true for one.
- **A date typed into a cell is not a date.** It stays the text you typed, so it displays and the date
  functions read it, but it is not a number and arithmetic on it will not work. A date *imported*
  from a file is a real serial.
- **The serial date system has one ambiguous day.** Excel counts 1900-02-29 as serial 60 although 1900
  was not a leap year. No such day exists in the framework, so serial 60 reads as 1900-03-01 — the
  same date as serial 61. Every real date converts both ways exactly; only that one value is
  ambiguous.

## File fidelity

- **One worksheet.** See above. The consequence worth repeating: opening a multi-sheet workbook and
  saving sends you to Save As.
- **`.xlsm` files cannot be saved** — a VBA project cannot be produced, so **Ctrl+S** falls through to
  Save As as `.xlsx`.
- **CSV loses what it cannot express.** A formula is written as its text, a date as text, and number
  formats are not carried at all. A string that looks like a number will come back as a number —
  leading zeros in such a field are lost. Exports report these as warnings.
- **Error results are not cached** when saving, on purpose: another application should recalculate
  rather than inherit `#NAME?` for a function this engine lacks.
- **A numeric-looking literal is normalised.** Typing `007` stores the number 7, as a spreadsheet
  does; use the `0000` number format to display it with leading zeros.
- **Formats this app cannot render still round-trip.** Scientific notation, fractions and elapsed
  time fall back to the plain rendering on screen, but the format code itself is read and written
  back unchanged, so a file does not lose them.
- **`?` padding is not reproduced,** so a `?` placeholder does not align decimal points.
- **A cell's style carries more than a number format,** and only the number format is taken: a cell
  formatted as bold but not as a number comes back plain.
- **Another application accepting our files is verified by hand.** The automated tests prove the
  output is structurally correct and that this app can read it back; whether Excel or LibreOffice
  opens it without complaint is a manual check.

## Scale and performance

These are the points where behaviour is deliberately capped rather than unbounded:

| Limit | Value | Why |
|---|---|---|
| Undo history | 200 steps, in memory only | Not written to the file; a fixed ceiling keeps memory predictable |
| Structural edits | O(populated cells) | Inserting a row rebuilds the sheet's cell map rather than shifting in place. Fine at current sizes; a large sheet would want a per-row offset map |
| Selection summary | Skipped above 20,000 populated cells | The scan would otherwise run on every step of a drag |
| Formula text cache | 2,048 entries | Drawn text recurs while scrolling, so it is cached — and bounded so it cannot grow without limit |
| Repeated text | Refused beyond a cell's worth | `REPT` reports `#VALUE!` rather than building a string no cell could hold |
| Dependency chains | Recursive | A chain thousands of cells deep could exhaust the stack |

## Things that look like bugs but are decisions

- **Clearing a formatted cell leaves it in the sheet.** The format is remembered for whatever you
  type next, which is what makes "format the column, then fill it in" work.
- **Ctrl+A selects the used range, not the whole sheet.** Selecting 17 billion empty cells is almost
  never what you meant.
- **Saving a multi-sheet workbook asks for a new file** rather than overwriting it. Refusing to
  destroy data you cannot see beats a quiet success.
- **Inserting rows is refused** when it would push data past the last row of the sheet. Losing cells
  silently would be worse than a message.
- **An unreadable formula is left alone.** Not rewriting what you do not understand is the safer
  failure.
- **The grid is not the data.** A single control paints every visible cell in one pass, so scrolling
  a large sheet stays smooth; nothing is created per cell.

Next: [development.md](development.md) if you want to work on the code itself.
