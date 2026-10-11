using System.Globalization;
using PGSheets.Core;

namespace PGSheets.Formulas;

/// <summary>
/// The test a COUNTIF, SUMIF or AVERAGEIF applies to each cell. A criterion arrives as a value — a
/// number, or text such as "&gt;=10" or "&lt;&gt;closed" — so it is interpreted once and then tested
/// against every cell rather than being re-read each time.
/// </summary>
public static partial class BuiltinFunctions
{
    private enum CriteriaOperator
    {
        Equal,
        NotEqual,
        LessThan,
        LessThanOrEqual,
        GreaterThan,
        GreaterThanOrEqual,
    }

    private readonly struct Criteria
    {
        private readonly CriteriaOperator _operator;
        private readonly double _number;
        private readonly bool _isNumeric;
        private readonly string _text;

        private Criteria(CriteriaOperator @operator, double number, bool isNumeric, string text)
        {
            _operator = @operator;
            _number = number;
            _isNumeric = isNumeric;
            _text = text;
        }

        /// <summary>Reads a criterion: an optional comparison prefix, then a number or some text.</summary>
        public static Criteria Parse(CellValue value)
        {
            if (value.Kind != CellValueKind.Text)
            {
                // A bare number or boolean is an equality test.
                return value.TryGetNumber(out var direct)
                    ? new Criteria(CriteriaOperator.Equal, direct, isNumeric: true, string.Empty)
                    : new Criteria(CriteriaOperator.Equal, 0d, isNumeric: false, TextOf(value));
            }

            var text = value.Text ?? string.Empty;
            var @operator = CriteriaOperator.Equal;
            var operand = text;

            // Two-character operators have to be tested before their one-character prefixes.
            if (text.StartsWith("<>", StringComparison.Ordinal))
            {
                @operator = CriteriaOperator.NotEqual;
                operand = text[2..];
            }
            else if (text.StartsWith(">=", StringComparison.Ordinal))
            {
                @operator = CriteriaOperator.GreaterThanOrEqual;
                operand = text[2..];
            }
            else if (text.StartsWith("<=", StringComparison.Ordinal))
            {
                @operator = CriteriaOperator.LessThanOrEqual;
                operand = text[2..];
            }
            else if (text.StartsWith('='))
            {
                @operator = CriteriaOperator.Equal;
                operand = text[1..];
            }
            else if (text.StartsWith('>'))
            {
                @operator = CriteriaOperator.GreaterThan;
                operand = text[1..];
            }
            else if (text.StartsWith('<'))
            {
                @operator = CriteriaOperator.LessThan;
                operand = text[1..];
            }

            return double.TryParse(
                operand,
                NumberStyles.Float | NumberStyles.AllowThousands,
                CultureInfo.InvariantCulture,
                out var parsed)
                ? new Criteria(@operator, parsed, isNumeric: true, string.Empty)
                : new Criteria(@operator, 0d, isNumeric: false, operand);
        }

        public bool Matches(CellValue value)
        {
            if (value.IsError)
            {
                return false;
            }

            if (_isNumeric)
            {
                // An empty cell counts as zero here, which is what a spreadsheet does.
                return value.TryGetNumber(out var number) && Compare(number.CompareTo(_number));
            }

            if (_operator is CriteriaOperator.Equal or CriteriaOperator.NotEqual)
            {
                var equal = WildcardMatch(TextOf(value), _text);
                return _operator == CriteriaOperator.Equal ? equal : !equal;
            }

            return Compare(string.Compare(TextOf(value), _text, StringComparison.OrdinalIgnoreCase));
        }

        private bool Compare(int comparison) => _operator switch
        {
            CriteriaOperator.Equal => comparison == 0,
            CriteriaOperator.NotEqual => comparison != 0,
            CriteriaOperator.LessThan => comparison < 0,
            CriteriaOperator.LessThanOrEqual => comparison <= 0,
            CriteriaOperator.GreaterThan => comparison > 0,
            _ => comparison >= 0,
        };
    }
}
