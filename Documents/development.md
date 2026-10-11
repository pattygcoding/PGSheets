# Development

The engineering notes live in [../AGENTS.md](../AGENTS.md) — architecture invariants, conventions,
the test suite, and the extension points. This page is the short version and a map of the code.

## Requirements and commands

You need **the .NET 8 SDK** and nothing else. There are no third-party spreadsheet packages, and the
only network access is the one-time NuGet restore.

```powershell
dotnet restore PGSheets.sln
dotnet build   PGSheets.sln -c Release
dotnet test    PGSheets.sln
dotnet run     --project src/PGSheets
```

`TreatWarningsAsErrors` is on, so a warning fails the build. That is deliberate: the project keeps
warnings at zero rather than accumulating them.

## Continuous integration

`.github/workflows/ci.yml` runs restore → build → test on **Ubuntu, Windows and macOS** for every push
and pull request to `main`, and uploads the `.trx` results and a Cobertura coverage report as
artifacts. To reproduce it locally, including coverage:

```powershell
dotnet restore PGSheets.sln
dotnet build   PGSheets.sln -c Release --no-restore
dotnet test    PGSheets.sln -c Release --no-build --collect:"XPlat Code Coverage"
```

## How the code is arranged

```
src/PGSheets/
  Core/        The workbook: addressing, values, the sparse model, undo, number formats,
               serial dates, clipboard blocks. No UI, no file formats.
  Formulas/    Lexer, parser, AST, the dependency graph, the writer and rewriter, and the
               built-in function library.
  Controls/    SpreadsheetGridControl — the single custom-drawn control, split into Render,
               Geometry, Headers, Viewport and Input partials.
  Import/      xlsx and CSV readers, behind ISpreadsheetImporter.
  Export/      xlsx and CSV writers, behind ISpreadsheetExporter.
  Ooxml/       Shared SpreadsheetML details: namespaces, reader/writer settings, text sanitation.
  Views/       MainWindow and the save prompt. The only place Avalonia types appear.
```

Files marked `*` in `AGENTS.md` are `partial` classes split purely to keep each file small.

## The invariants worth knowing before changing anything

These are load-bearing; breaking one silently degrades correctness or performance. The full list is
in [../AGENTS.md](../AGENTS.md), but the four that surprise people most:

1. **The grid never creates a control per cell.** `SpreadsheetGridControl` paints every visible cell
   in one `Render` call. Only cells in the viewport are visited.
2. **Recalculation happens on edit, never on render.** `Render` only reads a precomputed display
   string. Do not evaluate or format during painting.
3. **Formulas are stored as text and are the source of truth.** Saving writes the raw input back
   verbatim. Anything that must change a formula goes through `FormulaRewriter`, which parses,
   rewrites references and re-renders — and leaves an unparseable formula alone.
4. **The model has exactly one mutation point.** `ApplyRawInput` writes value and number format
   together; undo and redo replay through it. Read the previous state *before* mutating, because it
   writes through the very object a caller might read afterwards.

## Testing

The test project is xUnit and runs **headless** — no `AppBuilder`, no window, no display, and no
dependence on the current directory. That is what makes the whole engine, the model, the importers,
the exporters and the grid's geometry testable in CI on three platforms.

Two useful facts when adding tests:

- `new SpreadsheetGridControl()` works without Avalonia initialisation, and its geometry
  (`TryHitTest`, `GetCellBounds`, `SetScrollOffset`) depends only on layout fields, so hit-testing and
  scrolling *are* unit-testable. Only `Render` needs a drawing surface.
- `XlsxFixtureBuilder` writes a real xlsx with `System.IO.Compression` alone, so the xlsx tests depend
  on no third-party package. `MutableParts()` breaks a single part for failure tests.

Coverage sits at roughly 75% overall and about 94% across `Core`, `Formulas`, `Import`, `Export` and
`Ooxml`. What is left out is `Render`, `MainWindow`, `App` and `Program`, which need a real window and
are covered by a manual smoke launch instead.

## Adding things

| To add | Do |
|---|---|
| A worksheet function | `BuiltinFunctions.Register("NAME", implementation)` and add the name to the registry. Arguments arrive unevaluated, so evaluate only the branches you need. Check the argument count first and report `#VALUE!` rather than throwing — a sweep over the whole library asserts this. |
| A file format | Implement `ISpreadsheetImporter` and add one line to `SheetImporter.Importers`. |
| A save format | Implement `ISpreadsheetExporter` and add one line to `SheetExporter.Writers`. |
| A formula engine | Implement `IFormulaEngine` and pass it to `SpreadsheetModel`'s constructor. |

Shared argument handling for functions lives in `BuiltinFunctions.Helpers.cs`, wildcard matching in
`BuiltinFunctions.Wildcards.cs`, and criterion parsing in `BuiltinFunctions.Criteria.cs` — reuse those
rather than re-deriving them.

## Conventions

- File-scoped namespaces, `namespace PGSheets.<Area>`.
- Nullable reference types and implicit usings enabled repo-wide.
- XML doc comments on public members; comments explain *why*, not *what*.
- Numeric parsing and formatting use the invariant culture deliberately, so results are deterministic
  and testable.
- Tests stay headless.

Back to the [documentation index](README.md).
