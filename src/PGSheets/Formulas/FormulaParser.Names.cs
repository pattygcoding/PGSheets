using PGSheets.Core;

namespace PGSheets.Formulas;

public static partial class FormulaParser
{
    private sealed partial class Parser
    {
        private IFormulaNode ParseErrorLiteral()
        {
            // _text[_index] == '#'
            var start = _index;
            _index++;

            while (_index < _text.Length && IsErrorLiteralChar(_text[_index]))
            {
                _index++;
            }

            var token = _text[start.._index].ToUpperInvariant();

            return token switch
            {
                "#DIV/0!" => new ErrorNode(CellError.DivideByZero),
                "#VALUE!" => new ErrorNode(CellError.Value),
                "#REF!" => new ErrorNode(CellError.Reference),
                "#NAME?" => new ErrorNode(CellError.Name),
                "#NUM!" => new ErrorNode(CellError.Number),
                "#N/A" or "#N/A!" => new ErrorNode(CellError.NotAvailable),
                _ => throw new FormulaException(CellError.Name, $"Unknown error literal '{token}'."),
            };
        }

        private IFormulaNode ParseCellRefOrFunction()
        {
            var start = _index;

            while (_index < _text.Length && IsIdentifierChar(_text[_index]))
            {
                _index++;
            }

            var token = _text[start.._index];

            SkipWhitespace();

            if (Peek() == '(')
            {
                return ParseFunctionCall(token);
            }

            if (string.Equals(token, "TRUE", StringComparison.OrdinalIgnoreCase))
            {
                return new BooleanNode(true);
            }

            if (string.Equals(token, "FALSE", StringComparison.OrdinalIgnoreCase))
            {
                return new BooleanNode(false);
            }

            if (!CellAddress.TryParseA1(
                    token,
                    out var row,
                    out var column,
                    out var absoluteRow,
                    out var absoluteColumn))
            {
                throw new FormulaException(CellError.Name, $"Unknown name '{token}'.");
            }

            if (row >= SheetBounds.MaxRows || column >= SheetBounds.MaxColumns)
            {
                return new ErrorNode(CellError.Reference);
            }

            SkipWhitespace();

            if (Peek() != ':')
            {
                return new CellRefNode(row, column, absoluteRow, absoluteColumn);
            }

            _index++; // ':'
            SkipWhitespace();

            var rangeStart = _index;
            while (_index < _text.Length && IsIdentifierChar(_text[_index]))
            {
                _index++;
            }

            var rangeToken = _text[rangeStart.._index];

            if (!CellAddress.TryParseA1(
                    rangeToken,
                    out var row2,
                    out var column2,
                    out var absoluteRow2,
                    out var absoluteColumn2))
            {
                throw new FormulaException(CellError.Reference, $"Invalid range end '{rangeToken}'.");
            }

            if (row2 >= SheetBounds.MaxRows || column2 >= SheetBounds.MaxColumns)
            {
                throw new FormulaException(CellError.Reference, "Range exceeds the sheet bounds.");
            }

            return new RangeRefNode(
                row,
                column,
                row2,
                column2,
                absoluteRow,
                absoluteColumn,
                absoluteRow2,
                absoluteColumn2);
        }

        private static bool IsIdentifierChar(char ch) =>
            char.IsLetterOrDigit(ch) || ch == '$' || ch == '_' || ch == '.';

        private static bool IsErrorLiteralChar(char ch) =>
            char.IsLetterOrDigit(ch) || ch == '/' || ch == '!' || ch == '?';
    }
}
