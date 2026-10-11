# AGENTS.md

Guidance for AI agents and contributors working in this repository.

## What this project is

PGSheets is a cross-platform spreadsheet desktop application: a virtualised custom-drawn grid,
a single floating in-place editor, and a dependency-tracking formula engine. It can open, edit and
save `.xlsx` and `.csv` files.

## Stack (do not change without a deliberate decision)

| Choice | Value | Why |
|---|---|---|
| Target framework | `net8.0` | Only the .NET 8 SDK is guaranteed present |
| UI framework | Avalonia **11.3.22** (Fluent theme) | Cross-platform; version pinned exactly |
| Font | `Avalonia.Fonts.Inter` | Embedded in the assembly, so no runtime font download |
| Spreadsheet file libraries | **none** | CSV and XLSX reading *and writing* are BCL-only (`System.IO.Compression` + `System.Xml`) |

**No third-party spreadsheet packages.** Keeping the dependency surface to the UI framework is an
explicit project goal. Do not add ClosedXML / DocumentFormat.OpenXml / CsvHelper / NPOI.

There are no online API calls, API keys, telemetry or quota-limited services anywhere in the app.
The only network access is one-time NuGet restore at build time.

## Layout

```
src/PGSheets/
  Core/        CellAddress, CellReference, CellRange, CellValue, CellData, Literals, SheetBounds,
               SerialDates, NumberFormat*, ICellStore, IExportSource, SpreadsheetModel, UndoStack,
               CellBuffer, SheetPaste
  Formulas/    IFormulaEngine, FormulaEngine, FormulaParser*, AstNodes, Operators,
               FormulaWriter, FormulaRewriter, DependencyGraph, BuiltinFunctions*
  Controls/    SpreadsheetGridControl* (Render / Geometry / Headers / Viewport / Input), GridEvents
  Import/      ISpreadsheetImporter, SheetImporter, CsvImporter, XlsxImporter*, ImportResult
  Export/      ISpreadsheetExporter, SheetExporter, CsvExporter, XlsxWriter*, ExportResult
  Ooxml/       SpreadsheetMl: namespaces, XML settings and text sanitation shared by both sides
  Views/       MainWindow.axaml(.cs), SavePrompt.axaml(.cs)
tests/PGSheets.Tests/   xUnit; no Avalonia initialisation required
Documents/   user documentation: getting started, grid, editing, formulas, files, limitations
```

Files marked `*` are split into `partial class` files purely to keep each file small and focused
(for example `SpreadsheetGridControl.Render.cs`, `FormulaParser.Primary.cs`).

## Commands

```powershell
dotnet build PGSheets.sln -c Release      # whole solution
dotnet test  PGSheets.sln                 # 789 tests, headless
dotnet run   --project src/PGSheets       # launch the app
```

`.github/workflows/ci.yml` runs restore → build → test on Ubuntu, Windows and macOS for every push
and pull request to `main`, and uploads the `.trx` results and a Cobertura coverage report as
artifacts. Reproduce it locally with:

```powershell
dotnet restore PGSheets.sln
dotnet build   PGSheets.sln -c Release --no-restore
dotnet test    PGSheets.sln -c Release --no-build --collect:"XPlat Code Coverage"
```

## Testing

`tests/PGSheets.Tests` is xUnit and runs **headless** — no `AppBuilder`, no window, no display:

