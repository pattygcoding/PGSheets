# Function reference

**100 functions**, in the categories below. Arguments in `[square brackets]` are optional.

Two things are true of every function here:

- **A wrong number of arguments is `#VALUE!`, never a crash.** Every function checks its arguments
  before using them, so a mistake in one cell cannot bring the sheet down.
- **Errors are passed along** rather than swallowed, unless looking at the error *is* the point —
  which is what `IFERROR`, `IFNA`, `ISERROR` and `ISNA` are for.

An aggregate — `SUM`, `MIN`, `AVERAGE`, `MEDIAN`, `STDEV` and the rest — **skips text and blanks**
inside a range rather than counting them as zero, and returns an error if any cell in the range holds
one. `COUNT` counts numbers; `COUNTA` counts anything non-empty.

## Numbers (21)

| Function | Returns |
|---|---|
| `SUM(value, …)` | The total of the numbers |
| `PRODUCT(value, …)` | The product of the numbers; `0` when there are none |
| `ABS(number)` | The magnitude, without sign |
| `SIGN(number)` | `-1`, `0` or `1` |
| `SQRT(number)` | The square root; `#NUM!` for a negative |
| `POWER(number, exponent)` | `number` raised to `exponent` (the same as `^`) |
| `EXP(number)` | e to that power |
| `LN(number)` | Natural logarithm; `#NUM!` at or below zero |
| `LOG(number, [base])` | Logarithm, base 10 by default; `#NUM!` for a base of `1` |
| `LOG10(number)` | Logarithm base 10 |
| `PI()` | 3.14159265358979 |
| `INT(number)` | Rounds **down** towards negative infinity: `INT(-2.1)` is `-3` |
| `TRUNC(number, [digits])` | Rounds **towards zero**: `TRUNC(-2.9)` is `-2` |
| `MOD(number, divisor)` | Division remainder, which takes the **divisor's** sign: `MOD(-3,5)` is `2` and `MOD(3,-5)` is `-2` |
| `ROUND(number, [digits])` | Rounds half away from zero: `ROUND(2.5)` is `3`, `ROUND(-2.5)` is `-3` |
| `ROUNDUP(number, [digits])` | Away from zero, always |
| `ROUNDDOWN(number, [digits])` | Towards zero, always |
| `CEILING(number, [significance])` | Up to the next multiple of the significance (1 by default): `CEILING(2.3,0.5)` is `2.5` |
| `FLOOR(number, [significance])` | Down to the previous multiple: `FLOOR(2.7,0.5)` is `2.5` |
| `RAND()` | A fresh random fraction in `[0, 1)`, seeded from the clock |
| `RANDBETWEEN(bottom, top)` | A whole number in the inclusive range; `#NUM!` when `bottom` exceeds `top` |

`INT`, `TRUNC` and `ROUNDDOWN` are worth telling apart: for `-2.9` they give `-3`, `-2` and `-2`.

## Statistics (22)

| Function | Returns |
|---|---|
| `MIN(value, …)` / `MAX(value, …)` | Smallest / largest number; `0` when nothing is numeric |
| `AVERAGE(value, …)` | The mean; `#DIV/0!` when nothing is numeric |
| `MEDIAN(value, …)` | The middle value, or the mean of the two middle ones; `#NUM!` over nothing |
| `MODE(value, …)` | The most frequent value; the smallest wins a tie; `#N/A` when nothing repeats |
| `COUNT(value, …)` | How many numbers |
| `COUNTA(value, …)` | How many non-empty cells |
| `COUNTBLANK(range)` | How many empty cells the range covers |
| `COUNTIF(range, criteria)` | How many cells match (see below) |
| `SUMIF(range, criteria, [sumRange])` | Adds the cells that match |
| `AVERAGEIF(range, criteria, [sumRange])` | Averages the cells that match; `#DIV/0!` when none do |
| `LARGE(range, k)` / `SMALL(range, k)` | The k-th largest / smallest, counting from 1; `#NUM!` outside the range |
| `RANK(number, range, [order])` | Its 1-based rank; order `0` or omitted ranks largest first; ties share a rank; `#N/A` when absent |
| `STDEV(value, …)` | Sample standard deviation; `#DIV/0!` with fewer than two numbers |
| `VAR(value, …)` | Sample variance |
| `SUMPRODUCT(range, range, …)` | Multiplies the ranges cell by cell and adds; `#VALUE!` if they differ in size |
| `COUNTIFS(range, criteria, …)` | How many rows satisfy **every** range/criteria pair |
| `SUMIFS(sumRange, range, criteria, …)` | Adds the rows that satisfy every pair |
| `AVERAGEIFS(avgRange, range, criteria, …)` | Averages the rows that satisfy every pair; `#DIV/0!` when none do |
| `MAXIFS(maxRange, range, criteria, …)` | The largest matching value; `0` when none match |
| `MINIFS(minRange, range, criteria, …)` | The smallest matching value; `0` when none match |

