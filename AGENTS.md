# AGENTS.md

Guidance for AI agents and contributors working in this repository.

## What this project is

PGSheets is a cross-platform spreadsheet desktop application: a virtualised custom-drawn grid, a single
floating in-place editor, and a dependency-tracking formula engine. It opens, edits and saves `.xlsx`
and `.csv` files.

It is written in Rust. There is no second implementation, no .NET project, and no third-party
spreadsheet library — see [the ADR](Documents/adr/0001-rust-rewrite.md) for how it got that way.

## Stack (do not change without a deliberate decision)

| Choice | Value | Why |
|---|---|---|
| Toolchain | Rust, `rust-version = "1.95"` | The floor `eframe` 0.36 requires; CI pins the exact version it tests |
| UI framework | `eframe`/`egui` 0.36, immediate mode | "One control painting every visible cell in one pass" is what immediate mode *is* |
| Dependencies | `eframe`, `zip`, `quick-xml`, `rfd` | `std` has neither a zip container nor an XML reader, and neither of the last two is a spreadsheet library |
| File formats | ours | Every OOXML part, content type, relationship, style-table entry and cell serialisation, and both CSV codecs, are written here |
| Cell formatting | number formats, bold/italic/underline, horizontal alignment, text colour and wrap | Rendered on the grid, stored on the cell, and round-tripped through the style table. Everything else a style carries — fonts, borders, fills, merged cells, widths — is dropped |

There are no online API calls, API keys, telemetry or quota-limited services anywhere in the app. The
only network access is one-time `cargo` restore at build time.

### The port is finished, and two things are now frozen

Until recently `rust/` ran beside a C#/.NET 8 + Avalonia implementation of the same program, which was
the behavioural oracle while the port was written. That tree has been deleted. Consequently:

- `rust/crates/pgsheets-core/tests/golden/number_format.tsv` (89 cases) and
  `rust/crates/pgsheets-sheet/tests/golden/formulas.tsv` (17 seeded cells and 243 formulas) are
  reference output captured from the deleted implementation. **They cannot be regenerated.** A change
  that disagrees with them is a behaviour change: argue for it, do not re-record the fixture.
- Two checks the old tree made possible are still open: nobody has launched the Rust window, and no
  Rust-written file has been opened in Excel. Do those before trusting the app with real work.

## Layout

```
rust/                      a cargo workspace
  crates/
    pgsheets-core/      addressing, values, literals, number formats, serial dates, undo, clipboard
    pgsheets-formulas/  parser, AST, dependency graph, engine, function library, writer, rewriter
    pgsheets-sheet/     the sparse workbook: one mutation point, undo wiring, structural edits, paste
    pgsheets-io/        xlsx and csv readers and writers
    pgsheets-app/       the window: grid, editor, menus (the only place UI types appear)
    pgsheets-testkit/   shared fixtures for the test suites
Documents/            user documentation: getting started, grid, editing, formulas, files, limitations
  adr/                why the decisions were made, including the rewrite itself
```

**Crate boundaries are the architecture.** `pgsheets-core` → `pgsheets-formulas` → `pgsheets-sheet` →
`pgsheets-io`/`pgsheets-app`. A dependency pointing backwards is a design error, not a style one: it is
what keeps the UI, the file formats and the model out of each other.

## Commands

From `rust/`:

```powershell
cargo build  --workspace                   # every crate; --release for the shipped binary
cargo test   --workspace                   # headless: no display, no window
cargo run    -p pgsheets-app               # launch the app
cargo fmt    --all --check                 # formatting is enforced, not suggested
cargo clippy --workspace --all-targets -- -D warnings
```

`.github/workflows/ci.yml` runs format → lint → build → test on Ubuntu, Windows and macOS for every
push and pull request to `main`, and uploads an `lcov` coverage report from Ubuntu. Reproduce it
locally with the five commands above.

Or from the repository root, through the wrapper — which is what it exists for, since every real
command runs from `rust/`:

```powershell
.\pg start      # build what changed and launch the app
.\pg test       # every test, headless
.\pg check      # format, lint and test, the way CI does — the pre-push command
.\pg release    # the optimised binary, at rust/target/release/pgsheets.exe
.\pg fmt        # format in place
```

