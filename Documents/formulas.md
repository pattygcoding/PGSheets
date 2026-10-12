# Formulas

A formula is a cell whose raw input starts with `=`. Everything after the sign is an expression,
evaluated by the built-in engine, and the result is what the grid shows. The formula bar always shows
the expression you typed, never the result — the cell shows the result.

```
=B2+C2                          arithmetic
=SUM(B2:B4)                     a range
=IF(D2>2000,"over","under")     a condition
=VLOOKUP($A2,$B$2:$C$20,2,FALSE) a lookup
```

## Operators

| Operator | Means |
|---|---|
| `+` `-` `*` `/` | Add, subtract, multiply, divide |
| `^` | Raise to a power (`2^10`, right-associative so `2^3^2` is `2^(3^2)`) |
| `-` | Unary minus, which binds tighter than `*` and looser than `^`, so `-A1^2` is `-(A1^2)` |
| `=` `<>` `<` `<=` `>` `>=` | Comparisons, which produce `TRUE` or `FALSE` |
| `( )` | Grouping |
| `,` | Separates function arguments |

Precedence runs from comparison, through additive, multiplicative, unary and power, up to function
calls and cell references.

Numbers compare numerically; anything else compares as text ignoring case, which is what `IF`
conditions need. `1` and `"1"` are not the same kind of value, though arithmetic will coerce the
text.

## References

| Written | Means |
|---|---|
| `A1` | A single cell |
| `$A$1` | A single cell, fully anchored |
| `$A1`, `A$1` | Partly anchored |
| `A1:B10` | A rectangular range |
| `$A$1:$B$10` | A range whose corners are anchored |

Anchors decide what happens when the formula moves: a **copy** shifts the relative parts and leaves
the anchored ones alone, while **inserting or deleting a row or column** moves every reference below
or after it regardless of anchors. [editing.md](editing.md) has the full table.

A range is only meaningful as a function argument — it is enumerated, not evaluated — so a cell
holding just `=A1:B2` reports `#VALUE!`.

## Literals

| Written | Is |
|---|---|
| `1200`, `1.5`, `2e3` | A number |
| `"North"`, `""` | Text; a quote inside is doubled, so `"say ""hi"""` is `say "hi"` |
| `TRUE`, `FALSE` | A boolean |
| `#N/A`, `#DIV/0!`, `#VALUE!`, `#REF!`, `#NAME?`, `#NUM!` | An error, usable as a value |

Text that looks like a number is coerced where a number is needed, so `=A1*2` works when `A1` holds
the text `"5"`. The exceptions are worth remembering: `ISNUMBER("5")` is `FALSE` because the cell
really does hold text, and `EXACT("5",5)` compares text with the display form of the number.

## Errors

| Error | Means |
|---|---|
| `#DIV/0!` | Division by zero, or `AVERAGE`/`STDEV`/`VAR` over nothing |
| `#VALUE!` | The wrong kind of argument, or the wrong number of them |
| `#REF!` | A reference that no longer exists — a deleted row, column or range corner |
| `#NAME?` | A function this engine does not implement |
| `#NUM!` | A number that cannot be produced, such as `SQRT(-1)` or `LOG(0)` |
| `#N/A` | A lookup found nothing |
| `#CIRCULAR!` | A formula that depends on itself |

Errors propagate: `=A1+1` is `#DIV/0!` when `A1` is, and an aggregate over a range containing an
error returns that error rather than skipping it. The exceptions are the deliberate ones — `IFERROR`,
`IFNA`, `ISERROR` and `ISNA` exist to look at an error instead of passing it on.

Cells showing `#CIRCULAR!` are not broken beyond repair: fix the formula that closes the loop and the
value returns.

## How arguments are read

- **A range argument is expanded** into its cells, in row-major order. `SUM(A1:B2)` adds four cells.
- **Aggregates skip text and blanks** rather than counting them as zero: `SUM` over a column of
  numbers and labels adds the numbers, and `AVERAGE` divides by how many numbers there were.
- **An empty cell counts as zero** in arithmetic, so `=A1+1` is `1` when `A1` is empty, and a numeric
  `COUNTIF` criterion such as `"=0"` matches an empty cell.
- **Errors are not skipped.** If a cell in the range is an error, the aggregate returns that error.
- **`SUMPRODUCT` is stricter than the rest**: it needs equally sized ranges, and reports `#VALUE!`
  rather than pairing up the wrong cells when they differ.

## Which functions are missing

Anything the engine does not implement evaluates to `#NAME?` and the formula text is kept exactly as
it was, so saving never loses a formula this app cannot compute. When you *import* a workbook, the
names it found are listed as warnings in the status bar, which is the quickest way to see what a real
file was relying on.

The function library covers the common ground rather than the long tail. There is no array or spill
semantics — every function works cell by cell — and no `DATE`, `TODAY` or `NOW`.
[functions.md](functions.md) lists what *is* there, and [limitations.md](limitations.md) explains the
gaps.

Next: [functions.md](functions.md) for every function with its arguments.
