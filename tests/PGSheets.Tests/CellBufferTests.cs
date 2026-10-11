using PGSheets.Core;

namespace PGSheets.Tests;

public class CellBufferTests
{
    [Fact]
    public void Keeps_the_shape_of_the_block()
    {
        var buffer = new CellBuffer();

        // A 2x2 block whose second column is empty must still cover two columns.
        buffer.SetRawInput(0, 0, "a");
        buffer.SetRawInput(0, 1, string.Empty);
        buffer.SetRawInput(1, 0, "c");
        buffer.SetRawInput(1, 1, string.Empty);

        Assert.True(buffer.TryGetBounds(out var bounds));
        Assert.Equal(new CellRange(0, 0, 1, 1), bounds);
        Assert.Equal(4, buffer.Count);
        Assert.Equal(2, buffer.NonEmptyCount);
    }

    [Fact]
    public void An_empty_buffer_has_no_bounds()
    {
        Assert.False(new CellBuffer().TryGetBounds(out _));
    }

    [Fact]
    public void Reads_back_what_it_was_given()
    {
        var buffer = new CellBuffer();
        buffer.SetRawInput(2, 3, "value");

        Assert.Equal("value", buffer.Get(2, 3));
        Assert.Equal(string.Empty, buffer.Get(0, 0));
    }

    [Fact]
    public void Bounds_span_the_outermost_fields()
    {
        var buffer = new CellBuffer();
        buffer.SetRawInput(5, 5, "a");
        buffer.SetRawInput(7, 9, "b");

        Assert.True(buffer.TryGetBounds(out var bounds));
        Assert.Equal(new CellRange(5, 5, 7, 9), bounds);
    }

    [Fact]
    public void A_null_value_is_stored_as_empty()
    {
        var buffer = new CellBuffer();
        buffer.SetRawInput(0, 0, null!);

        Assert.Equal(string.Empty, buffer.Get(0, 0));
    }
}