The `…IFS` functions take a leading **result range** and then any number of `range, criteria` pairs.
Every range must cover the same cells as the result range — a mismatched pair would silently line up
the wrong ones — and that is reported as `#VALUE!` rather than guessed at. `COUNTIFS` has no result
range of its own; it starts with the first `range, criteria` pair. Each criteria is the same one
`COUNTIF` accepts (see below).

### Criteria

`COUNTIF`, `SUMIF` and `AVERAGEIF` take a criteria that is either a value or a comparison written as
text:

| Criteria | Matches |
|---|---|
| `5`, `"5"` | Cells equal to 5 |
| `"apple"` | Cells equal to `apple`, ignoring case |
| `">10"`, `"<=3"` | Ordering comparisons |
| `"<>closed"` | Anything **not** equal to `closed` |
| `"a*"` | Text starting with `a` |
| `"a?c"` | `abc`, `axc`, and so on |
| `"~*"` | A literal asterisk |

`*` matches any run of characters, `?` exactly one, and `~` makes the next character literal.
Numeric criteria treat an empty cell as zero, which is why `"=0"` finds blank cells.

The sum range is **anchored at its own top-left and walked with the tested range's shape**, so
`SUMIF(A1:A4, ">2", B1)` adds `B3` and `B4`. With no third argument the tested range is the one that
gets added up.
## Logic (13)

The conditional functions evaluate **only the branch they are going to return**, which is what lets
you write a guard around a division that would otherwise raise an error.

| Function | Returns |
|---|---|
| `IF(condition, then, [else])` | The chosen branch; `FALSE` when there is no else branch |
| `IFS(condition, value, condition, value, …)` | The value of the first true condition; `#N/A` when none is |
| `IFERROR(value, fallback)` | The fallback **only** when the value is an error, otherwise the value |
| `IFNA(value, fallback)` | The same, but catching `#N/A` alone, so a real mistake still shows |
| `AND(value, …)` | `TRUE` when every argument is |
| `OR(value, …)` | `TRUE` when any argument is |
| `XOR(value, …)` | `TRUE` when an odd number of arguments are, so `XOR(TRUE,TRUE,TRUE)` is `TRUE` |
| `NOT(value)` | The opposite |
| `SWITCH(expression, value, result, …, [default])` | The result of the first matching value; `#N/A` without a default |
| `CHOOSE(index, value, …)` | The n-th value, counting from 1 |
| `TRUE()` / `FALSE()` | The boolean, without arguments |
| `NA()` | `#N/A` |

`AND`, `OR` and `XOR` count numbers as true when non-zero, and **skip text and blank cells that came
from a range** rather than treating them as false. With nothing to test at all they report `#VALUE!`.

```
=IFERROR(A1/B1,0)        zero instead of #DIV/0!
=IFS(A1>90,"A",A1>80,"B",TRUE,"C")
```

## Text (23)

A number used where text is wanted is read through its display form, so `LEN(1234)` is `4` and
`LEFT(1234,2)` is `"12"`.

| Function | Returns |
|---|---|
| `CONCAT(value, …)` | Everything joined, ranges included |
| `CONCATENATE(value, …)` | The same thing under its older name |
| `TEXTJOIN(delimiter, ignoreEmpty, value, …)` | Values joined with a separator, skipping blanks when the second argument is `TRUE` |
| `LEFT(text, [n])` / `RIGHT(text, [n])` | The first / last `n` characters, 1 by default; asking for more than exists is not an error |
| `MID(text, start, length)` | A run of characters; `start` counts from 1 |
| `LEN(text)` | How many characters |
| `UPPER(text)` / `LOWER(text)` | Case changed |
| `PROPER(text)` | First letter of each word capitalised |
| `TRIM(text)` | Ends removed **and runs of spaces inside collapsed to one** |
| `REPT(text, count)` | The text repeated; `#VALUE!` if the result would be too long for a cell |
| `SUBSTITUTE(text, old, new, [instance])` | Every occurrence replaced, or only the n-th |
| `FIND(find, within, [start])` | The 1-based position, **case sensitive**; `#VALUE!` when absent |
| `SEARCH(find, within, [start])` | The same, but **case insensitive** and honouring `*` and `?` |
| `REPLACE(text, start, length, new)` | The run of characters from `start` replaced by `new` |
| `TEXTBEFORE(text, delimiter, [instance], [matchMode], [matchEnd], [ifNotFound])` | The part before the delimiter |
| `TEXTAFTER(text, delimiter, [instance], [matchMode], [matchEnd], [ifNotFound])` | The part after the delimiter |
| `TEXT(value, format)` | The number rendered with a number-format code |
| `EXACT(a, b)` | `TRUE` when identical **including case** — the one text comparison that cares |
| `VALUE(text)` | Text read as a number; `#VALUE!` when it is not one |
| `CHAR(code)` | The character for a code in 1–255 |
| `CODE(text)` | The code of the first character |

