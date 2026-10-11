namespace PGSheets.Core;

/// <summary>
/// Warning bookkeeping shared by the import and export results. Warnings are capped so a
/// pathological file (thousands of unsupported functions, say) cannot exhaust memory, while a
/// final entry still tells the reader that the list was truncated.
/// </summary>
internal static class CappedWarnings
{
    /// <summary>Message appended once the cap is reached.</summary>
    public const string SuppressedMessage = "Further warnings suppressed.";

    private const int MaxWarnings = 100;

    public static void Add(List<string> warnings, string message)
    {
        if (warnings.Count < MaxWarnings)
        {
            warnings.Add(message);
        }
        else if (warnings.Count == MaxWarnings)
        {
            warnings.Add(SuppressedMessage);
        }
    }
}
