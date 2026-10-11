# PGSheets

[![CI](https://github.com/pattygcoding/PGSheets/actions/workflows/ci.yml/badge.svg)](https://github.com/pattygcoding/PGSheets/actions/workflows/ci.yml)

A high-performance, cross-platform spreadsheet desktop application built with C# / .NET 8 and
Avalonia UI. It renders a virtualised grid with a single custom-drawn control, edits cells in place
with one reusable `TextBox`, evaluates formulas through a dependency-tracking engine, and opens,
edits and saves `.xlsx` and `.csv` files.

## Documentation

Full user documentation lives in [`Documents/`](Documents/):

| Page | Covers |
|---|---|
| [Getting started](Documents/getting-started.md) | Build, run, first edits, opening and saving |
| [Using the grid](Documents/using-the-grid.md) | Navigation, selection, the status-bar summary |
| [Editing](Documents/editing.md) | Undo, the clipboard, inserting and deleting rows and columns |
| [Formulas](Documents/formulas.md) | The language, references, operators, errors |
| [Function reference](Documents/functions.md) | All 83 functions with their arguments |
| [Number formats](Documents/number-formats.md) | Percentages, currency, dates and the format language |
| [Files](Documents/files.md) | What is read and written, and what round-trips |
| [Keyboard shortcuts](Documents/keyboard-shortcuts.md) | Every key in one table |
| [Limitations](Documents/limitations.md) | What is not built, and why |
| [Development](Documents/development.md) | Building, testing and the code layout |

The sections below are the short version; those pages are the detail.

## Highlights

- **No control-per-cell.** `SpreadsheetGridControl : Control` paints the whole sheet in one
  `Render(DrawingContext)` pass. Gridlines are pen primitives, text is cached `FormattedText`, and
  only the cells inside the viewport are ever touched — a 1,048,576 × 16,384 sheet costs the same
  per frame as a small one.
- **Sticky headers** for columns (`A`, `B`, `C`…) and rows (`1`, `2`, `3`…).
- **In-place editing** with a single floating `TextBox` that is repositioned over the active cell on
  double-click, `F2`, or when you start typing.
- **Range selection** by dragging, Shift+click, Shift+arrows or `Ctrl+A`, with an Excel-style
  Count / Sum / Average summary in the status bar.
- **Undo and redo** that collapse a paste, a cut or a range clear into a single step.
- **Clipboard as tab-separated text** — the same flavour Excel uses for a range, so a block of cells
  copies and pastes between PGSheets and Excel in either direction.
- **Anchored references** (`$A$1`). Relative parts shift when a formula is copied or pasted and
  anchored parts stay put, so a copied formula means what it says.
- **Insert and delete rows and columns**, with every formula in the sheet rewritten to follow — and
  `#REF!` where a referenced row or column was removed.
- **Number formats**, from `0.00` and `#,##0.00` to `0%`, `$#,##0.00` and `yyyy-mm-dd`. A format
  changes what a cell shows without touching what it holds, so `=A1+1` on a date still means the
  next day. They are read from a `.xlsx` style table and written back, and Sheet → Number format
  applies one to the selection; undo undoes it.
- **Formula engine** over a sparse model, with an extensible function registry, dependency tracking
  and circular-reference detection.
- **Import and export** of Excel workbooks (`.xlsx`) and delimited text (`.csv`, `.tsv`), including
  formulas. Saving writes raw input by default, so formulas survive a save/reload round trip.
- **Dirty-state tracking**: the title bar marks unsaved work and closing prompts to save.
- **Self-contained**: no third-party spreadsheet libraries. XLSX is read *and written* with
  `System.IO.Compression` and `System.Xml`; CSV has a hand-written RFC 4180 parser and writer. The
  app makes no network calls at runtime and requires no API keys.

## Requirements

- .NET 8 SDK
- Windows, Linux or macOS (Avalonia desktop backends)

## Build and run

```powershell
dotnet build PGSheets.sln -c Release
dotnet run   --project src/PGSheets
dotnet test  PGSheets.sln          # 789 tests, headless
```

Warnings are errors (`TreatWarningsAsErrors` in `Directory.Build.props`), so a clean build really
does mean zero warnings.

## Testing

789 xUnit tests run headless — no Avalonia initialisation, no window, no display required:

```powershell
dotnet test PGSheets.sln
dotnet test PGSheets.sln --collect:"XPlat Code Coverage"   # writes a Cobertura report
```

They cover the formula engine (precedence, ranges, aggregates, propagation, cycles, every built-in
function and its error branches), the sparse model, A1 addressing and value coercion, the CSV and
XLSX readers and writers, the OOXML packaging the writer produces, import → export → re-import
round-trip fidelity, range selection and paste geometry, undo grouping, the dependency graph, and the
grid's hit-testing and scroll maths. The XLSX fixtures are generated with the BCL, so the tests depend
on no third-party spreadsheet package.

Coverage is ~75% overall and ~94% across `Core`/`Formulas`/`Import`/`Export`/`Ooxml`. The rendering
path and the clipboard/undo wiring need a real window, so they are verified by launching the app
rather than by unit test.

CI runs restore → build → test on Ubuntu, Windows and macOS for every push and pull request.

## Using the grid

| Action | Result |
|---|---|
| Click a cell | Select it |
| Double-click / `F2` | Edit the cell's existing content |
| Type a character | Edit, seeded with that character |
| `Enter` / `Tab` | Commit and move down / right |
| `Escape` | Cancel the edit |
| Arrows, `Home`, `End`, `PgUp`, `PgDn` | Move the active cell (auto-scrolls into view) |
| `Delete` / `Backspace` | Clear the active cell |
| Mouse wheel | Scroll |
| Drag, Shift+click, Shift+arrows | Select a range |
| Click a row or column header | Select the whole row or column |
| `Ctrl+A` | Select the used range |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | Copy, cut and paste the selection as tab-separated text |
| `Ctrl+Z` / `Ctrl+Y` / `Ctrl+Shift+Z` | Undo and redo |
| `Delete` | Clear every cell in the selection |
| Sheet → Insert Rows / Columns | Insert as many rows or columns as the selection spans |
| Sheet → Delete Rows / Columns | Remove them, rewriting every reference in the sheet |
| Sheet → Number format | Apply `0.00`, `#,##0.00`, `0%`, `$#,##0.00`, `yyyy-mm-dd` … to the selection (General clears it) |
| `Ctrl+O` / File → Open | Import a `.xlsx` or `.csv` file |
| `Ctrl+S` / File → Save | Save (asks for a name the first time) |
| `Ctrl+Shift+S` | Save As — including `.csv` / `.tsv` |
| `Ctrl+N` / File → New | Empty the sheet |

The name box shows the active cell reference — or the range, such as `B2:D5` — and the formula bar
shows (and edits) that cell's raw input. While a range is selected, the status bar reports
Count / Sum / Average of its numbers, like a spreadsheet does.

Copy and paste use **tab-separated text**, which is exactly the flavour Excel puts on the clipboard
for a range. A block of cells therefore moves between PGSheets and Excel in either direction, with no
file format involved.

## Formulas

Supported today:

- Cell references and ranges: `A1`, `$B$7`, `$B7`, `A1:B10`, `$A$1:$B$2`. The anchors are kept and
  honoured: copying or pasting a formula shifts its relative parts and leaves anchored ones where
  they are, and inserting or deleting rows and columns moves every reference that follows.
- Operators: `+ - * / ^`, unary minus, parentheses, and comparisons `= <> < <= > >=`
- Errors: `#DIV/0!`, `#VALUE!`, `#REF!`, `#NAME?`, `#NUM!`, `#N/A`, `#CIRCULAR!`
- **83 functions**:
  - *Numbers* — `SUM` `PRODUCT` `ABS` `SIGN` `SQRT` `POWER` `EXP` `LN` `LOG` `LOG10` `PI` `INT`
    `TRUNC` `MOD` `ROUND` `ROUNDUP` `ROUNDDOWN` `CEILING` `FLOOR`
  - *Statistics* — `MIN` `MAX` `AVERAGE` `MEDIAN` `MODE` `COUNT` `COUNTA` `COUNTBLANK` `COUNTIF`
    `SUMIF` `AVERAGEIF` `LARGE` `SMALL` `RANK` `STDEV` `VAR` `SUMPRODUCT`
  - *Logic* — `IF` `IFS` `IFERROR` `IFNA` `AND` `OR` `XOR` `NOT` `SWITCH` `CHOOSE` `TRUE` `FALSE`
    `NA`
  - *Text* — `CONCAT` `CONCATENATE` `TEXTJOIN` `LEFT` `RIGHT` `MID` `LEN` `UPPER` `LOWER` `PROPER`
    `TRIM` `REPT` `SUBSTITUTE` `FIND` `EXACT` `VALUE` `CHAR` `CODE`
  - *Lookup* — `VLOOKUP` `HLOOKUP` `INDEX` `MATCH`
  - *Dates* — `YEAR` `MONTH` `DAY` `WEEKDAY`
  - *Type tests* — `ISBLANK` `ISNUMBER` `ISTEXT` `ISLOGICAL` `ISERROR` `ISNA` `ISEVEN` `ISODD`

An aggregate skips text and blanks inside a range rather than counting them as zero. `COUNTIF`,
`SUMIF` and `AVERAGEIF` accept criteria such as `">=10"`, `"<>closed"` and `"a*"`, with `*` and `?`
wildcards and `~` to escape them. A lookup that finds nothing is `#N/A`, which is what makes `IFERROR`
and `IFNA` worth having.

The date functions take either a serial number (`=YEAR(45356)` → 2024) or ISO text
(`=YEAR("2024-03-05")` → 2024). A date from a file arrives as a serial with its date format, so it
displays as a date and still behaves like the number it is.

Examples: `=SUM(A1:A5)`, `=A1 + B2`, `=IF(B2>10,"big","small")`, `=VLOOKUP($A1,$B$2:$C$20,2,FALSE)`,
`=COUNTIF(C2:C99,">0") - COUNTBLANK(D2:D99)`.

New functions are one line — see `BuiltinFunctions.Register` and the extension notes in
[AGENTS.md](AGENTS.md).

## Importing files

**CSV** is parsed with a quote-aware state machine: quoted fields, escaped quotes (`""`), embedded
newlines, CRLF/LF/CR line endings, UTF-8 BOM stripping and delimiter auto-detection
(`, ; tab |`).

**XLSX** is read directly from the Open Packaging zip: shared strings, inline strings, numbers,
booleans, cached results, formulas and number formats. Only the first worksheet is imported.

Number formats are read from the workbook's style table, so a date arrives as the serial the file
holds together with the format that turns it back into a date — which is why a date from a real
workbook sorts and calculates like one.

Formulas are re-evaluated by the built-in engine by default. Any function the engine does not
implement becomes `#NAME?` and is reported as a warning (the count appears in the status bar).
Set `XlsxImporter.ImportFormulas = false` to import cached values as literals instead.

## Saving files

Save writes **raw input** by default, so `=SUM(B2:B4)` is written as a formula and survives a
reload. `Save As` can write `.xlsx` (the default), `.csv` or `.tsv`.

Details worth knowing:

- Formula results that are *errors* are deliberately not cached, so another application recalculates
  them rather than inheriting `#NAME?` for a function this engine does not implement.
- xlsx output is a minimal but complete package: content types, package relationships, workbook,
  workbook relationships, a style table sized to the number formats actually used, and one worksheet
  using inline strings.
- Saving a `.xlsm` file falls through to `Save As` as `.xlsx` — macros cannot be written, and a
  `.xlsm` that silently dropped them would be a lie. The same rule covers extra worksheets: opening a
  workbook that holds more than one sheet sends Save to `Save As` instead of overwriting the
  original, because writing in place truncates the file and would destroy the sheets that were never
  loaded. This app never overwrites a document it cannot write back in full.
- One worksheet is written. Number formats are written with it; fonts, colours, borders, merged cells
  and column widths are not preserved.
- A numeric-looking literal is normalised ("007" is the number 7), matching how a spreadsheet
  treats it.

## Architecture

```
Core/      sparse model + typed values + A1 addressing + ranges, undo history, paste
Formulas/  parser, AST, dependency graph, function registry, engine
Controls/  SpreadsheetGridControl (render / geometry / headers / viewport / input)
Import/    importer interface, CSV reader, XLSX reader
Export/    exporter interface, CSV writer, XLSX writer
Ooxml/     SpreadsheetML constants and text sanitation shared by reader and writer
Views/     MainWindow: grid + scroll bars + the single reusable editor
```

The design rules that keep it fast (and that should be preserved) are documented in
[AGENTS.md](AGENTS.md).

## Not implemented

Multiple sheet tabs, cell formatting (fonts, colours, borders, merged cells, column widths),
find/replace and sorting are out of scope for this version. Export writes one worksheet.

**Number formats are the exception.** They are read from a workbook's style table and written back,
so a date or a percentage keeps its meaning through a save. Everything else about a cell's style is
dropped, so a bold cell comes back plain.

A date *imported* from a file is a serial with a date format, which is what makes it behave like a
number. One *typed* into a cell stays the text you typed, because the raw input is kept verbatim: it
displays, and the date functions read it, but arithmetic on it will not work.

`DATE`, `TODAY` and `NOW` are not implemented. A formula cannot give its own result a number format,
so `=DATE(2024,3,5)` would show the bare serial 45356 unless the cell had already been given a date
format by hand.

Worth knowing:

- **Undo history is in memory only**, capped at 200 steps, and is not written into the file.
- A formula the parser cannot read is left exactly as it was, so its references do not follow a copy
  or an inserted row.
- Pasting text from another application inserts formulas verbatim: an outside paste has no known
  origin, so there is nothing to shift its relative references by.
- Deleting a row that a formula pointed at leaves `#REF!`, and a range with one deleted corner
  collapses to a bare `#REF!`.

