# ADR 0001: Rewrite PGSheets in Rust

- **Status:** accepted, and carried out: the Rust implementation replaced the C# tree, which was
  deleted once it matched. The golden fixtures captured from it are now frozen reference output, and the
  two manual checks the old tree allowed — launching the window, and exchanging files with Excel — are
  recorded as still open in [`Documents/development.md`](../development.md).
- **Date:** 2026-10-11
- **Affects:** the "Stack" and "Commands" sections of the root `AGENTS.md`, which are updated in the
  same change. The rest of `AGENTS.md` — the architecture invariants, the conventions, the
  limitations — survives the port and is restated in Rust terms.

## Context

PGSheets is a cross-platform spreadsheet desktop application: a virtualised custom-drawn grid, a
single floating in-place editor and a dependency-tracking formula engine, able to open, edit and save
`.xlsx` and `.csv` files.

The C#/.NET 8 + Avalonia implementation is ~8,800 lines of source and ~4,800 lines of tests. Roughly
70% of that (addressing, values, the sparse model, undo history, the number-format interpreter, the
serial-date codec, the formula engine and function library, and both file formats) contains no UI
types at all, and 789 xUnit tests already pin its behaviour.

The request is to rewrite the program in Rust.

## Decision

**Rewrite in Rust, in this repository, under `rust/`, with the C# tree kept until the port reaches
parity.**

1. **UI framework: `eframe`/`egui` (immediate mode).** The central architectural invariant — one
   control that paints every visible cell in a single pass, no control per cell — is what immediate
   mode does by construction. egui's `Painter` is the direct counterpart of Avalonia's
   `Render(DrawingContext)`, its galley cache is the counterpart of the `FormattedText` cache, and a
   single `TextEdit` with a stable `Id` is the counterpart of the one reusable `TextBox`. A retained
   toolkit (`iced`, `slint`) would mean re-deriving the same custom-drawn control inside a framework
   whose grain is the opposite.

2. **Dependencies: five, none of them a spreadsheet library.** `eframe`/`egui` for the window,
   `zip` (pure-Rust deflate backend only, so no C toolchain on any CI platform) and `quick-xml` for
   the `.xlsx` container, and `rfd` for native open/save dialogs. `std` has neither a zip container
   nor an XML reader; hand-writing a DEFLATE decoder would add a week and a class of decompression
   bugs for no user-visible benefit. The project's rule is *no third-party spreadsheet packages*, and
   it is kept: every OOXML part, content type, relationship, style-table entry and cell
   serialisation, and both CSV codecs, remain ours.

3. **Layout: a workspace that mirrors the existing folder seams.** `pgsheets-core`,
   `pgsheets-formulas`, `pgsheets-io` and `pgsheets-app`, plus `pgsheets-testkit`. The seams the C#
   tree enforces by convention (`ICellStore`, `IExportSource`, "the only place UI types appear")
   become *crate boundaries*, which the compiler enforces.

4. **The C# implementation is the behavioural oracle until cutover.** Every test file is ported
   one-to-one, and a throwaway `tools/PGSheets.Golden` console project dumps golden data from the C#
   implementation (formula corpus, number-format matrix, serial-date matrix, file round trips) which
   the Rust tests compare against. Rewriting a behaviour-compatible program without an oracle is
   guesswork; this repository's unusual advantage is that it still has one.

5. **Warnings are errors, as before.** `[workspace.lints]` forbids `unsafe_code`, denies the `unused`
   group, and warns on `missing_docs`; every CI command builds with `-- -D warnings`. This is the
   Rust spelling of `TreatWarningsAsErrors`.

## Consequences

### Behaviour that must not drift

The `Documents/` pages are the specification and stay valid: the same formulas, the same number
formats, the same errors, the same save rules (raw input written verbatim, `.xlsm` never overwritten
in place, a multi-worksheet workbook never saved over in place), the same single-worksheet import.

### Deliberate deviations from the C# shape

- **`UndoStack` does not hold an `apply` callback.** In C# the stack replays changes through a
  delegate the model hands it. Rust cannot borrow the model mutably through a field it owns, so the
  stack returns the changes and the model applies them. Same grouping, same coalescing, same
  suspension, same 200-step cap, same observable behaviour.
- **The engine contract lives in `pgsheets-core`, not next to the engine.** The model depends on the
  engine, so the trait must sit below the model. `pgsheets-formulas` implements it. An alternative
  engine is still a drop-in, which is the property the interface existed for.
- **The store is passed to the engine per call** (`engine.recalculate(store, cell)`) rather than being
  attached once behind a back-pointer. This removes a shared-mutable-state hazard the C# code lives
  with by construction.
- **The last microsecond of the calendar.** `SerialDates::try_to_datetime` returns "not a date" at the
  top edge of the serial range where .NET's `DateTime.AddDays` would throw
  `ArgumentOutOfRangeException`. Refusing is strictly safer, and no workbook contains such a serial.
- **Text inside `CellValue` is an `Rc<str>`.** The C# readonly struct copies a reference; the Rust
  equivalent must be cheap to clone on the evaluation path. This makes `CellValue` — and therefore
  the model — single-threaded. That is already true of the design: the UI thread owns the model, and
  importers work on a buffer, not on the model.

### Invariants that change meaning

- "No per-cell allocation in the render path" cannot be restated literally: an immediate-mode UI
  builds shapes per frame by design. It becomes **"bounded work per frame, proportional to the
  visible cells, with no allocation proportional to the sheet"**, asserted by a counting-allocator
  test rather than by inspection.
- The `Render` half of the grid is no longer reachable only by a manual smoke test: `egui_kittest`
  drives egui headlessly, so grid interaction and painting move *into* CI. Coverage should therefore
  improve over the ~75% the C# tree reports.

### Toolchain

`rust-version = "1.85"` is the floor (what `eframe` requires) rather than an exact pin, because this
repository's rule is that only a base toolchain is guaranteed to be present. CI pins the exact
toolchain it tests against. `Cargo.lock` is committed: this is an application, so the build should be
reproducible.