`pg.ps1` holds it; `pg.cmd` is the `cmd.exe` entry point, so `pg start` works there without a `.\`
prefix, and it bypasses the execution policy for that one run.

## Testing

260 tests, all headless — no display, no window, no dependence on the working directory. Each crate
owns its own suite:

| Suite | Covers |
|---|---|
| `pgsheets-core/tests/` | addressing and `$` anchors, ranges, values and their coercion, literals, serial dates, undo, clipboard blocks, number formats, cell styles |
| `pgsheets-formulas/tests/` | precedence-aware rendering and the formula rewriter, which is what reference rewriting turns on |
| `pgsheets-sheet/tests/` | the sparse workbook, undo wiring, a format and a style following its cell, structural edits, formula-aware paste, and the engine driven through a test double |
| `pgsheets-io/tests/` | CSV and xlsx readers and writers: quoting, delimiters, BOM, shared and inline strings, dates, formulas, warnings, and the package's parts and style-table counts |
| `pgsheets-app/tests/` | the grid's geometry and its scroll bars, the widget driven by synthetic input, a whole frame of the window, and real files on disk |

Three things worth knowing before adding tests:

- **The app needs no window either.** `geometry_tests.rs` tests the layout arithmetic directly — it is
  a plain type rather than a method on the control — and `grid_input_tests.rs` builds a root `Ui` in a
  headless `egui::Context` and feeds it synthetic events. `window_tests.rs` drives one frame of the
  window, which is what covers the single editor, the clipboard round trip, undo and the
  unsaved-changes question. `file_tests.rs` is the only suite that puts a file on disk; each test gets a
  directory of its own under the system temp dir and removes it afterwards.
- **egui has four behaviours that cost time to learn**, all of them by design and none of them
  documented where you would look: modifiers are tracked only from `Event::ModifiersChanged` (a `Key`
  event carries them but does not set the state); a drag is recognised only *after* the pointer has
  moved, so a selection grows from the press origin rather than from where the pointer has reached; a
  second press of a key with no release counts as auto-repeat and is ignored, so synthetic keys need
  their releases; and **unmodified arrow and Tab presses are consumed by egui for its own widget-focus
  navigation**, which is why the grid re-takes the keyboard every frame the window says it owns it, and
  why the formula bar is a label until it is clicked.
- **Two suites compare against frozen golden output**, `include_str!`-ed so they depend on no working
  directory: the format interpreter (89 cases) and a fixed sheet with 243 formulas covering every
  function and its error branches. The format one caught a real defect the first time it ran — `0.5`
  under `"0"` is `1`, not `0`, and Rust's own `{:.0}` rounds half-to-even where the original rounded
  half away from zero.

## Architecture invariants

These are load-bearing. Breaking one silently degrades correctness or performance.

1. **The grid never creates a widget per cell.** `GridView` is one widget whose entire content is
   painted by a single pass of `Painter` calls, and only cells inside the viewport are visited — the
   loop bounds come from the layout, not from the sheet.
2. **Recalculation happens on edit, never on render.** The render path reads
   `CellData::display_value` and nothing else. Do not evaluate or parse a formula while painting.
3. **Allocation discipline in the render path.** Colours and strokes are `const`s, text goes through
   egui's galley cache, and only a cell whose text actually overflows pays for a clip rectangle. No
   per-cell object allocation.
4. **Sparsity.** Cells live in a `HashMap<(i32, i32), CellData>`. Writing `""` removes the entry, and a
   computed value must ignore cells that do not exist, so clearing a cell is not undone by the
   recalculation that follows it.
5. **Exactly one in-place editor**, positioned from a cell's rectangle. Never a text box per cell, and
   never one that is present when it is not needed: egui's focus navigation hands the keyboard to
   whatever focusable widget is in the way.
6. **Readers write through `ImportSink`** and never touch the UI, which is what keeps them
   unit-testable. A bulk import runs inside one `begin_bulk_update`/`end_bulk_update`, so it costs one
   recalculation rather than one per cell.
7. **Cycle detection is by re-entry**: the engine tracks in-progress and done cells per evaluation run,
   and re-entering an in-progress cell yields `#CIRCULAR!` rather than hanging.
8. **Writers read through `ExportSource`**, never a workbook's writer members, so a writer cannot mutate
   the document and stays unit-testable. The workbook implements that interface, which is why a saved
   file is exactly what the grid is showing.
9. **The workbook has exactly one mutation point.** `set_raw_input` reads the previous value, applies,
   then records an undo step; undo and redo replay through the same path so they never re-record. A
   number format and a style travel with their value through that pair, which is why the three never
   disagree. Read the
   previous state *before* mutating: the write goes through the very record a caller holds, so reading
   afterwards records "no change".
10. **Undo steps follow `begin_bulk_update`.** Opening a bulk update opens an undo group and closing it
    commits one step, which is why clearing or pasting a range undoes in a single action. Replacing the
    document wholesale (open, new) drops the history instead, because there is nothing meaningful to
    return to.
11. **Selection is an anchor plus an active cell**, normalised into a range, so it may be dragged in any
    direction; nothing stores a per-cell selection flag.
12. **Formulas are stored as text and are the source of truth.** `CellData::raw_input` holds exactly
    what was typed, so saving never rewrites a formula. Programmatic changes go through the rewriter and
    the writer, and a formula that cannot be parsed is left untouched rather than guessed at.
13. **A document the app cannot write back in full is never overwritten in place.** `Save` writes one
    worksheet and truncates the target, so a workbook that held more — or a name no writer claims —
    goes to `Save As`. `SheetExporter::can_save_in_place` is the one place that decides, and the loss is
    reported through `describe_lost_worksheets` rather than left to be discovered. A save also writes a
    sibling `.name.saving` file and moves it into place, so a writer that fails halfway cannot truncate
    the file that was already there.

## Extension points

