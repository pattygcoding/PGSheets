# Files

## What you can open and save

| Format | Open | Save | Notes |
|---|---|---|---|
| `.xlsx` | Yes | Yes | The native format. Values, formulas, number formats and the simple text styles round-trip. |
| `.xlsm` | Yes | No | Macros cannot be written, so **Ctrl+S** sends you to Save As as `.xlsx`. |
| `.csv` | Yes | Yes | Comma-separated text, UTF-8. |
| `.tsv` | Yes | Yes | Tab-separated text. |
| `.txt` | Yes | Yes | Treated as CSV — comma-separated on save. |

Opening a file **replaces** what is on screen, clears the undo history (there is nothing meaningful
to undo back to), and reports a summary in the status bar: the extent, how many non-empty cells were
imported, and any warnings.

## Reading a workbook

An `.xlsx` file is a zip of XML parts, and this app reads it directly — no third-party library is
involved. From it, it takes:

| Read | Detail |
|---|---|
| Shared strings, inline strings | Text, including rich text, which is flattened |
| Numbers | As numbers |
| Booleans | As `TRUE` / `FALSE` |
| Formulas | As formulas, re-evaluated by the local engine; the cached result is used instead if you ask for values |
| Number formats | From the workbook's style table, so a date arrives as a serial with its date format |
| Cell styles | Bold, italic, underline, horizontal alignment, text colour, the background fill and wrap, from the same style table |
| Sheet name | Kept, and used as the worksheet name when the sheet is saved |

**Every worksheet is read**, and every worksheet is written back, so a multi-sheet workbook round-trips
whole. A sheet that is not a worksheet — a chartsheet or a macro sheet — is the exception: it cannot be
represented as cells, so it is skipped with a warning and the file is then never saved over in place.
See *Saving* below.

Functions the local engine does not implement become `#NAME?`, and the names are listed in the
status bar so you can see what the file was relying on. The formula text is kept either way, so
saving never loses a formula this app cannot compute.

**A note on dates.** Date-formatted cells come in as the serial number the file holds plus the format
that turns it into a date. That is why a date from a real workbook sorts, sums and calculates like a
number, and why it still looks like a date.

**A note on numbers.** A numeric-looking literal is normalised: a cell holding the text `007` is
imported as the number 7, which is how a spreadsheet treats it. Use the `0000` format to show leading
zeros.

## Reading delimited text

CSV, TSV and TXT files are read by a quote-aware parser:

| Handled | Detail |
|---|---|
| Quoting | Fields wrapped in `"` may contain the delimiter, newlines and `""` for a literal quote |
| Line endings | CRLF, LF and CR all work |
| Byte-order mark | A UTF-8 BOM at the front is stripped |
| Encoding | UTF-8 |
| Delimiter | Detected from the first line: `,`, `;`, tab or `|`, whichever appears most |

There is no header row to declare: the first line is data, so a CSV of `Name,Amount` imports the word
`Name` into `A1`.

## Saving

### `.xlsx`

Writes a complete but minimal package: content types, relationships, workbook, a style table sized to
the number formats, fonts and alignments actually used across every sheet, and one part per worksheet
using inline strings. The style table declares only what the cells need, numbered from the first id the
specification leaves free.

| Preserved | Not preserved |
|---|---|
| Values, with numeric-looking literals normalised the way a spreadsheet normalises them | Fonts (family and size), borders |
| Raw input, so formulas stay formulas | Merged cells and column widths |
| Number formats | Chartsheets, macro sheets and other non-worksheet sheets |
| Bold, italic, underline, horizontal alignment, text colour, the background fill and wrap | Macros |
| Every worksheet, and every worksheet name | |

Formula results that are *errors* are deliberately left uncached, so another application recalculates
them rather than inheriting this engine's `#NAME?` for a function it does not implement.

### `.csv`, `.tsv`, `.txt`

RFC 4180-style text: fields are quoted when they contain the delimiter, a quote or a newline, and
quotes inside are doubled. The delimiter follows the extension — tab for `.tsv`, comma otherwise —
and a UTF-8 byte-order mark is written so that other applications read the encoding correctly. This
app strips it again on the way back in.

Two kinds of loss are reported as warnings rather than left to be discovered: values that cannot
survive a round trip, and text that looks like a number (which will come back as one).

### When Ctrl+S does *not* overwrite

Every worksheet is written, so a multi-sheet workbook saves in place. Two cases are still sent to
**Save As** instead, with an explanation in the status bar:

1. **A macro-enabled `.xlsm` file** — a VBA project cannot be produced, and a file that silently
   dropped its macros would be a lie.
2. **A workbook that held a sheet this app cannot represent** — a chartsheet or a macro sheet, which
   has no cells to hold and would be destroyed by a save.

The rule behind both is the same, and it is deliberate: *a document this app cannot write back in
full is never overwritten in place.* After a save to a new file, in-place saving resumes, because the
file on disk now holds exactly what is on screen.

## Round-trip summary

What survives opening a file, changing it and saving it again:

| | `.xlsx` | `.csv` / `.tsv` |
|---|---|---|
| Cell values | Yes | Yes |
| Formulas | Yes | No — the text of the formula is written as text |
| Number formats | Yes | No |
| Dates as numbers | Yes | No — a date becomes text |
| Bold, italic, underline, alignment, text colour, fill, wrap | Yes | No |
| Multiple worksheets | Yes | No — one table, so only the active sheet |
| Fonts, borders, merged cells, widths | No | No |

## Saved colours

The colours you save from the ribbon's colour menu are the one thing this app keeps outside a document.
They live in a small text file under your user configuration:

| Platform | File |
|---|---|
| Windows | `%APPDATA%\PGSheets\colors.txt` |
| macOS | `~/Library/Application Support/PGSheets/colors.txt` |
| Linux and other | `$XDG_CONFIG_HOME/pgsheets/colors.txt`, or `~/.config/pgsheets/colors.txt` |

One `saved #RRGGBB` line per colour, in the order they were added. Nothing else is written there: the
recently used colours are held in memory for the session only, so applying a colour never touches the
disk. A missing or unreadable file is simply an empty palette, and a palette that cannot be written is
ignored rather than reported — neither is worth interrupting anyone over.

## How far this has been verified

The `.xlsx` writer is checked structurally by the test suite — the parts exist, are well-formed XML,
carry no byte-order marks, declare every part in the content types, and have style tables whose
counts match their children and whose elements are in the order the specification requires. It is
then read back by this app's own importer to prove the two agree.

What automated tests cannot prove is that **another** application accepts the file. Opening a saved
workbook in Excel is a manual check, and if Excel ever offers to *repair* a file this app wrote, that
is a bug worth reporting.

Next: [limitations.md](limitations.md) for the honest list of what is not built.
