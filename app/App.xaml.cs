using Microsoft.UI.Xaml;

namespace Bibbia.Windows;

public partial class App : Application
{
    private Window? window;
    // The installer checks this marker before replacing files. Multiple app
    // instances each retain a handle, so it disappears only after all close.
    private readonly Mutex lifetimeMarker = new(false, @"Local\BibbiaRust.App");
    public App() => InitializeComponent();
    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        window = new MainWindow();
        window.Closed += (_, _) => lifetimeMarker.Dispose();
        window.Activate();
    }
}
