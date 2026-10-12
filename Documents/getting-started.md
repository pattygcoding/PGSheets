# Getting started

## What you need

- **A Rust toolchain, 1.95 or newer.** Nothing else. PGSheets has no third-party spreadsheet packages
  and no other runtime dependency; the grid, the formula engine and the `.xlsx` reader and writer are
  all part of this repository.
- **No network access at run time.** The one-time `cargo` restore when you first build is the only
  thing that touches the internet.

It runs on Windows, macOS and Linux — the window is `eframe`/`egui`, which is cross-platform, and CI
builds and tests on all three.

## Build and run

From the repository root, one command:

```powershell
.\pg start            # PowerShell; in cmd it is just:  pg start
```

That builds what changed and opens the window. `pg` also takes `test`, `check` (format, lint and test,
as CI runs them), `release` and `fmt`.

Or do it by hand:

```powershell
cd rust
cargo run -p pgsheets-app
```

Or build the release binary once and run it directly:

```powershell
.\pg release
.\rust\target\release\pgsheets.exe
```

There is no installer. After a release build, `rust\target\release\pgsheets.exe` is a single file you
can copy elsewhere.

## What you see when it opens

The window is a menu bar, a name box and formula bar, the grid, a status bar, and scroll bars.

```
+--------------------------------------------------------------+
| File  Edit  Sheet                                            |  menu
+--------------------------------------------------------------+
|  B2          |  =B2+C2                                       |  name box + formula bar
+--------------------------------------------------------------+
|    |     A      |     B      |     C      |     D      |      |  column headers
|----+------------+------------+------------+------------+------|
|  1 |  Region    |  Q1        |  Q2        |  Total     |      |
|  2 |  North     |  1200      |  1450      |  2650      |      |
|  3 |  South     |  980       |  1100      |  2080      |      |
|  4 |  East      |  1600      |  1300      |  2900      |      |
|  5 |            |            |            |            |      |
|  6 |  Totals    |  3780      |  3850      |  7630      |      |
+--------------------------------------------------------------+
| Ready                          Count: 6  Sum: 7630  Average: …|  status bar
+--------------------------------------------------------------+
```

To make the window useful before you open a file, the first sheet is seeded with that sample table.
It is ordinary cell content — edit over it, clear it, or start a new document with **Ctrl+N**.

## Your first edits

| I want to | Do this |
|---|---|
| Replace what is in a cell | Click it and start typing. The first character opens the editor with that character; the rest of the cell's old content is replaced. |
| Add to what is in a cell | Press **F2**, or double-click the cell, then edit the text. |
| Enter a value without moving | Type it and click another cell. |
| Enter a value and go down | **Enter** — the active cell moves down one row. |
| Enter a value and go right | **Tab** — the active cell moves right one column. |
| Abandon an edit | **Escape** while editing. Nothing is written. |
| Edit a long value comfortably | Click the cell and use the **formula bar**; it shows and edits that cell's raw input. **Enter** commits, **Escape** reverts. |

"Raw input" is the important idea: a cell stores exactly what you typed. `0.155` is stored as
`0.155`, `=B2+C2` is stored as `=B2+C2`, and saving writes those back unchanged. That is why a
formula you typed is still a formula after a save.

## Opening a file

**File → Open** (**Ctrl+O**) accepts:

- `.xlsx` and `.xlsm` (Excel workbooks)
- `.csv`, `.tsv` and `.txt` (delimited text)

Only the **first worksheet** of a workbook is read; the others are reported in the status bar as a
warning rather than silently ignored. See [files.md](files.md) for what is read from each kind.

If the sheet on screen has unsaved changes, opening another file asks first.

## Saving

| Command | What it does |
|---|---|
| **Ctrl+S** (File → Save) | Writes back to the file you opened. The first time, it asks for a name — as does any file that cannot be written back in full. |
| **Ctrl+Shift+S** (File → Save As) | Always asks, and lets you choose `.xlsx` (the default), `.csv` or `.tsv`. |

Two cases send **Ctrl+S** to Save As instead of overwriting, both because writing in place truncates
the file:

- **A macro-enabled `.xlsm` file**, because a VBA project cannot be written.
- **A workbook with more than one worksheet**, because only one is written — overwriting would
  destroy the sheets that were never loaded.

The title bar reads `PGSheets - Untitled` and gains a `*` while there are unsaved changes, and
closing or replacing the document prompts to save first.

Next: [using-the-grid.md](using-the-grid.md) for navigation and selection, or
[keyboard-shortcuts.md](keyboard-shortcuts.md) for the full key list.