| File | Covers |
|---|---|
| `FormulaEngineTests` / `FormulaEngineAdvancedTests` | arithmetic, precedence, ranges, aggregates, dependency propagation, IF laziness |
| `FormulaErrorTests` | malformed formulas, out-of-range references, repair after a cycle |
| `BuiltinFunctionsTests` | every registered function plus its error branches |
| `DependencyGraphTests` | edge bookkeeping, transitive collection, cycles |
| `SpreadsheetModelTests` | sparsity, raw-vs-display, deletion, bulk update |
| `CellAddressTests` / `CellValueTests` / `LiteralsTests` | addressing, coercion, formatting, errors |
| `CsvImporterTests` | quoting, escaping, delimiters, line endings, BOM |
| `XlsxImporterTests` + `XlsxFixtureBuilder` | OOXML parts, shared/inline strings, dates, formulas, warnings |
| `CsvExporterTests` | delimited quoting and escaping, delimiter per extension, BOM, lossy-value warnings |
| `XlsxWriterTests` | OOXML packaging: content types, relationship targets, style-table counts, escaping, sanitising |
| `RoundTripTests` | import → export → re-import fidelity for xlsx and csv |
| `SheetExporterTests` / `ExportResultTests` | exporter resolution, `.xlsm` refusal, warning cap |
| `CellRangeTests` / `CellBufferTests` | normalised ranges, block shape |
| `CellReferenceTests` | `$` anchors: formatting and parsing |
| `FormulaWriterTests` / `FormulaRewriterTests` | precedence-aware rendering, reference translation, structural shifts |
| `StructuralEditTests` | inserting and deleting rows and columns, reference rewriting, undo |
| `BuiltinFunctionsMathTests` / `BuiltinFunctionsTextTests` | the numeric and text function families, and their error branches |
| `BuiltinFunctionsStatisticsTests` / `BuiltinFunctionsCriteriaTests` | distribution functions, and how criteria such as `">=10"` or `"a*"` are interpreted |
| `BuiltinFunctionsLogicalTests` | logic, type tests, and the conditional functions |
| `BuiltinFunctionsLookupTests` | `VLOOKUP`, `HLOOKUP`, `INDEX`, `MATCH`, `CHOOSE` |
| `SerialDatesTests` / `BuiltinFunctionsDateTests` | the 1900 serial date system, and reading a date part from a serial or from ISO text |
| `NumberFormatTests` | the number-format interpreter, including everything it must refuse |
| `SpreadsheetModelFormatTests` | how a format changes what the grid shows, and that it undoes, survives edits and follows its cell |
| `UndoStackTests` / `SpreadsheetModelUndoTests` | grouping, coalescing, suspension, recalculation after undo |
| `SheetPasteTests` / `CopyPasteTests` | paste tiling, and copy → tab-separated text → paste end to end |
| `GridGeometryTests` | hit-testing, cell rectangles, clamping, scroll/extent maths, range selection |

Two things worth knowing before adding tests:

- `new SpreadsheetGridControl()` works without Avalonia initialisation, and its geometry
  (`TryHitTest`, `GetCellBounds`, `SetScrollOffset`) depends only on the layout fields, so those
  paths **are** unit-testable. Only `Render` needs a drawing surface.
- `XlsxFixtureBuilder` writes a real xlsx with `System.IO.Compression` only, so xlsx tests do not
  depend on any third-party package. Use `MutableParts()` to break a single part for failure tests.

Coverage (as measured by the CI collector) is ~75% overall and ~94% across `Core`, `Formulas`,
`Import`, `Export` and `Ooxml`. The remainder is `Render`/`MainWindow`/`App`/`Program`, which are
covered by a manual smoke launch instead — `MainWindow` in particular is clipboard and undo wiring
that needs a real window.

## Architecture invariants

These are load-bearing. Breaking them silently degrades correctness or performance.

1. **The grid never creates child controls per cell.** `SpreadsheetGridControl` is a single
   `Control` whose entire content is painted in one `Render(DrawingContext)`. Only cells inside the
   viewport are visited; loop bounds come from `Bounds` and the scroll offsets.
2. **Recalculation happens on edit, never on render.** `Render` must only read
   `CellData.DisplayValue`. Do not evaluate formulas or parse formulas during painting.
3. **Allocation discipline in the render path.** Pens and brushes are `static readonly`
   `ImmutablePen` / `ImmutableSolidColorBrush`. `FormattedText` is cached in a bounded dictionary
   (`MaxTextCacheEntries`) because it is immutable and recurs while scrolling. No LINQ or
   per-cell object allocation inside `Render`.
4. **Sparsity.** `SpreadsheetModel` stores only populated cells in
   `Dictionary<(int Row, int Column), CellData>`. Writing `""` removes the entry. `SetComputed`
   must ignore cells that do not exist, so deleting a cell is not undone by the recalc that follows.
