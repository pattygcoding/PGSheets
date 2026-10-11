using System.Globalization;
using PGSheets.Core;

namespace PGSheets.Tests;

public class CellValueTests
{
    [Fact]
    public void Empty_is_the_default()
    {
        CellValue value = default;

        Assert.Equal(CellValueKind.Empty, value.Kind);
        Assert.True(value.IsEmpty);
        Assert.False(value.IsError);
        Assert.Equal(string.Empty, value.ToDisplayString());
    }

    [Fact]
    public void Text_of_null_or_empty_collapses_to_empty()
    {
        Assert.True(CellValue.FromText(null).IsEmpty);
        Assert.True(CellValue.FromText(string.Empty).IsEmpty);
    }

    [Theory]
    [InlineData(CellError.DivideByZero, "#DIV/0!")]
    [InlineData(CellError.Value, "#VALUE!")]
    [InlineData(CellError.Reference, "#REF!")]
    [InlineData(CellError.Name, "#NAME?")]
    [InlineData(CellError.Number, "#NUM!")]
    [InlineData(CellError.Circular, "#CIRCULAR!")]
    public void Errors_display_their_excel_code(CellError error, string expected)
    {
        var value = CellValue.FromError(error);

        Assert.True(value.IsError);
        Assert.Equal(expected, value.ToDisplayString());
    }

    [Fact]
    public void Booleans_display_uppercase()
    {
        Assert.Equal("TRUE", CellValue.FromBoolean(true).ToDisplayString());
        Assert.Equal("FALSE", CellValue.FromBoolean(false).ToDisplayString());
    }

    [Fact]
    public void Numbers_avoid_floating_point_noise()
    {
        var value = CellValue.FromNumber(0.1d + 0.2d);

        Assert.Equal("0.3", value.ToDisplayString());
    }

    [Fact]
    public void Numbers_format_with_the_invariant_culture()
    {
        var value = CellValue.FromNumber(1234.5d);

        Assert.Equal(1234.5d.ToString("G15", CultureInfo.InvariantCulture), value.ToDisplayString());
    }

    [Theory]
    [InlineData(CellValueKind.Empty, 0d)]
    [InlineData(CellValueKind.Boolean, 1d)]
    public void Try_get_number_coerces_empty_and_boolean(CellValueKind kind, double expected)
    {
        var value = kind == CellValueKind.Empty ? CellValue.Empty : CellValue.FromBoolean(true);

        Assert.True(value.TryGetNumber(out var number));
        Assert.Equal(expected, number, 9);
    }

    [Fact]
    public void Try_get_number_coerces_numeric_text()
    {
        Assert.True(CellValue.FromText("42.5").TryGetNumber(out var number));
        Assert.Equal(42.5d, number, 9);

        Assert.False(CellValue.FromText("North").TryGetNumber(out _));
    }

    [Fact]
    public void Try_get_number_rejects_errors()
    {
        Assert.False(CellValue.FromError(CellError.Value).TryGetNumber(out _));
    }

    [Fact]
    public void As_number_or_default_falls_back_to_zero()
    {
        Assert.Equal(0d, CellValue.FromText("North").AsNumberOrDefault(), 9);
        Assert.Equal(7d, CellValue.FromNumber(7d).AsNumberOrDefault(), 9);
    }

    [Fact]
    public void As_boolean_interprets_conditions()
    {
        Assert.True(CellValue.FromBoolean(true).AsBoolean());
        Assert.False(CellValue.FromBoolean(false).AsBoolean());
        Assert.True(CellValue.FromNumber(1d).AsBoolean());
        Assert.False(CellValue.FromNumber(0d).AsBoolean());
        Assert.True(CellValue.FromText("TRUE").AsBoolean());
        Assert.False(CellValue.Empty.AsBoolean());
    }

    [Fact]
    public void Equality_is_structural()
    {
        Assert.Equal(CellValue.FromNumber(3d), CellValue.FromNumber(3d));
        Assert.NotEqual(CellValue.FromNumber(3d), CellValue.FromNumber(4d));
        Assert.Equal(CellValue.FromText("a"), CellValue.FromText("a"));
        Assert.NotEqual(CellValue.FromText("a"), CellValue.FromText("b"));
        Assert.Equal(CellValue.FromText("a").GetHashCode(), CellValue.FromText("a").GetHashCode());
    }
}
