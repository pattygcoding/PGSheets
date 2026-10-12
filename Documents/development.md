# Development

The engineering notes live in [../AGENTS.md](../AGENTS.md) — architecture invariants, conventions, the
test suites and the extension points. This page is the short version and a map of the code.

## Requirements and commands

You need **a Rust toolchain (1.95 or newer)** and nothing else. None of the four dependencies is a
spreadsheet package, and the only network access is the one-time `cargo` restore.

```powershell
cd rust
cargo build  --workspace                   # every crate; --release for the shipped binary
cargo test   --workspace                   # 260 tests, headless: no display, no window
cargo run    -p pgsheets-app               # launch the app
cargo fmt    --all --check                 # formatting is enforced, not suggested
cargo clippy --workspace --all-targets -- -D warnings
```

`rust-version` in `Cargo.toml` is a floor rather than a pin; CI pins the exact toolchain it tests, and
enforces the same five steps on Ubuntu, Windows and macOS.

## Where the work stands

| Area | State |
|---|---|
| `pgsheets-core` | complete: addressing, values, literals, number formats, serial dates, undo, clipboard |
| `pgsheets-formulas` | complete: parser, AST, writer, rewriter, dependency graph, engine, all 83 functions |
| `pgsheets-sheet` | complete: the sparse workbook, undo wiring, structural edits, formula-aware paste |
| `pgsheets-io` | complete: xlsx and csv readers and writers, over our own OOXML packaging |
| `pgsheets-app` | complete: the grid (geometry, render, pointer and keyboard input, scroll bars), the window (menus including the number-format list, the one editor, the formula bar, the split status bar, the three-button unsaved-changes prompt, close interception) and the file layer (reading on a worker thread, saving through a staging file) — 70 headless tests |

`[workspace.lints]` denies the `unused` group and warns on `missing_docs`, and every CI command builds
with `-- -D warnings`, so a warning fails the build. That is deliberate: the project keeps warnings at
zero rather than accumulating them.

## How this tree got here, and what is frozen because of it

This crate chain is a rewrite of an earlier C#/.NET 8 + Avalonia implementation of the same program,
which ran beside it as the behavioural oracle while the port was written. That tree has since been
deleted — see [the ADR](adr/0001-rust-rewrite.md) — and with it the ability to regenerate two fixtures:

- `rust/crates/pgsheets-core/tests/golden/number_format.tsv` — 89 cases for the format interpreter.
- `rust/crates/pgsheets-sheet/tests/golden/formulas.tsv` — a fixed sheet and 243 formulas covering every
  function and its error branches.

They are reference output captured from the deleted implementation, checked in and `include_str!`-ed. A
change that disagrees with them is a behaviour change to be argued for, not a fixture to re-record.

Two checks the old tree made possible are still open, and are the reason to be careful before trusting
the app with real work:

1. **Launch the window and use it.** Everything under it is covered by 260 headless tests, but the
   layout, the menus, the dialogs and the appearance have never been seen. `cargo run -p pgsheets-app`,
   then: arrows, Tab, type-to-edit, a drag selection, the scroll bars, the menus, the formula bar (it
   becomes editable when clicked), Open / Save / Save As, and the unsaved-changes question when the
   window is closed with edits pending.
2. **Exchange files with Excel.** The xlsx writer is validated structurally, and by reading its own
   output back; only Excel can confirm that the rest of the world agrees. Open a file written here in
   Excel, and an Excel-written workbook here.

## How the code is arranged

```
rust/                      a cargo workspace
  crates/
    pgsheets-core/      addressing, values, literals, number formats, serial dates, undo, clipboard
    pgsheets-formulas/  parser, AST, dependency graph, engine, function library, writer, rewriter
    pgsheets-sheet/     the sparse workbook: one mutation point, undo wiring, structural edits, paste
    pgsheets-io/        xlsx and csv readers and writers
    pgsheets-app/       the window: grid (geometry, render, input, scroll bars), editor, menus, dialogs
    pgsheets-testkit/   shared fixtures for the test suites
Documents/            this documentation, and the ADRs under `adr/`
```

**Crate boundaries are the architecture.** `pgsheets-core` → `pgsheets-formulas` → `pgsheets-sheet` →
`pgsheets-io`/`pgsheets-app`. A dependency pointing backwards is a design error rather than a style
one: it is what keeps the UI, the file formats and the model out of each other.

## The invariants worth knowing before changing anything

These are load-bearing; breaking one silently degrades correctness or performance. The full list is in
[../AGENTS.md](../AGENTS.md), but the four that surprise people most:

1. **The grid never creates a widget per cell.** `GridView` paints every visible cell in one pass and
   only visits the cells inside the viewport.
2. **Recalculation happens on edit, never on render.** The render path reads a precomputed display
   string and nothing else. Do not evaluate or format during painting.
3. **Formulas are stored as text and are the source of truth.** Saving writes the raw input back
   verbatim. Anything that must change a formula goes through the rewriter, which parses, rewrites
   references and re-renders — and leaves an unparseable formula alone.
4. **The workbook has exactly one mutation point.** A value and its number format travel together
   through it, and undo and redo replay through it. Read the previous state *before* mutating, because
   the write goes through the very record a caller might read afterwards.

## Testing

291 tests, headless: no display, no window, no dependence on the working directory. That is what makes
the engine, the model, the readers, the writers, the grid's geometry and the window's own wiring
testable in CI on three platforms.

Three useful facts when adding tests:

- **The app needs no window either.** The layout arithmetic is a plain type, so it is tested directly;
  the widget and a whole frame of the window are driven through a headless `egui::Context` with
  synthetic events; and only one suite puts a real file on disk.
- **egui has four behaviours that cost time to learn**: modifiers arrive only as
  `Event::ModifiersChanged`; a drag is recognised only after the pointer has moved, so a selection grows
  from the press origin; a second key press with no release counts as auto-repeat and is ignored; and
  unmodified arrows and Tab are consumed by egui's own widget-focus navigation — which is why the grid
  re-takes the keyboard and why the formula bar is a label until it is clicked.
- `pgsheets-testkit` holds the shared fixtures, so a suite does not have to re-invent a memory-backed
  source or a recording sink.

## Adding things

| To add | Do |
|---|---|
| A worksheet function | Register it in `pgsheets-formulas`'s function library. Arguments arrive unevaluated, so evaluate only the branches you need. Check the argument count first and report `#VALUE!` rather than panicking — a sweep over the whole library asserts this. |
| A file format | Implement `SpreadsheetImporter` and add one line to `ImporterRegistry::standard`. |
| A save format | Implement `SpreadsheetExporter` and add one line to `SheetExporter::standard`. |
| A formula engine | Implement `FormulaEngine` and pass it to `Workbook::new`. |

Shared argument handling, wildcard matching and criterion parsing live in the function library's
`helpers`, `wildcards` and `criteria` modules — reuse those rather than re-deriving them.

## Conventions

- Doc comments on public items; comments explain *why*, not *what*.
- `[workspace.lints]` forbids `unsafe_code`, denies the `unused` group and warns on `missing_docs`;
  warnings stay at zero.
- Numeric parsing and formatting is deterministic and testable, and deliberately independent of the
  machine's locale.
- Tests stay headless.

Back to the [documentation index](README.md).
