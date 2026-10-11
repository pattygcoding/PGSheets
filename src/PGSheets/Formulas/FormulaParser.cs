using System.Globalization;
using System.Text;
using PGSheets.Core;

namespace PGSheets.Formulas;

/// <summary>
/// Recursive-descent parser for a pragmatic spreadsheet formula subset:
/// <c>=SUM(A1:A5)</c>, <c>=A1 + B2 * 3</c>, <c>=-A1 ^ 2</c>, string literals, comparisons via IF.
/// Parsing is only done on recalculation, never during rendering.
/// </summary>
public static partial class FormulaParser
{
    /// <summary>Parses <paramref name="formula"/> (including the leading '=').</summary>
    public static bool TryParse(string formula, out IFormulaNode node, out string? error)
    {
        var parser = new Parser(formula);
        try
        {
            node = parser.Parse();
            error = null;
            return true;
        }
        catch (FormulaException ex)
        {
            node = new ErrorNode(ex.Error);
            error = ex.Message;
            return false;
        }
    }

    private sealed partial class Parser
    {
        private readonly string _text;
        private int _index;

        public Parser(string text)
        {
            _text = text ?? string.Empty;
            _index = 0;
        }

        public IFormulaNode Parse()
        {
            SkipWhitespace();
            if (Peek() == '=')
            {
                _index++;
            }

            var node = ParseComparison();
            SkipWhitespace();

            if (_index < _text.Length)
            {
                throw new FormulaException(CellError.Value, $"Unexpected '{_text[_index]}' at {_index}.");
            }

            return node;
        }

        /// <summary>Lowest precedence level: =, &lt;&gt;, &lt;, &lt;=, &gt;, &gt;=.</summary>
        private IFormulaNode ParseComparison()
        {
            var left = ParseAdditive();

            while (true)
            {
                SkipWhitespace();
                var ch = Peek();
                ComparisonOperator? comparisonOperator = null;

                if (ch == '=')
                {
                    _index++;
                    comparisonOperator = ComparisonOperator.Equal;
                }
                else if (ch == '<')
                {
                    _index++;

                    if (Peek() == '>')
                    {
                        _index++;
                        comparisonOperator = ComparisonOperator.NotEqual;
                    }
                    else if (Peek() == '=')
                    {
                        _index++;
                        comparisonOperator = ComparisonOperator.LessThanOrEqual;
                    }
                    else
                    {
                        comparisonOperator = ComparisonOperator.LessThan;
                    }
                }
                else if (ch == '>')
                {
                    _index++;

                    if (Peek() == '=')
                    {
                        _index++;
                        comparisonOperator = ComparisonOperator.GreaterThanOrEqual;
                    }
                    else
                    {
                        comparisonOperator = ComparisonOperator.GreaterThan;
                    }
                }

                if (comparisonOperator is null)
                {
                    return left;
                }

                left = new ComparisonNode(comparisonOperator.Value, left, ParseAdditive());
            }
        }

        private IFormulaNode ParseAdditive()
        {
            var left = ParseMultiplicative();

            while (true)
            {
                SkipWhitespace();
                var ch = Peek();

                if (ch is '+' or '-')
                {
                    _index++;
                    var right = ParseMultiplicative();
                    left = new BinaryNode(
                        ch == '+' ? BinaryOperator.Add : BinaryOperator.Subtract,
                        left,
                        right);
                    continue;
                }

                return left;
            }
        }

        private IFormulaNode ParseMultiplicative()
        {
            var left = ParseUnary();

            while (true)
            {
                SkipWhitespace();
                var ch = Peek();

                if (ch == '*')
                {
                    _index++;
                    left = new BinaryNode(BinaryOperator.Multiply, left, ParseUnary());
                    continue;
                }

                if (ch == '/')
                {
                    _index++;
                    left = new BinaryNode(BinaryOperator.Divide, left, ParseUnary());
                    continue;
                }

                return left;
            }
        }

        private IFormulaNode ParseUnary()
        {
            SkipWhitespace();
            var ch = Peek();

            if (ch == '-')
            {
                _index++;
                return new UnaryNode(UnaryOperator.Negate, ParseUnary());
            }

            if (ch == '+')
            {
                _index++;
                return ParseUnary();
            }

            return ParsePower();
        }

        private IFormulaNode ParsePower()
        {
            var left = ParsePrimary();
            SkipWhitespace();

            if (Peek() == '^')
            {
                _index++;
                // Right associative, and the right operand may itself be signed: 2^-1.
                var right = ParseUnary();
                return new BinaryNode(BinaryOperator.Power, left, right);
            }

            return left;
        }

        private char Peek() => _index < _text.Length ? _text[_index] : '\0';

        private void SkipWhitespace()
        {
            while (_index < _text.Length && char.IsWhiteSpace(_text[_index]))
            {
                _index++;
            }
        }
    }
}
