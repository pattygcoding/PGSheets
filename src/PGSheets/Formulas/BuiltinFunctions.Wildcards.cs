namespace PGSheets.Formulas;

public static partial class BuiltinFunctions
{
    /// <summary>
    /// Matches text against a pattern using spreadsheet wildcards: <c>*</c> for any run of
    /// characters, <c>?</c> for exactly one, and <c>~</c> to escape the next character.
    /// </summary>
    /// <remarks>
    /// Two-pointer backtracking rather than a regular expression, so a COUNTIF over a large range
    /// compiles nothing and allocates nothing per cell.
    /// </remarks>
    private static bool WildcardMatch(string text, string pattern)
    {
        if (pattern.Length == 0)
        {
            return text.Length == 0;
        }

        var t = 0;
        var p = 0;
        var starIndex = -1;
        var starText = 0;

        while (t < text.Length)
        {
            // Try to consume one pattern token against one character.
            var probe = p;

            if (TokenMatches(pattern, ref probe, text[t]))
            {
                p = probe;
                t++;
                continue;
            }

            if (p < pattern.Length && pattern[p] == '*')
            {
                // Remember the star and try matching the rest with nothing under it first.
                starIndex = p;
                starText = t;
                p++;
                continue;
            }

            if (starIndex < 0)
            {
                return false;
            }

            // Let the last star swallow one more character and retry from just after it.
            starText++;
            t = starText;
            p = starIndex + 1;
        }

        // Trailing stars match nothing at all.
        while (p < pattern.Length && pattern[p] == '*')
        {
            p++;
        }

        return p == pattern.Length;
    }

    /// <summary>
    /// Consumes the pattern token at <paramref name="position"/> if it matches <paramref name="ch"/>.
    /// Stars are the caller's business because they match a variable number of characters.
    /// </summary>
    private static bool TokenMatches(string pattern, ref int position, char ch)
    {
        if (position >= pattern.Length)
        {
            return false;
        }

        var token = pattern[position];

        if (token == '~' && position + 1 < pattern.Length)
        {
            // An escaped character is matched literally, and consumes both pattern characters.
            if (!SameCharacter(ch, pattern[position + 1]))
            {
                return false;
            }

            position += 2;
            return true;
        }

        if (token == '*')
        {
            return false;
        }

        if (token == '?' || SameCharacter(ch, token))
        {
            position++;
            return true;
        }

        return false;
    }

    private static bool SameCharacter(char left, char right) => char.ToUpperInvariant(left) == char.ToUpperInvariant(right);
}
