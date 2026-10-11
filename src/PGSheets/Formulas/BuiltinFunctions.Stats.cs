using PGSheets.Core;

namespace PGSheets.Formulas;

/// <summary>
/// Statistical functions. They share one rule: text and blanks in a range are skipped rather than
/// counted as zero, and an error anywhere in the range propagates.
/// </summary>
public static partial class BuiltinFunctions
{
    private static CellValue Median(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        var numbers = new List<double>();

        if (!TryCollectNumbers(arguments, context, numbers, out var error))
        {
            return error;
        }

        if (numbers.Count == 0)
        {
            return CellValue.FromError(CellError.Number);
        }

        numbers.Sort();

        var middle = numbers.Count / 2;

        return numbers.Count % 2 == 1
            ? CellValue.FromNumber(numbers[middle])
            : CellValue.FromNumber((numbers[middle - 1] + numbers[middle]) / 2d);
    }

    private static CellValue Large(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => NthExtreme(arguments, context, kthLargest: true);

    private static CellValue Small(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => NthExtreme(arguments, context, kthLargest: false);

    /// <summary>LARGE and SMALL take the k-th value counting in from one end of the sorted numbers.</summary>
    private static CellValue NthExtreme(
        IReadOnlyList<IFormulaNode> arguments,
        IFormulaContext context,
        bool kthLargest)
    {
        if (arguments.Count != 2)
        {
            return CellValue.FromError(CellError.Value);
        }

        var numbers = new List<double>();

        if (!TryCollectNumbers(new[] { arguments[0] }, context, numbers, out var error))
        {
            return error;
        }

        if (!TryInteger(arguments, 1, context, out var k, out var kError))
        {
            return kError;
        }

        if (numbers.Count == 0 || k < 1 || k > numbers.Count)
        {
            return CellValue.FromError(CellError.Number);
        }

        numbers.Sort();

        return CellValue.FromNumber(kthLargest ? numbers[^k] : numbers[k - 1]);
    }

    /// <summary>MODE is the most frequent value; the smallest wins a tie, as in a spreadsheet.</summary>
    private static CellValue Mode(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        var numbers = new List<double>();

        if (!TryCollectNumbers(arguments, context, numbers, out var error))
        {
            return error;
        }

        if (numbers.Count == 0)
        {
            return CellValue.FromError(CellError.NotAvailable);
        }

        numbers.Sort();

        var mostFrequent = double.NaN;
        var highestCount = 1;
        var index = 0;

        while (index < numbers.Count)
        {
            var run = 1;

            while (index + run < numbers.Count && numbers[index + run].Equals(numbers[index]))
            {
                run++;
            }

            // Strictly greater keeps the first, lowest, value when two run lengths tie.
            if (run > highestCount)
            {
                highestCount = run;
                mostFrequent = numbers[index];
            }

            index += run;
        }

        return highestCount > 1
            ? CellValue.FromNumber(mostFrequent)
            : CellValue.FromError(CellError.NotAvailable);
    }

    private static CellValue StDev(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (!TrySampleVariance(arguments, context, out var variance, out var error))
        {
            return error;
        }

        return CellValue.FromNumber(Math.Sqrt(variance));
    }

    private static CellValue Variance(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
        => TrySampleVariance(arguments, context, out var variance, out var error)
            ? CellValue.FromNumber(variance)
            : error;

    /// <summary>
    /// The sample variance, and with it STDEV. Two passes rather than the sum-of-squares shortcut,
    /// which loses most of its precision on values that are large and close together.
    /// </summary>
    private static bool TrySampleVariance(
        IReadOnlyList<IFormulaNode> arguments,
        IFormulaContext context,
        out double variance,
        out CellValue error)
    {
        variance = 0d;
        var numbers = new List<double>();

        if (!TryCollectNumbers(arguments, context, numbers, out error))
        {
            return false;
        }

        if (numbers.Count < 2)
        {
            error = CellValue.FromError(CellError.DivideByZero);
            return false;
        }

        var mean = 0d;

        foreach (var number in numbers)
        {
            mean += number;
        }

        mean /= numbers.Count;

        var sumOfSquares = 0d;

        foreach (var number in numbers)
        {
            var deviation = number - mean;
            sumOfSquares += deviation * deviation;
        }

        variance = sumOfSquares / (numbers.Count - 1);
        error = CellValue.Empty;
        return true;
    }

    /// <summary>The number of empty cells across the range.</summary>
    private static CellValue CountBlank(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count != 1)
        {
            return CellValue.FromError(CellError.Value);
        }

        var count = 0;

        foreach (var value in Expand(arguments[0], context))
        {
            if (value.IsEmpty)
            {
                count++;
            }
        }

        return CellValue.FromNumber(count);
    }

    /// <summary>RANK(number, range, [order]) — order 0 or omitted ranks the largest value first.</summary>
    private static CellValue Rank(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count is < 2 or > 3)
        {
            return CellValue.FromError(CellError.Value);
        }

        if (!TryNumber(arguments, 0, context, out var number, out var error))
        {
            return error;
        }

        var numbers = new List<double>();

        if (!TryCollectNumbers(new[] { arguments[1] }, context, numbers, out var listError))
        {
            return listError;
        }

        var ascending = false;

        if (arguments.Count == 3)
        {
            if (!TryNumber(arguments, 2, context, out var order, out var orderError))
            {
                return orderError;
            }

            ascending = order != 0d;
        }

        if (!numbers.Contains(number))
        {
            return CellValue.FromError(CellError.NotAvailable);
        }

        var ahead = 0;

        foreach (var candidate in numbers)
        {
            // Counting strictly better values gives tied entries the same rank.
            if (ascending ? candidate < number : candidate > number)
            {
                ahead++;
            }
        }

        return CellValue.FromNumber(ahead + 1);
    }

    /// <summary>
    /// SUMPRODUCT multiplies its arguments cell by cell and adds the products. Every argument must
    /// cover the same number of cells, and a non-numeric entry counts as zero.
    /// </summary>
    private static CellValue SumProduct(IReadOnlyList<IFormulaNode> arguments, IFormulaContext context)
    {
        if (arguments.Count == 0)
        {
            return CellValue.FromError(CellError.Value);
        }

        var arrays = new List<List<double>>();

        foreach (var argument in arguments)
        {
            var values = new List<double>();

            foreach (var value in Expand(argument, context))
            {
                if (value.IsError)
                {
                    return value;
                }

                values.Add(value.Kind == CellValueKind.Number ? value.Number : 0d);
            }

            arrays.Add(values);
        }

        var length = arrays[0].Count;

        foreach (var array in arrays)
        {
            if (array.Count != length)
            {
                // Mismatched shapes would otherwise pair up the wrong cells and quietly return a
                // number that means nothing.
                return CellValue.FromError(CellError.Value);
            }
        }

        var total = 0d;

        for (var index = 0; index < length; index++)
        {
            var product = 1d;

            foreach (var array in arrays)
            {
                product *= array[index];
            }

            total += product;
        }

        return CellValue.FromNumber(total);
    }
}
