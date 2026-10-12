# Number formats

A number format changes what a cell **shows**, never what it **holds**. `0.155` with the format
`0.0%` reads `15.5%`, but the cell still contains the number `0.155` — so `=A1*2` is `0.31`, not
`31%`. That separation is the whole point: a spreadsheet that changed the value to make it look right
would be useless for arithmetic.

```
1200.5   formatted  $#,##0.00      shows   $1,200.50
0.155    formatted  0.0%           shows   15.5%
45356    formatted  yyyy-mm-dd     shows   2024-03-05
7        formatted  0000           shows   0007
```

## Applying one

**Sheet → Number format** offers these, and applies the chosen code to every cell in the selection:

| Menu entry | Shows |
|---|---|
| General | Clears the format — the plain rendering |
| `0` | A whole number |
| `0.00` | Two decimals |
| `#,##0` | A whole number with thousands separators |
| `#,##0.00` | Thousands separators and two decimals |
| `0%` | A percentage, no decimals |
| `0.0%` | A percentage with one decimal |
| `$#,##0.00` | Currency |
| `0000` | Padded with leading zeros |
| `yyyy-mm-dd` | An ISO date |
| `yyyy-mm-dd hh:mm` | An ISO date and time |
| `hh:mm:ss` | A time |

Applying a format is **one undo step**, however many cells it covered, and **Ctrl+Z** removes it
without touching the values.

Only cells that already exist are formatted. Selecting a whole column and choosing a format will not
create a cell for each of the million empty rows — but a format you apply to a cell that holds a
value **stays with that cell** when you clear it and type again, so "format the column, then fill it
in" works the way you would expect.

## Dates

A date in a spreadsheet is a **number plus a date format** — there is no separate date type. The
number counts days from 1900, with the time of day in the fraction, which is why a date sorts
correctly, can be added to (`=A1+1` is the next day) and can be subtracted from another date.

A date imported from a `.xlsx` file arrives as that serial together with the file's own date format,
so it displays the way the file said — `mm-dd-yy` shows `03-15-23`, not a bare number.

A date you **type** into a cell stays text, because a cell stores exactly what you typed. It displays
correctly and `YEAR`, `MONTH`, `DAY` and `WEEKDAY` read it, but arithmetic on it will not work. To
make it a real date, put the number in it and apply a date format.

`DATE`, `TODAY` and `NOW` are not implemented: a formula cannot give its own result a number format,
so `=DATE(2024,3,5)` would display the bare serial `45356` unless the cell had already been given a
date format by hand, and this app does not guess one.

## The format language

A format code is a small pattern language, and this app interprets the parts workbooks usually use.

A code may have up to four **sections** separated by `;`, tried in turn:

| Sections | Applies to |
|---|---|
| `positive` | Everything |
| `positive;negative` | Two sections, split by sign |
| `positive;negative;zero` | Three add a separate rendering for zero |
| `positive;negative;zero;text` | Four add one for text |

An empty section **hides** that value, which is how `0.00;` shows nothing at all for a negative
number. With two or more sections the negative section supplies its own sign, so `0.00;(0.00)` shows
`-1.5` as `(1.50)`.

### Placeholders and modifiers

| Written | Means |
|---|---|
| `0` | A digit, padded with zeros |
| `#` | A digit, but nothing if there is none |
| `?` | A digit; this app treats it as `#` (see the note below) |
| `.` | The decimal point |
| `,` between digits | Thousands separators |
| `,` at the end | Divide by a thousand, once per comma |
| `%` | Multiply by 100 and show a percent sign |
| `"text"` | Literal text, kept as written |
| `\x` | The literal character `x` |
| `_x` | A space the width of `x` |
| `*x` | A fill, which is ignored |
| `[Red]` | A colour directive, which is ignored |

### Date and time tokens

| Written | Shows |
|---|---|
| `yyyy` / `yy` | `2024` / `24` |
| `mmmm` / `mmm` | `March` / `Mar` |
| `mm` / `m` | The month, or the **minute** next to an hour or before seconds |
| `dddd` / `ddd` | `Tuesday` / `Tue` |
| `dd` / `d` | `05` / `5` |
| `hh` / `h` | The hour, on a 12-hour clock when the pattern has `AM/PM` |
| `ss` / `s` | The second |
| `AM/PM` or `A/P` | The half of the day |
| `.000` | Fractions of a second, to as many digits as you write |

So `hh:mm` is an hour and a minute while `dd/mm/yyyy` is a day, a month and a year — the same `mm`
means two different things, and the tokens around it decide which.

## What it refuses

The format language is large, and this app would rather show you the plain value than a wrong one. A
code containing something it cannot represent falls back to the **plain rendering** rather than being
approximated:

- scientific notation, `0.00E+00`
- fractions, `# ?/?`
- elapsed time, `[h]:mm:ss`
- a five-digit year or five-letter month, `yyyyy` or `mmmmm`
- a text placeholder, `@`
- more than four sections

**The code itself is kept.** Because a format is read from a file and written back exactly as it
appeared, a format this app cannot *render* still survives opening and saving intact — you see the
plain number while you are here, and see it properly again in another application.

The one thing not reproduced is `?` padding: a `?` placeholder behaves like `#`, so it will not align
decimal points by inserting spaces.

## Where formats come from and go

| Path | Carries number formats? |
|---|---|
| Opening and saving `.xlsx` | **Yes** — read from the workbook's style table and written back |
| Saving `.csv` / `.tsv` / `.txt` | No — delimited text holds values only |
| Copy and paste | No — the clipboard carries text, and pasting must not reformat what it lands on |

Everything **else** about a cell's style — fonts (family and size), borders, merged cells and column
widths — is not read or written at all. The one other thing that does round-trip is the set of simple
styles the ribbon sets: **bold**, *italic*, underline, horizontal alignment, text colour, the background
fill and wrap. So a cell that was bold comes back bold, and a cell that was bold *and* a percentage comes
back bold and still a percentage.

Next: [files.md](files.md) for what opening and saving does to your data.
