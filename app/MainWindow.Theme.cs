using Microsoft.UI.Xaml;
using Microsoft.UI.Windowing;
using System.Runtime.InteropServices;

namespace Bibbia.Windows;

public sealed partial class MainWindow
{
    [DllImport("dwmapi.dll")]
    private static extern int DwmSetWindowAttribute(IntPtr hwnd, int attribute, ref int value, int size);
    [DllImport("uxtheme.dll",EntryPoint="#135")]
    private static extern int SetPreferredAppMode(int mode);
    [DllImport("uxtheme.dll",EntryPoint="#133")]
    [return:MarshalAs(UnmanagedType.I1)]
    private static extern bool AllowDarkModeForWindow(IntPtr hwnd,[MarshalAs(UnmanagedType.I1)] bool allow);
    [DllImport("uxtheme.dll",CharSet=CharSet.Unicode)]
    private static extern int SetWindowTheme(IntPtr hwnd,string? appName,string? subIdList);
    [DllImport("user32.dll")]
    private static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int width,int height,uint flags);
    private int nativeAppMode = -1;
    private bool? nativeDark;
    private void UpdateWindowTheme()
    {
        var dark = Root.ActualTheme == ElementTheme.Dark;
        var hwnd = WinRT.Interop.WindowNative.GetWindowHandle(this);
        // Older DWM caption rendering also requires the process/window to opt
        // into dark theming. These ordinals have this signature since 1903;
        // our minimum Windows version is 2004. Keep public DWM support if an
        // OS build does not export them.
        var mode = Root.RequestedTheme == ElementTheme.Default ? 1 : dark ? 2 : 3;
        if (mode != nativeAppMode || nativeDark != dark) {
            nativeAppMode = mode; nativeDark = dark;
            try {
                SetPreferredAppMode(mode);
                AllowDarkModeForWindow(hwnd,dark);
                SetWindowTheme(hwnd,dark ? "DarkMode_Explorer" : null,null);
            } catch (EntryPointNotFoundException) { }
            SetWindowPos(hwnd,IntPtr.Zero,0,0,0,0,0x37); // Preserve bounds/z-order; refresh the non-client frame.
        }
        var flag = dark ? 1 : 0;
        DwmSetWindowAttribute(hwnd,20,ref flag,sizeof(int));
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