5. **There is exactly one `TextBox` for editing**, declared in `MainWindow.axaml`, repositioned via
   `SpreadsheetGridControl.GetCellBounds`. Never create an editor per cell.
6. **Importers write through `IImportSink`** and never touch the UI, so they stay unit-testable.
   Bulk imports run inside `BeginBulkUpdate()` / `EndBulkUpdate()` so the engine does one `Rebuild`
   instead of one recalculation per cell.
7. **Cycle detection is by re-entry**: `FormulaEngine` tracks `InProgress` and `Done` cells per
   evaluation run; re-entering an in-progress cell yields `#CIRCULAR!` rather than hanging.
8. **Exporters read, never write.** They take an `IExportSource`, not `SpreadsheetModel`'s writer
   members, so a writer cannot mutate the workbook and stays unit-testable. The model implements the
   interface, which is why a saved file is exactly what the grid is showing.
9. **The model has exactly one mutation point.** `SetRawInput` reads the previous value, calls
   `ApplyRawInput`, then records an undo step; undo and redo replay through `ApplyRawInput` so they
   never re-record. A number format travels with its value through the same pair, which is why the
   two never disagree. Read the previous state *before* the mutation: `ApplyRawInput` writes through
   the very `CellData` object a caller holds, so reading it afterwards records "no change".
10. **Undo steps follow `BeginBulkUpdate`.** Opening a bulk update opens an undo group and closing it
    commits one step, which is why clearing or pasting a range undoes in a single action. A wholesale
    document replacement (open, new) runs inside `SuspendUndoHistory()`, because there is nothing
    meaningful to return to.
11. **Selection is an anchor plus an active cell.** `SpreadsheetGridControl.Selection` is normalised
    from those two, so a range may be dragged in any direction; a plain move collapses the anchor
    onto the active cell. Nothing stores a per-cell selection flag.
12. **Formulas are stored as text and are the source of truth.** `CellData.RawInput` holds exactly
    what the user typed, so saving never rewrites a formula. Programmatic changes (paste, insert,
    delete) go through `FormulaRewriter`, which parses, rewrites references and re-renders through
    `FormulaWriter`. Nothing else may edit formula text, and a formula that cannot be parsed is left
    untouched rather than guessed at.
13. **A document this app cannot write back in full is never overwritten in place.** `Save` writes a
    single worksheet and truncates the target, so a workbook that held more sends the user to
    `Save As` — the same rule that sends an `.xlsm` there, because a VBA project cannot be produced.
    `SheetExporter.CanSaveInPlace` is the one place that decides this, and the resulting loss is
    reported through `SheetExporter.DescribeLostWorksheets` rather than left to be discovered.

## Extension points

- **New worksheet function**: `BuiltinFunctions.Register("NAME", implementation)` then add the name
  to the registry dictionary. Arguments arrive as unevaluated `IFormulaNode`s, so evaluate only the
  branches you need (see `IF`) and use `RangeRefNode.Enumerate` for ranges.
  Two rules every built-in follows, both asserted by a sweep over the whole library: check the
  argument count first and report `#VALUE!` rather than throwing, and never hold state between
  calls. Shared argument handling lives in `BuiltinFunctions.Helpers.cs`, wildcard matching in
  `BuiltinFunctions.Wildcards.cs`, and criterion parsing in `BuiltinFunctions.Criteria.cs` — reuse
  those rather than re-deriving them.
- **New file format**: implement `ISpreadsheetImporter` and add one line to `SheetImporter.Importers`.
- **New save format**: implement `ISpreadsheetExporter` and add one line to `SheetExporter.Writers`.
- **New engine**: implement `IFormulaEngine` and pass it to `SpreadsheetModel`'s constructor.

## Conventions

- File-scoped namespaces; `namespace PGSheets.<Area>`.
- `Nullable` and `ImplicitUsings` are enabled repo-wide via `Directory.Build.props`.
- XML doc comments on public types and members; comments explain *why*, not *what*.
- Culture: numeric parsing/formatting uses `CultureInfo.InvariantCulture` deliberately, so results
  are deterministic and testable.
