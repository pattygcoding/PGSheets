using Avalonia.Controls;
using Avalonia.Interactivity;

namespace PGSheets.Views;

/// <summary>What the user chose when asked about unsaved changes.</summary>
public enum SavePromptResult
{
    /// <summary>Abandon the pending action. The default, so closing the dialog means "don't lose my work".</summary>
    Cancel,

    /// <summary>Proceed and discard the changes.</summary>
    Discard,

    /// <summary>Save first, then proceed.</summary>
    Save,
}

/// <summary>
/// A small modal question with Save / Don't Save / Cancel. Avalonia has no built-in message box, so
/// this is the application's one piece of prompting UI.
/// </summary>
public partial class SavePrompt : Window
{
    public SavePrompt()
    {
        InitializeComponent();
    }

    /// <summary>Shows the prompt centred on <paramref name="owner"/> and awaits the user's choice.</summary>
    public static async Task<SavePromptResult> ShowAsync(Window owner, string message)
    {
        var prompt = new SavePrompt();
        prompt.MessageText.Text = message;

        // Dismissing via the title bar yields default(SavePromptResult) == Cancel, which is the
        // safe outcome: nothing is discarded without an explicit click.
        return await prompt.ShowDialog<SavePromptResult>(owner);
    }

    private void OnSaveClicked(object? sender, RoutedEventArgs e) => Close(SavePromptResult.Save);

    private void OnDiscardClicked(object? sender, RoutedEventArgs e) => Close(SavePromptResult.Discard);

    private void OnCancelClicked(object? sender, RoutedEventArgs e) => Close(SavePromptResult.Cancel);
}
