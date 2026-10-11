# AGENTS.md

Guidance for AI agents and contributors working in this repository.

## What this project is

PGSheets is a cross-platform spreadsheet desktop application: a virtualised custom-drawn grid,
a single floating in-place editor, and a dependency-tracking formula engine. It can import and
display `.xlsx` and `.csv` files.

## Stack (do not change without a deliberate decision)

| Choice | Value | Why |
|---|---|---|
| Target framework | `net8.0` | Only the .NET 8 SDK is guaranteed present |
| UI framework | Avalonia **11.3.22** (Fluent theme) | Cross-platform; version pinned exactly |
| Font | `Avalonia.Fonts.Inter` | Embedded in the assembly, so no runtime font download |
| Spreadsheet file libraries | **none** | CSV and XLSX reading are BCL-only (`System.IO.Compression` + `System.Xml`) |

**No third-party spreadsheet packages.** Keeping the dependency surface to the UI framework is an
explicit project goal. Do not add ClosedXML / DocumentFormat.OpenXml / CsvHelper / NPOI.

There are no online API calls, API keys, telemetry or quota-limited services anywhere in the app.
The only network access is one-time NuGet restore at build time.

## Layout

```
src/PGSheets/
  Core/        CellAddress, CellValue, CellData, Literals, SheetBounds, ICellStore, SpreadsheetModel
  Formulas/    IFormulaEngine, FormulaEngine, FormulaParser*, AstNodes, Operators,
               DependencyGraph, BuiltinFunctions*
  Controls/    SpreadsheetGridControl* (Render / Geometry / Headers / Viewport / Input), GridEvents
  Import/      ISpreadsheetImporter, SheetImporter, CsvImporter, XlsxImporter*, ImportResult
  Views/       MainWindow.axaml(.cs)
tests/PGSheets.Tests/   xUnit; no Avalonia initialisation required
```

Files marked `*` are split into `partial class` files purely to keep each file small and focused
(for example `SpreadsheetGridControl.Render.cs`, `FormulaParser.Primary.cs`).

## Commands

```powershell
dotnet build PGSheets.sln -c Release      # whole solution
dotnet test  PGSheets.sln                 # 172 tests, headless
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
| `GridGeometryTests` | hit-testing, cell rectangles, clamping, scroll/extent maths |

Two things worth knowing before adding tests:

- `new SpreadsheetGridControl()` works without Avalonia initialisation, and its geometry
  (`TryHitTest`, `GetCellBounds`, `SetScrollOffset`) depends only on the layout fields, so those
  paths **are** unit-testable. Only `Render` needs a drawing surface.
- `XlsxFixtureBuilder` writes a real xlsx with `System.IO.Compression` only, so xlsx tests do not
  depend on any third-party package. Use `MutableParts()` to break a single part for failure tests.

Coverage (as measured by the CI collector) is ~68% overall and ~93% across `Core`, `Formulas` and
`Import`. The remainder is `Render`/`MainWindow`/`App`/`Program`, which are covered by a manual
smoke launch instead.

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

## Extension points

- **New worksheet function**: `BuiltinFunctions.Register("NAME", implementation)` then add the name
  to the registry dictionary. Arguments arrive as unevaluated `IFormulaNode`s, so evaluate only the
  branches you need (see `IF`) and use `RangeRefNode.Enumerate` for ranges.
- **New file format**: implement `ISpreadsheetImporter` and add one line to `SheetImporter.Importers`.
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

- One worksheet is imported (the first). Multi-sheet tabs are not implemented.
- Date-formatted xlsx cells are imported as ISO text (`yyyy-MM-dd`), not as numeric serials.
- xlsx formulas are re-evaluated by the local engine. Functions the engine does not implement
  surface as `#NAME?` and are listed in `ImportResult.Warnings`.
- Saving/exporting, undo/redo, cell formatting, range selection and column resizing are not built.
- Deep dependency chains rely on recursion; a formula chain thousands of cells deep could exhaust
  the stack.
- A CSV/number-looking string is treated as a number, so leading zeros in such fields are lost.
