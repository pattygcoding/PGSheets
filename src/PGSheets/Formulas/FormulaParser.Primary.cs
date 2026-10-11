using System.Globalization;
using System.Text;
using PGSheets.Core;

namespace PGSheets.Formulas;

public static partial class FormulaParser
{
    private sealed partial class Parser
    {
        private IFormulaNode ParsePrimary()
        {
            SkipWhitespace();

            if (_index >= _text.Length)
            {
                throw new FormulaException(CellError.Value, "Unexpected end of formula.");
            }

            var ch = _text[_index];

            if (ch == '(')
            {
                _index++;
                var inner = ParseComparison();
                SkipWhitespace();

                if (Peek() != ')')
                {
                    throw new FormulaException(CellError.Value, "Missing ')'.");
                }

                _index++;
                return inner;
            }

            if (ch == '"')
            {
                return ParseString();
            }

            if (ch == '#')
            {
                return ParseErrorLiteral();
            }

            if (ch == '$' || char.IsLetter(ch))
            {
                return ParseCellRefOrFunction();
            }

            if (char.IsDigit(ch) || ch == '.')
            {
                return ParseNumber();
            }

            throw new FormulaException(CellError.Value, $"Unexpected character '{ch}'.");
        }

        private IFormulaNode ParseNumber()
        {
            var start = _index;

            while (_index < _text.Length)
            {
                var ch = _text[_index];
                var isExponentSign = (ch == '+' || ch == '-') &&
                                     _index > start &&
                                     (_text[_index - 1] == 'e' || _text[_index - 1] == 'E');

                if (char.IsDigit(ch) || ch == '.' || ch == 'e' || ch == 'E' || isExponentSign)
                {
                    _index++;
                    continue;
                }

                break;
            }

            var token = _text[start.._index];

            if (!double.TryParse(token, NumberStyles.Float, CultureInfo.InvariantCulture, out var value))
            {
                throw new FormulaException(CellError.Value, $"Invalid number '{token}'.");
            }

            return new NumberNode(value);
        }

        private IFormulaNode ParseString()
        {
            _index++; // opening quote

            var builder = new StringBuilder();

            while (_index < _text.Length)
            {
                var ch = _text[_index];

                if (ch == '"')
                {
                    // "" is an escaped quote inside a string literal.
                    if (_index + 1 < _text.Length && _text[_index + 1] == '"')
                    {
                        builder.Append('"');
                        _index += 2;
                        continue;
                    }

                    _index++;
                    return new TextNode(builder.ToString());
                }

                builder.Append(ch);
                _index++;
            }

            throw new FormulaException(CellError.Value, "Unterminated string literal.");
        }

        private IFormulaNode ParseFunctionCall(string name)
        {
            _index++; // '('

            var arguments = new List<IFormulaNode>();
            SkipWhitespace();

            if (Peek() == ')')
            {
                _index++;
                return new FunctionNode(name, arguments.ToArray());
            }

            while (true)
            {
                arguments.Add(ParseComparison());
                SkipWhitespace();

                var ch = Peek();

                if (ch == ',')
                {
                    _index++;
                    SkipWhitespace();
                    continue;
                }

                if (ch == ')')
                {
                    _index++;
                    break;
                }

                throw new FormulaException(CellError.Value, $"Expected ',' or ')' in {name}().");
            }

            return new FunctionNode(name, arguments.ToArray());
        }
    }
}
