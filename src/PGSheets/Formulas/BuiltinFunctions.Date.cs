using System.Globalization;
using PGSheets.Core;

namespace PGSheets.Formulas;

/// <summary>
/// Functions that read a date. A date is accepted either as a serial number, which is how a
/// spreadsheet stores one, or as the ISO text this app's importer produces for a date-formatted
/// cell — so these work on a real workbook today and keep working once a number format is what
/// turns a serial back into something that reads like a date.
/// </summary>
public static partial class BuiltinFunctions
{
    private enum DatePart
    {
        Year,
        Month,
        Day,
    }

    /// <summary>
    /// The shapes a date can arrive in as text. Deliberately exact rather than a general parse, so a
    /// cell holding "2024" is not silently read as a year.
    /// </summary>
    private static readonly string[] IsoDateFormats =
    {
        "yyyy-MM-dd",
        "yyyy-MM-dd HH:mm:ss",
        "yyyy-MM-ddTHH:mm:ss",
        "yyyy-MM-ddTHH:mm:ssZ",
    };

    private static CellValue Year(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => DatePartOf(arguments, context, DatePart.Year);

    private static CellValue Month(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => DatePartOf(arguments, context, DatePart.Month);

    private static CellValue Day(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => DatePartOf(arguments, context, DatePart.Day);

    private static CellValue DatePartOf(
        IReadOnlyList<IFormulaNode> arguments,
        IFormulaContext context,
        DatePart part)
    {
        if (arguments.Count != 1)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryDate(arguments[0], context, out var date, out var error))
        {
            return error;
        }

        return CellValue.FromNumber(part switch
        {
            DatePart.Year => date.Year,
            DatePart.Month => date.Month,
            _ => date.Day,
        });
    }

    /// <summary>WEEKDAY(date, [type]) — 1 to 7, with the type choosing where the week starts.</summary>
    private static CellValue WeekDay(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count is < 1 or > 2)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryDate(arguments[0], context, out var date, out var error))
        {
            return error;
        }

        var type = 1;

        if (arguments.Count == 2 && !TryInteger(arguments, 1, context, out type, out var typeError))
        {
            return typeError;
        }

        // DayOfWeek counts up from Sunday, which is what WEEKDAY's default type reports.
        var fromSunday = (int)date.DayOfWeek;

        return type switch
        {
            1 => CellValue.FromNumber(fromSunday + 1),
            2 => CellValue.FromNumber(((fromSunday + 6) % 7) + 1),
            3 => CellValue.FromNumber((fromSunday + 6) % 7),
            _ => CellValue.FromError(CellError.Number),
        };
    }

    /// <summary>
    /// Reads an argument as a date, from either a serial number or ISO text.
    /// </summary>
    private static bool TryDate(
        IFormulaNode node,
        IFormulaContext context,
        out DateTime date,
        out CellValue error)
    {
        date = default;

        var value = node.Evaluate(context);

        if (value.IsError)
        {
            error = value;
            return false;
        }

        if (value.Kind == CellValueKind.Text)
        {
            // A date-formatted cell from a real workbook arrives as text, so it has to be readable.
            if (DateTime.TryParseExact(
                    value.Text,
                    IsoDateFormats,
                    CultureInfo.InvariantCulture,
                    DateTimeStyles.None,
                    out date))
            {
                error = CellValue.Empty;
                return true;
            }

            error = CellValue.FromError(CellError.Value);
            return false;
        }

        if (!value.TryGetNumber(out var serial))
        {
            error = CellValue.FromError(CellError.Value);
            return false;
        }

        if (SerialDates.TryToDateTime(serial, out date))
        {
            error = CellValue.Empty;
            return true;
        }

        error = CellValue.FromError(CellError.Number);
        return false;
    }
}
