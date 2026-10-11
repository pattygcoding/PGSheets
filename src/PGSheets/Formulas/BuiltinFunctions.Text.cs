using System.Globalization;
using System.Text;
using PGSheets.Core;

namespace PGSheets.Formulas;

/// <summary>
/// Text functions. A numeric argument is coerced through its display form, so LEN(1234) is 4 and
/// LEFT(1234,2) is "12", which is what a spreadsheet does.
/// </summary>
public static partial class BuiltinFunctions
{
    /// <summary>Evaluates the argument at <paramref name="index"/> as text.</summary>
    private static bool TryText(
        IReadOnlyList<IFormulaNode> arguments,
        int index,
        IFormulaContext context,
        out string value,
        out CellValue error)
    {
        value = string.Empty;

        if (index >= arguments.Count)
        {
            error = CellValue.FromError(CellError.Value);
            return false;
        }

        var result = arguments[index].Evaluate(context);

        if (result.IsError)
        {
            error = result;
            return false;
        }

        value = TextOf(result);
        error = CellValue.Empty;
        return true;
    }

    private static CellValue Len(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count != 1)
        {
            return CellValue.FromError(CellError.Value);
        }

        return TryText(arguments, 0, context, out var text, out var error)
            ? CellValue.FromNumber(text.Length)
            : error;
    }

    private static CellValue Upper(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => Transform(arguments, context, text => text.ToUpperInvariant());

    private static CellValue Lower(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => Transform(arguments, context, text => text.ToLowerInvariant());

    private static CellValue Proper(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => Transform(arguments, context, ToProperCase);

    private static CellValue Trim(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => Transform(arguments, context, CollapseSpaces);

    /// <summary>Applies a one-argument text transformation.</summary>
    private static CellValue Transform(
        IReadOnlyList<IFormulaNode> arguments,
        IFormulaContext context,
        Func<string, string> transform)
    {
        if (arguments.Count != 1)
        {
            return CellValue.FromError(CellError.Value);
        }

        return TryText(arguments, 0, context, out var text, out var error)
            ? CellValue.FromText(transform(text))
            : error;
    }

    /// <summary>Capitalises the first letter of each word and lowers the rest.</summary>
    private static string ToProperCase(string text)
    {
        var builder = new StringBuilder(text.Length);
        var startOfWord = true;

        foreach (var ch in text)
        {
            if (char.IsLetter(ch))
            {
                builder.Append(startOfWord ? char.ToUpperInvariant(ch) : char.ToLowerInvariant(ch));
                startOfWord = false;
                continue;
            }

            builder.Append(ch);
            startOfWord = true;
        }

        return builder.ToString();
    }

    /// <summary>
    /// TRIM removes the ends and collapses runs of spaces inside to one, which is more than
    /// <see cref="string.Trim()"/> does and is what a spreadsheet offers.
    /// </summary>
    private static string CollapseSpaces(string text)
    {
        var builder = new StringBuilder(text.Length);
        var pendingSpace = false;

        foreach (var ch in text)
        {
            if (ch == ' ')
            {
                // Only worth remembering once something has been written.
                pendingSpace = builder.Length > 0;
                continue;
            }

            if (pendingSpace)
            {
                builder.Append(' ');
                pendingSpace = false;
            }

            builder.Append(ch);
        }

        return builder.ToString();
    }

    /// <summary>LEFT and RIGHT differ only in which end they take from.</summary>
    private static CellValue Left(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => TakeCharacters(arguments, context, fromEnd: false);

    private static CellValue Right(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => TakeCharacters(arguments, context, fromEnd: true);

    private static CellValue TakeCharacters(
        IReadOnlyList<IFormulaNode> arguments,
        IFormulaContext context,
        bool fromEnd)
    {
        if (arguments.Count is < 1 or > 2)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryText(arguments, 0, context, out var text, out var error))
        {
            return error;
        }

        var count = 1;

        if (arguments.Count == 2 && !TryInteger(arguments, 1, context, out count, out var countError))
        {
            return countError;
        }

        if (count < 0)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (count >= text.Length)
        {
            return CellValue.FromText(text);
        }

        return CellValue.FromText(fromEnd ? text[^count..] : text[..count]);
    }

    /// <summary>MID(text, start, length) — start is 1-based and must be at least 1.</summary>
    private static CellValue Mid(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count != 3)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryText(arguments, 0, context, out var text, out var error))
        {
            return error;
        }

        if (!TryInteger(arguments, 1, context, out var start, out var startError))
        {
            return startError;
        }

        if (!TryInteger(arguments, 2, context, out var length, out var lengthError))
        {
            return lengthError;
        }

        if (start < 1 || length < 0)
        {
            return CellValue.FromError(CellError.Value);
        }

        var from = start - 1;

        if (from >= text.Length)
        {
            // Asking past the end is empty text, not an error.
            return CellValue.FromText(string.Empty);
        }

        return CellValue.FromText(text.Substring(from, Math.Min(length, text.Length - from)));
    }

    private static CellValue Rept(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count != 2)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryText(arguments, 0, context, out var text, out var error))
        {
            return error;
        }

        if (!TryInteger(arguments, 1, context, out var count, out var countError))
        {
            return countError;
        }

        if (count < 0)
        {
            return CellValue.FromError(CellError.Value);
        }

        // Refuse to build a string no cell could hold rather than exhaust memory.
        if ((long)count * text.Length > MaxTextLength)
        {
            return CellValue.FromError(CellError.Value);
        }

        var builder = new StringBuilder(text.Length * count);

        for (var index = 0; index < count; index++)
        {
            builder.Append(text);
        }

        return CellValue.FromText(builder.ToString());
    }

    /// <summary>EXACT is the one text comparison that does care about case.</summary>
    private static CellValue Exact(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count != 2)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryText(arguments, 0, context, out var left, out var error))
        {
            return error;
        }

        if (!TryText(arguments, 1, context, out var right, out var rightError))
        {
            return rightError;
        }

        return CellValue.FromBoolean(string.Equals(left, right, StringComparison.Ordinal));
    }

    /// <summary>VALUE parses text into a number, refusing anything a spreadsheet would not accept.</summary>
    private static CellValue Value(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count != 1)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryText(arguments, 0, context, out var text, out var error))
        {
            return error;
        }

        return double.TryParse(
            text,
            NumberStyles.Float | NumberStyles.AllowThousands,
            CultureInfo.InvariantCulture,
            out var number)
            ? CellValue.FromNumber(number)
            : CellValue.FromError(CellError.Value);
    }

    /// <summary>CHAR turns a code in the 1..255 range into its character.</summary>
    private static CellValue Character(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count != 1)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryInteger(arguments, 0, context, out var code, out var error))
        {
            return error;
        }

        return code is >= 1 and <= 255
            ? CellValue.FromText(((char)code).ToString())
            : CellValue.FromError(CellError.Value);
    }

    /// <summary>CODE is the character code of the first character.</summary>
    private static CellValue Code(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count != 1)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryText(arguments, 0, context, out var text, out var error))
        {
            return error;
        }

        return text.Length == 0 ? CellValue.FromError(CellError.Value) : CellValue.FromNumber(text[0]);
    }

    /// <summary>FIND is case sensitive and 1-based, and reports a miss as #VALUE!.</summary>
    private static CellValue Find(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count is < 2 or > 3)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryText(arguments, 0, context, out var needle, out var error))
        {
            return error;
        }

        if (!TryText(arguments, 1, context, out var haystack, out var haystackError))
        {
            return haystackError;
        }

        var start = 1;

        if (arguments.Count == 3 && !TryInteger(arguments, 2, context, out start, out var startError))
        {
            return startError;
        }

        // One past the end is allowed, so FIND can report that nothing follows the text.
        if (start < 1 || start > haystack.Length + 1)
        {
            return CellValue.FromError(CellError.Value);
        }

        var found = haystack.IndexOf(needle, start - 1, StringComparison.Ordinal);

        return found < 0 ? CellValue.FromError(CellError.Value) : CellValue.FromNumber(found + 1);
    }

    /// <summary>SUBSTITUTE(text, old, new, [instance]) replaces all, or only the nth occurrence.</summary>
    private static CellValue Substitute(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count is < 3 or > 4)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryText(arguments, 0, context, out var text, out var error))
        {
            return error;
        }

        if (!TryText(arguments, 1, context, out var oldText, out error))
        {
            return error;
        }

        if (!TryText(arguments, 2, context, out var newText, out error))
        {
            return error;
        }

        if (oldText.Length == 0)
        {
            // Nothing to look for, so the text passes through unchanged.
            return CellValue.FromText(text);
        }

        if (arguments.Count == 3)
        {
            return CellValue.FromText(text.Replace(oldText, newText, StringComparison.Ordinal));
        }

        if (!TryInteger(arguments, 3, context, out var instance, out var instanceError))
        {
            return instanceError;
        }

        if (instance < 1)
        {
            return CellValue.FromError(CellError.Value);
        }

        return CellValue.FromText(ReplaceOccurrence(text, oldText, newText, instance));
    }

    /// <summary>Replaces the nth (1-based) occurrence only, leaving the others alone.</summary>
    private static string ReplaceOccurrence(string text, string oldText, string newText, int instance)
    {
        var index = -1;

        for (var found = 0; found < instance; found++)
        {
            index = text.IndexOf(oldText, index + 1, StringComparison.Ordinal);

            if (index < 0)
            {
                // Fewer occurrences than asked for: the text comes back unchanged.
                return text;
            }
        }

        return text[..index] + newText + text[(index + oldText.Length)..];
    }

    /// <summary>TEXTJOIN(delimiter, ignoreEmpty, value, ...) — the values may be ranges.</summary>
    private static CellValue TextJoin(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count < 3)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryText(arguments, 0, context, out var delimiter, out var error))
        {
            return error;
        }

        var ignoreEmptyValue = arguments[1].Evaluate(context);

        if (ignoreEmptyValue.IsError)
        {
            return ignoreEmptyValue;
        }

        var ignoreEmpty = ignoreEmptyValue.AsBoolean();
        var builder = new StringBuilder();
        var appended = false;

        for (var index = 2; index < arguments.Count; index++)
        {
            foreach (var value in Expand(arguments[index], context))
            {
                if (value.IsError)
                {
                    return value;
                }

                if (ignoreEmpty && value.IsEmpty)
                {
                    continue;
                }

                if (appended)
                {
                    builder.Append(delimiter);
                }

                builder.Append(TextOf(value));
                appended = true;
            }
        }

        return CellValue.FromText(builder.ToString());
    }
}
