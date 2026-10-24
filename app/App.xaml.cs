using Microsoft.UI.Xaml;

namespace Bibbia.Windows;

public partial class App : Application
{
    private Window? window;
    // The installer checks this marker before replacing files. Multiple app
    // instances each retain a handle, so it disappears only after all close.
    private readonly Mutex lifetimeMarker = new(false, @"Local\BibbiaRust.App");
    public App()
    {
        InitializeComponent();
        UnhandledException += (_, e) =>
        {
            try
            {
                var directory = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "BibbiaRust");
                Directory.CreateDirectory(directory);
                File.WriteAllText(Path.Combine(directory, "last-error.log"), e.Exception.ToString());
            }
            catch { /* Preserve the original exception if diagnostic storage is unavailable. */ }
        };
    }
    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        window = new MainWindow();
        window.Closed += (_, _) => lifetimeMarker.Dispose();
        window.Activate();
    }
}
