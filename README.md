# PGSheets

[![CI](https://github.com/pattygcoding/PGSheets/actions/workflows/ci.yml/badge.svg)](https://github.com/pattygcoding/PGSheets/actions/workflows/ci.yml)

A high-performance, cross-platform spreadsheet desktop application built with C# / .NET 8 and
Avalonia UI. It renders a virtualised grid with a single custom-drawn control, edits cells in place
with one reusable `TextBox`, evaluates formulas through a dependency-tracking engine, and imports
`.xlsx` and `.csv` files.

## Highlights

- **No control-per-cell.** `SpreadsheetGridControl : Control` paints the whole sheet in one
  `Render(DrawingContext)` pass. Gridlines are pen primitives, text is cached `FormattedText`, and
  only the cells inside the viewport are ever touched — a 1,048,576 × 16,384 sheet costs the same
  per frame as a small one.
- **Sticky headers** for columns (`A`, `B`, `C`…) and rows (`1`, `2`, `3`…).
- **In-place editing** with a single floating `TextBox` that is repositioned over the active cell on
  double-click, `F2`, or when you start typing.
- **Formula engine** over a sparse model, with an extensible function registry, dependency tracking
  and circular-reference detection.
- **Import** of Excel workbooks (`.xlsx`) and delimited text (`.csv`, `.tsv`) including formulas.
- **Self-contained**: no third-party spreadsheet libraries. XLSX is read with
  `System.IO.Compression` and `System.Xml`; CSV has a hand-written RFC 4180 parser. The app makes no
  network calls at runtime and requires no API keys.

## Requirements

- .NET 8 SDK
- Windows, Linux or macOS (Avalonia desktop backends)

## Build and run

```powershell
dotnet build PGSheets.sln -c Release
dotnet run   --project src/PGSheets
dotnet test  PGSheets.sln          # 172 tests, headless
```

Warnings are errors (`TreatWarningsAsErrors` in `Directory.Build.props`), so a clean build really
does mean zero warnings.

## Testing

172 xUnit tests run headless — no Avalonia initialisation, no window, no display required:

```powershell
dotnet test PGSheets.sln
dotnet test PGSheets.sln --collect:"XPlat Code Coverage"   # writes a Cobertura report
```

They cover the formula engine (precedence, ranges, aggregates, propagation, cycles, every built-in
function and its error branches), the sparse model, A1 addressing and value coercion, the CSV and
XLSX readers (including malformed-workbook paths), the dependency graph, and the grid's hit-testing
and scroll maths. The XLSX fixtures are generated with the BCL, so the tests depend on no
third-party spreadsheet package.

Coverage is ~68% overall and ~93% across `Core`/`Formulas`/`Import`. The rendering path needs a real
drawing surface, so it is verified by launching the app rather than by unit test.

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
| `Ctrl+O` / File → Open | Import a `.xlsx` or `.csv` file |

The name box shows the active cell reference and the formula bar shows (and edits) its raw input.

## Formulas

Supported today:

- Cell references and ranges: `A1`, `$B$7`, `A1:B10`
- Operators: `+ - * / ^`, unary minus, parentheses, and comparisons `= <> < <= > >=`
- Functions: `SUM`, `MIN`, `MAX`, `AVERAGE`, `COUNT`, `COUNTA`, `IF`, `ABS`, `ROUND`, `SQRT`,
  `POWER`, `CONCAT`
- Errors: `#DIV/0!`, `#VALUE!`, `#REF!`, `#NAME?`, `#NUM!`, `#CIRCULAR!`

Examples: `=SUM(A1:A5)`, `=A1 + B2`, `=IF(B2>10,"big","small")`.

New functions are one line — see `BuiltinFunctions.Register` and the extension notes in
[AGENTS.md](AGENTS.md).

## Importing files

**CSV** is parsed with a quote-aware state machine: quoted fields, escaped quotes (`""`), embedded
newlines, CRLF/LF/CR line endings, UTF-8 BOM stripping and delimiter auto-detection
(`, ; tab |`).

**XLSX** is read directly from the Open Packaging zip: shared strings, inline strings, numbers,
booleans, cached results, formulas and date number formats. Only the first worksheet is imported.

Formulas are re-evaluated by the built-in engine by default. Any function the engine does not
implement becomes `#NAME?` and is reported as a warning (the count appears in the status bar).
Set `XlsxImporter.ImportFormulas = false` to import cached values as literals instead.

## Architecture

```
Core/      sparse model + typed values + A1 addressing
Formulas/  parser, AST, dependency graph, function registry, engine
Controls/  SpreadsheetGridControl (render / geometry / headers / viewport / input)
Import/    importer interface, CSV reader, XLSX reader
Views/     MainWindow: grid + scroll bars + the single reusable editor
```

The design rules that keep it fast (and that should be preserved) are documented in
[AGENTS.md](AGENTS.md).

## Not implemented

Saving/exporting, multiple sheet tabs, undo/redo, cell formatting, range selection, column resizing
and the clipboard are out of scope for this version. Date cells from xlsx are imported as ISO text.