- **New worksheet function**: register its name and implementation in `pgsheets-formulas`'s function
  library. Arguments arrive as unevaluated `AstNode`s, so evaluate only the branches you need (`IF` is
  the model for this) and enumerate ranges rather than materialising them. Two rules every built-in
  follows, both asserted by a sweep over the whole library: check the argument count first and report
  `#VALUE!` rather than panicking, and hold no state between calls. Shared argument handling, wildcard
  matching and criterion parsing live in `functions/helpers.rs`, `functions/wildcards.rs` and
  `functions/criteria.rs` — reuse those rather than re-deriving them.
- **New file format**: implement `SpreadsheetImporter` and add one line to `ImporterRegistry::standard`.
- **New save format**: implement `SpreadsheetExporter` and add one line to `SheetExporter::standard`.
- **New engine**: implement `FormulaEngine` and pass it to `Workbook::new`.

## Conventions

- Doc comments (`///`, `//!`) on public items; comments explain *why*, not *what*.
- `[workspace.lints]` forbids `unsafe_code`, denies the `unused` group and warns on `missing_docs`, and
  every CI command builds with `-- -D warnings`. Warnings stay at zero.
- Tests stay headless and depend on no working directory.
- Numeric parsing and formatting is deterministic and testable, and deliberately independent of the
  machine's locale.

## Known limitations (intentional, not bugs)

- One worksheet is imported (the first). Multi-sheet tabs are not implemented, and a workbook with more
  than one sheet is never saved over in place — see invariant 13.
- **Number formats and the simple text styles round-trip; the rest of a style does not.** A number format
  code, and bold, italic, underline, horizontal alignment, text colour and wrap, are read from the style
  table on import and written back on export. Fonts (family and size), borders, the cell fill, merged
  cells and column widths are not read or written at all, so a cell with a custom font or a border comes
  back in the default face with no border.
- **A CSV export carries no style.** A `.csv`/`.tsv` file has nowhere to put a number format or a style,
  so both are lost on a text save — the same loss the format warnings already describe.
- **Wrap has no column of its own.** A wrapped cell wraps to its column's width, but rows are still a
  fixed height, so text taller than a row is clipped rather than pushing the row taller.
- Because a formula has no way to give its result a number format, `DATE`, `TODAY` and `NOW` stay
  unimplemented: `=DATE(2024,3,5)` would show the bare serial 45356 unless the cell had already been
  given a date format by hand, and this app does not guess one.
- A date *typed* into a cell stays text rather than becoming a serial, because the raw input records
  exactly what was typed. It displays correctly and the date functions read it; what it does not do is
  behave like a number. A date *imported* from a file is a serial with a format, so it does.
- A numeric-looking literal is normalised on save ("007" becomes the number 7), matching how a
  spreadsheet treats it. A CSV field that looks like a number is treated as one, so leading zeros in
  such fields are lost.
- xlsx output has been validated structurally (parts, content types, relationships, style counts) and by
  reading it back with our own importer. Opening it in Excel is still a manual check, and until it has
  been done the writer should be treated as unproven.
- Formulas are re-evaluated by the local engine. A function the engine does not implement surfaces as
  `#NAME?`, and the import reports it as a warning. Formula results that are errors are deliberately
  left uncached on save, so the reading application recalculates rather than inheriting `#NAME?`.
- The format interpreter covers what workbooks usually use and refuses the rest — scientific notation,
  fractions, elapsed time (`[h]:mm`), alignment fills and text sections fall back to the plain
  rendering rather than being approximated. The code is still read and written back unchanged, so a
  format this app cannot render survives a round trip intact. `?` padding is not reproduced either.
- The function library covers the common ground, not the long tail: there is no `LOOKUP`/`XLOOKUP`, no
  `TEXT`/`SEARCH`/`TEXTBEFORE`, and no array or spill semantics — `SUMPRODUCT` needs equally sized
  ranges rather than broadcasting a smaller one across a larger.
- A lookup reports a miss as `#N/A`; nothing else produces that code, so `IFNA` only ever catches a
  lookup and `ISNA` is only ever true for one.
- Structural edits rewrite the whole sheet, so inserting a row costs O(populated cells) rather than
  O(cells moved). Fine at current sizes; a bulk shift would want a per-row offset map instead.
- A formula the parser cannot read is left exactly as it was by a rewrite, which means its references do
  not follow a copy or an inserted row. Deleting a reference leaves `#REF!`, and a range with one
  deleted corner collapses to a bare `#REF!` rather than to a partial range.
- Pasting text from another application inserts formulas verbatim: an outside paste has no known
  origin, so there is nothing to shift its relative references by.
- Undo history lives in memory, is capped at 200 steps and is not written to the file.
- The selection summary is skipped once a workbook has more than 20,000 populated cells, because the
  scan would otherwise run on every step of a drag. It counts number *values*, so a cell holding "5" as
  text is not in the count.
- Deep dependency chains rely on recursion; a formula chain thousands of cells deep could exhaust the
  stack.
- The window has never been launched by a human. Everything under it is covered by headless tests; the
  layout, the dialogs and the appearance are not.
