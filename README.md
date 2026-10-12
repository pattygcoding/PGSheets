# PGSheets

[![CI](https://github.com/pattygcoding/PGSheets/actions/workflows/ci.yml/badge.svg)](https://github.com/pattygcoding/PGSheets/actions/workflows/ci.yml)

A cross-platform spreadsheet desktop application: a virtualised grid drawn by a single control, one
floating in-place editor, a dependency-tracking formula engine, and `.xlsx` / `.csv` reading and
writing. Written in Rust, with `eframe`/`egui` for the window — and no spreadsheet library anywhere in
the dependency tree, because every OOXML part, content type, relationship, style-table entry and cell
serialisation here is our own.

## Documentation

Full user documentation lives in [`Documents/`](Documents/):

| Page | Covers |
|---|---|
| [Getting started](Documents/getting-started.md) | Build, run, first edits, opening and saving |
| [Using the grid](Documents/using-the-grid.md) | Navigation, selection, the status-bar summary |
| [Editing](Documents/editing.md) | Undo, the clipboard, inserting and deleting rows and columns |
| [Formulas](Documents/formulas.md) | The language, references, operators, errors |
| [Function reference](Documents/functions.md) | Every function with its arguments |
| [Number formats](Documents/number-formats.md) | Percentages, currency, dates and the format language |
| [Files](Documents/files.md) | What is read and written, and what round-trips |
| [Keyboard shortcuts](Documents/keyboard-shortcuts.md) | Every key in one table |
| [Limitations](Documents/limitations.md) | What is not built, and why |
| [Development](Documents/development.md) | Building, testing and the code layout |

The sections below are the short version; those pages are the detail.

## Highlights

- **No widget per cell.** The grid paints the whole sheet in one pass of painter calls and only ever
  visits the cells inside the viewport, so a 1,048,576 × 16,384 sheet costs the same per frame as a
  small one.
- **Sticky headers** for columns (`A`, `B`, `C`…) and rows (`1`, `2`, `3`…), with the selected block
  highlighted in both.
- **In-place editing** with a single floating text box, repositioned over the active cell when you
  double-click it, press `F2`, or simply start typing.
- **Range selection** by dragging, Shift+click, Shift+arrows or `Ctrl+A`, with an Excel-style
  Count / Sum / Average summary at the right of the status bar.
- **Undo and redo** that collapse a paste, a cut or a range clear into a single step.
- **Clipboard as tab-separated text** — the flavour Excel uses for a range, so a block of cells travels
  between PGSheets and Excel in either direction. Raw input is what is copied, so formulas survive; a
  paste of something copied *here* shifts relative references and leaves `$`-anchored ones alone, while
  a paste from elsewhere keeps its references exactly as written.
- **Insert and delete rows and columns**, in the size of the selection, with every formula in the sheet
  rewritten to follow — and `#REF!` where a referenced row or column was removed.
- **Number formats**, from `0.00` and `#,##0.00` to `0%`, `$#,##0.00` and `yyyy-mm-dd`. A format changes
  what a cell shows without touching what it holds, so `=A1+1` on a date still means the next day. They
  are read from a `.xlsx` style table, applied from **Sheet → Number format**, and written back.
- **Files**: `.xlsx` and `.csv`/`.tsv`, both directions, written by this repository's own code.

## Building and running

The Rust toolchain (1.95 or newer — CI pins the exact version it tests). From the repository root,
one command does the rest:

```powershell
.\pg start            # PowerShell; in cmd it is just:  pg start
```

`pg` also takes `test`, `check` (format, lint and test, as CI runs them), `release` and `fmt`. The raw
commands, if you would rather be explicit:

```powershell
cd rust
cargo run  -p pgsheets-app                          # launch the app
cargo test --workspace                              # 243 tests, headless
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt  --all --check
```

## Architecture

The workspace is a chain of crates, and the chain is the architecture: a dependency pointing backwards
is a design error, because it is what keeps the UI, the file formats and the model out of each other.

```
pgsheets-core/      addressing, values, literals, number formats, serial dates, undo, clipboard
pgsheets-formulas/  parser, AST, dependency graph, engine, function library, writer, rewriter
pgsheets-sheet/     the sparse workbook: one mutation point, undo wiring, structural edits, paste
pgsheets-io/        xlsx and csv readers and writers, over our own OOXML packaging
pgsheets-app/       the window: grid, editor, menus (the only place UI types appear)
pgsheets-testkit/   shared fixtures for the test suites
```

The design rules that keep it fast (and that should be preserved) are in [AGENTS.md](AGENTS.md);
[Development](Documents/development.md) has the longer story, including how this tree replaced an
earlier C#/.NET implementation.

## Not implemented

Multiple sheet tabs, cell formatting (fonts, colours, borders, merged cells, column widths),
find/replace and sorting are out of scope for this version. Saving writes one worksheet.

**Number formats are the exception.** They are read from a workbook's style table and written back, so a
date or a percentage keeps its meaning through a save. Everything else about a cell's style is dropped,
so a bold cell comes back plain.

A date *imported* from a file is a serial with a date format, which is what makes it behave like a
number. One *typed* into a cell stays the text you typed, because the raw input is kept verbatim: it
displays, and the date functions read it, but arithmetic on it will not work.

`DATE`, `TODAY` and `NOW` are not implemented. A formula cannot give its own result a number format, so
`=DATE(2024,3,5)` would show the bare serial 45356 unless the cell had already been given a date format
by hand.

Worth knowing:

- **Undo history is in memory only**, capped at 200 steps, and is not written into the file.
- A formula the parser cannot read is left exactly as it was, so its references do not follow a copy or
  an inserted row.
- Pasting text from another application inserts formulas verbatim: an outside paste has no known origin,
  so there is nothing to shift its relative references by.
- Deleting a row that a formula pointed at leaves `#REF!`, and a range with one deleted corner collapses
  to a bare `#REF!`.
- The window has been tested headlessly rather than by hand: nobody has launched it yet, and no file
  written here has been opened in Excel. See [Development](Documents/development.md#cutover).
