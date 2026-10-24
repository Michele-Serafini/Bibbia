using Microsoft.UI.Xaml;
using Microsoft.UI.Windowing;
using System.Runtime.InteropServices;

namespace Bibbia.Windows;

public sealed partial class MainWindow
{
    [DllImport("dwmapi.dll")]
    private static extern int DwmSetWindowAttribute(IntPtr hwnd, int attribute, ref int value, int size);
    private void UpdateWindowTheme()
    {
        var dark = Root.ActualTheme == ElementTheme.Dark;
        var flag = dark ? 1 : 0;
        DwmSetWindowAttribute(WinRT.Interop.WindowNative.GetWindowHandle(this),20,ref flag,sizeof(int));
        if (AppWindowTitleBar.IsCustomizationSupported())
        {
            var background = EditorBackground();
            var foreground = dark ? Microsoft.UI.Colors.White : Microsoft.UI.Colors.Black;
            var title = AppWindow.TitleBar;
            title.BackgroundColor = title.InactiveBackgroundColor = background;
            title.ForegroundColor = title.InactiveForegroundColor = foreground;
            title.ButtonBackgroundColor = title.ButtonInactiveBackgroundColor = background;
            title.ButtonForegroundColor = title.ButtonInactiveForegroundColor = foreground;
            title.ButtonHoverBackgroundColor = dark ? global::Windows.UI.Color.FromArgb(255,60,60,60) : global::Windows.UI.Color.FromArgb(255,225,225,225);
            title.ButtonHoverForegroundColor = foreground;
        }
        foreach (var editor in liveEditors.Values) if (editor.DataContext is VerseRow row) ApplyTextStyles(editor,row);
    }
    private global::Windows.UI.Color EditorBackground() => Root.ActualTheme == ElementTheme.Dark
        ? global::Windows.UI.Color.FromArgb(255,32,32,32) : global::Windows.UI.Color.FromArgb(255,250,250,250);
}