`SEARCH` and the two split functions understand the `*` (any run) and `?` (one character) wildcards,
with `~` escaping the next character. `TEXTBEFORE`/`TEXTAFTER` count occurrences from the start when
`instance` is positive and from the end when it is negative; `matchMode` `1` ignores case; and
`matchEnd` `1` treats the end of the text as a delimiter, so a missing one yields the whole string
rather than `#N/A`. `TEXT`'s format code is the same language a cell's own number format uses, so a
code the interpreter cannot represent falls back to the plain rendering.

## Lookup (9)

A lookup that finds nothing returns **`#N/A`**, which is exactly what `IFERROR` and `IFNA` are for.
An index outside the table is `#REF!`, and a table that is not a range is `#VALUE!`.

| Function | Returns |
|---|---|
| `VLOOKUP(key, range, column, [approximate])` | The value in `column` (1-based) of the matching row |
| `HLOOKUP(key, range, row, [approximate])` | The same, across a row |
| `INDEX(range, row, [column])` | One cell of a range, 1-based; a single-row range reads a lone index as the column |
| `MATCH(key, range, [type])` | The 1-based position within a single row or column |
| `LOOKUP(key, lookupVector, [resultVector])` | The approximate lookup over a sorted vector; the last entry at or below the key |
| `XLOOKUP(key, lookupArray, returnArray, [ifNotFound], [matchMode], [searchMode])` | The matching entry, or the fallback instead of `#N/A` |
| `XMATCH(key, lookupArray, [matchMode], [searchMode])` | The 1-based position, with the same modes |
| `ROWS(range)` / `COLUMNS(range)` | How many rows / columns a range covers (a single cell is `1`) |

The fourth argument of `VLOOKUP` and `HLOOKUP` decides the search:

| Argument | Behaviour |
|---|---|
| `FALSE` | Exact match only. Nothing identical means `#N/A`. |
| `TRUE` or omitted | Assumes the key column is sorted ascending and returns the **last key at or below** the value you asked for. |

`MATCH` takes the type in the same spirit: `0` for exact, `1` for a list sorted ascending, `-1` for
one sorted descending.

`XLOOKUP` and `XMATCH` share two mode arguments. `matchMode` `0` (the default) matches exactly, `-1`
takes the next smaller value, `1` the next larger, and `2` reads the key as a wildcard pattern.
`searchMode` scans forwards when positive (the default) and backwards when negative. Both arrays must
be the same size and lie along a single row or column, and `XLOOKUP` reads the result from the
matching offset of its return array.

```
=VLOOKUP($A2,$B$2:$C$20,2,FALSE)     exact lookup, anchored table
=INDEX(B2:B20,MATCH($A2,A2:A20,0))   the same lookup, column independent
=IFNA(VLOOKUP(A2,Table,2,FALSE),"")  blank instead of #N/A
```

## Dates (4)

A date is a number with a date format, so these accept either a **serial number** or **ISO text**
(`"2024-03-05"`). Both shapes are accepted because a date from a file arrives as a serial, while a
date you type arrives as the text you typed.

| Function | Returns |
|---|---|
| `YEAR(date)` | The year |
| `MONTH(date)` | The month, 1–12 |
| `DAY(date)` | The day of the month |
| `WEEKDAY(date, [type])` | The day of the week, 1–7 |

`WEEKDAY`'s type decides where the week starts: `1` (the default) makes Sunday `1`, `2` makes Monday
`1`, and `3` makes Monday `0`.

```
=YEAR(A2)          2024 when A2 holds 45356 or "2024-03-05"
=WEEKDAY(A2)       3 — Tuesday, with Sunday as 1
```

## Type tests (8)

These **look at** a value instead of propagating it, so `ISERROR(1/0)` is `TRUE` rather than
`#DIV/0!`.

| Function | Is `TRUE` when the value is |
|---|---|
| `ISBLANK(value)` | Empty |
| `ISNUMBER(value)` | A number — `ISNUMBER("5")` is `FALSE`, because the cell holds text |
| `ISTEXT(value)` | Text |
| `ISLOGICAL(value)` | `TRUE` or `FALSE` |
| `ISERROR(value)` | Any error at all |
| `ISNA(value)` | `#N/A` in particular |
| `ISEVEN(value)` / `ISODD(value)` | An even / odd number; `#VALUE!` for anything else |

## The total

21 numbers + 22 statistics + 13 logic + 23 text + 9 lookup + 4 dates + 8 type tests = **100**.

For what is deliberately *not* here, and why, see [limitations.md](limitations.md).