- Warnings must stay at zero. `TreatWarningsAsErrors` is `true` in `Directory.Build.props`, so this
  is enforced by the compiler and by CI rather than by convention.
- Tests must stay headless: do not initialise Avalonia, open a window, or depend on the current
  directory.

## Known limitations (intentional, not bugs)

- One worksheet is imported (the first). Multi-sheet tabs are not implemented, and a workbook with
  more than one sheet is never saved over in place — see invariant 13.
- **Number formats round-trip; cell formatting does not.** A format code is read from the style table
  on import and written back on export, so a date stays a date and a percentage stays a percentage.
  Fonts, colours, borders, alignment, merged cells and column widths are not read or written at all.
- A cell's style carries more than a number format, and only the number format is taken: a cell
  formatted as bold but not as a number comes back plain.
- Because a formula has no way to give its result a number format, `DATE`, `TODAY` and `NOW` stay
  unimplemented: `=DATE(2024,3,5)` would show the bare serial 45356 unless the cell had already been
  given a date format by hand, and this app does not guess one.
- A date *typed* into a cell stays text rather than becoming a serial, because `RawInput` records
  exactly what was typed (invariant 12). It displays correctly and the date functions read it; what
  it does not do is behave like a number. A date *imported* from a file is a serial with a format,
  so it does behave like one.
- xlsx formulas are re-evaluated by the local engine. Functions the engine does not implement
  surface as `#NAME?` and are listed in `ImportResult.Warnings`.
- Cell formatting (fonts, colours, borders, alignment) and column resizing are not built. Number
  formats are the one part of a cell style that is interpreted and round-tripped.
- The format interpreter covers what workbooks usually use and refuses the rest — scientific
  notation, fractions, elapsed time (`[h]:mm`), alignment fills and text sections all fall back to
  the plain rendering rather than being approximated. The code is still read and written back
  unchanged, so a format this app cannot render survives a round trip intact. `?` padding is not
  reproduced either, so a "?" placeholder aligns nothing.
- The function library covers the common ground, not the long tail. There is no `LOOKUP`/`XLOOKUP`,
  no `TEXT`/`SEARCH`/`TEXTBEFORE`, and no array or spill semantics: `SUMPRODUCT` needs equally sized
  ranges rather than broadcasting a smaller one across a larger.
- A lookup reports a miss as `#N/A`; nothing else produces that code, so `IFNA` only ever catches a
  lookup and `ISNA` is only ever true for one.
- Structural edits rewrite the whole sheet through `ReplaceContents`, so inserting a row costs
  O(populated cells) rather than O(cells moved). Fine at current sizes; a bulk shift would want a
  per-row offset map instead.
- A formula the parser cannot read is left exactly as it was by a rewrite, which means its
  references do not follow a copy or an inserted row.
- Deleting a reference leaves `#REF!`, and a range with one deleted corner collapses to a bare
  `#REF!` rather than to a partial range.
- Undo history lives in memory, is capped at 200 steps and is not written to the file.
- The status-bar selection summary is skipped once a workbook has more than 20,000 populated cells,
  because the scan would otherwise run on every step of a drag.
- Saving writes one worksheet and no formatting: column widths, merged cells, fonts and number
  formats are not preserved. Raw input, so formulas, *is* preserved.
- A format this app cannot write is never overwritten: `.xlsm` saves fall through to Save As as
  `.xlsx`, because `vbaProject.bin` cannot be produced.
- Formula results that are errors are deliberately left uncached on save, so the reading application
  recalculates instead of inheriting `#NAME?` for functions this engine lacks.
- A numeric-looking literal is normalised on save ("007" becomes the number 7), matching how a
  spreadsheet treats it.
- xlsx output has only been validated structurally (parts, content types, relationships, style
  counts) and by reading it back with our own importer; opening it in Excel is a manual check.
- Deep dependency chains rely on recursion; a formula chain thousands of cells deep could exhaust
  the stack.
- A CSV/number-looking string is treated as a number, so leading zeros in such fields are lost.
