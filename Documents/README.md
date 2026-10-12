# PGSheets documentation

PGSheets is a cross-platform spreadsheet for the desktop: a virtualised grid you can type in, a
dependency-tracking formula engine with **100 functions**, number formats, and the ability to open,
edit and save **`.xlsx`** and **`.csv`** files with no third-party spreadsheet library involved.

It is deliberately offline. There are no online API calls, no accounts, no telemetry and no
runtime downloads: the only network access in the whole project is the one-time `cargo` restore when
you build it.

## Where to start

| If you want to… | Read |
|---|---|
| Build it and open it for the first time | [getting-started.md](getting-started.md) |
| Know what every key does | [keyboard-shortcuts.md](keyboard-shortcuts.md) |
| Move around a sheet and select things | [using-the-grid.md](using-the-grid.md) |
| Type, undo, copy, and insert or delete rows | [editing.md](editing.md) |
| Write formulas and understand errors | [formulas.md](formulas.md) |
| Look up one of the 100 functions | [functions.md](functions.md) |
| Show `0.0%`, `$1,234.57` or a date | [number-formats.md](number-formats.md) |
| Open and save files, and know what survives | [files.md](files.md) |
| Know what is *not* built yet | [limitations.md](limitations.md) |
| Work on the code | [development.md](development.md) |

## A sixty-second tour

```powershell
.\pg start            # PowerShell; in cmd it is just:  pg start
```

The window opens with a small sample table already filled in. Now try this:

1. Click cell **B2** and type `1200`, then press **Enter**.
2. Click **D2**, type `=B2+C2`, press **Enter**. The formula's result appears; the formula bar shows
   what you typed.
3. Select **B2:D4** by dragging, and read the status bar: it reports Count, Sum and Average of the
   numbers you selected.
4. Press **Ctrl+Z** a few times — every edit undoes separately.
5. Select **B2:B6** and choose **Sheet → Number format → `$#,##0.00`**. The numbers become currency.
   **Ctrl+Z** puts them back.
6. Press **Ctrl+S**, save as `demo.xlsx`, then reopen it with **Ctrl+O**: values, formulas and number
   formats all come back.

That is most of the application. The rest of these pages go into detail.

## Read the same thing on the web

If you are looking at this repository on GitHub, the pages above are plain Markdown files in
[`Documents/`](.) — the links work in the GitHub file browser and in any Markdown viewer.

## How this documentation is organised

Each page covers one job and is meant to be read on its own. They are written against the behaviour
of the program as it actually is, including the parts that are missing — if something is limited, it
is stated plainly in [limitations.md](limitations.md) rather than left for you to discover.

**Working on the code instead of using it?** The engineering notes live in
[../AGENTS.md](../AGENTS.md): architecture invariants, the test suite, conventions, and how to add a
function or a file format. [development.md](development.md) summarises it and points there.
