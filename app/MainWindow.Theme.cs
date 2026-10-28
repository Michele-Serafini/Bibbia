using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Windowing;
using System.Runtime.InteropServices;

namespace Bibbia.Windows;

public sealed partial class MainWindow
{
    [DllImport("dwmapi.dll")]
    private static extern int DwmSetWindowAttribute(IntPtr hwnd, int attribute, ref int value, int size);
    [DllImport("user32.dll")]
    private static extern bool ReleaseCapture();
    [DllImport("user32.dll",EntryPoint="SendMessageW")]
    private static extern IntPtr SendWindowMessage(IntPtr hwnd,uint message,IntPtr wParam,IntPtr lParam);
    [DllImport("user32.dll")]
    private static extern uint GetDoubleClickTime();
    [StructLayout(LayoutKind.Sequential)]
    private struct NativePoint { public int X,Y; }
    [DllImport("user32.dll")]
    private static extern bool GetCursorPos(out NativePoint point);
    private bool legacyCaption;
    private long titleClickTime;
    private global::Windows.Foundation.Point titleClickPoint;

    private void ConfigureWindowCaption()
    {
        // Windows 10/Server 2022 can accept the DWM dark-mode flag while still
        // painting a white system caption. Use themed WinUI chrome there.
        legacyCaption = !OperatingSystem.IsWindowsVersionAtLeast(10,0,22000) || !AppWindowTitleBar.IsCustomizationSupported();
        if (!legacyCaption || AppWindow.Presenter is not OverlappedPresenter presenter) return;
        presenter.SetBorderAndTitleBar(true,false);
        LegacyTitleBar.Visibility = Visibility.Visible;
        AppWindow.Changed += (_,_) => RefreshCaptionButtons();
        RefreshCaptionButtons();
    }
    private void RefreshCaptionButtons()
    {
        var maximized = AppWindow.Presenter is OverlappedPresenter {State:OverlappedPresenterState.Maximized};
        MaximizeGlyph.Glyph = maximized ? "\uE923" : "\uE922";
        void Label(Button button,string label) {
            AutomationProperties.SetName(button,T(label));
            ToolTipService.SetToolTip(button,T(label));
        }
        Label(MinimizeWindowButton,"Riduci a icona");
        Label(MaximizeWindowButton,maximized ? "Ripristina" : "Ingrandisci");
        Label(CloseWindowButton,"Chiudi");
    }
    private void MinimizeWindowClick(object sender,RoutedEventArgs e)
    {
        if (AppWindow.Presenter is OverlappedPresenter presenter) presenter.Minimize();
    }
    private void ToggleMaximize()
    {
        if (AppWindow.Presenter is not OverlappedPresenter presenter) return;
        if (presenter.State == OverlappedPresenterState.Maximized) presenter.Restore(); else presenter.Maximize();
    }
    private void MaximizeWindowClick(object sender,RoutedEventArgs e) => ToggleMaximize();
    private void CloseWindowClick(object sender,RoutedEventArgs e) => Close();
    private void TitleBarPointerPressed(object sender,PointerRoutedEventArgs e)
    {
        var point = e.GetCurrentPoint(LegacyTitleBar);
        if (!legacyCaption || !point.Properties.IsLeftButtonPressed) return;
        var now = Environment.TickCount64;
        e.Handled = true;
        if (now-titleClickTime <= GetDoubleClickTime() && Math.Abs(point.Position.X-titleClickPoint.X)<4 && Math.Abs(point.Position.Y-titleClickPoint.Y)<4) {
            titleClickTime = 0; ToggleMaximize(); return;
        }
        titleClickTime = now; titleClickPoint = point.Position;
        // Delegate moving/snapping to the window manager; keep its resize frame.
        ReleaseCapture();
        var position = GetCursorPos(out var cursor) ? new IntPtr(unchecked((cursor.Y<<16)|(cursor.X&0xffff))) : IntPtr.Zero;
        SendWindowMessage(WinRT.Interop.WindowNative.GetWindowHandle(this),0xA1,new IntPtr(2),position);
    }
    private global::Windows.UI.Color? appliedSurface;
    private void UpdateWindowTheme()
    {
        var background = WindowBackground();
        if (appliedSurface == background) return;
        appliedSurface = background;
        var dark = Root.ActualTheme == ElementTheme.Dark;
        var flag = dark ? 1 : 0;
        DwmSetWindowAttribute(WinRT.Interop.WindowNative.GetWindowHandle(this),20,ref flag,sizeof(int));
        LegacyTitleBar.Background = new SolidColorBrush(background);
        if (!legacyCaption && AppWindowTitleBar.IsCustomizationSupported())
        {
            var foreground = dark ? Microsoft.UI.Colors.White : Microsoft.UI.Colors.Black;
            var title = AppWindow.TitleBar;
            title.BackgroundColor = title.InactiveBackgroundColor = background;
            title.ForegroundColor = title.InactiveForegroundColor = foreground;
            title.ButtonBackgroundColor = title.ButtonInactiveBackgroundColor = background;
            title.ButtonForegroundColor = title.ButtonInactiveForegroundColor = foreground;
            title.ButtonHoverBackgroundColor = dark ? global::Windows.UI.Color.FromArgb(255,60,60,60) : global::Windows.UI.Color.FromArgb(255,225,225,225);
            title.ButtonHoverForegroundColor = foreground;
        }
        foreach (var editor in Descendants<RichTextBlock>(BookColumns)) if (editor.DataContext is VerseRow row) ApplyTextStylesCore(editor,row);
    }
    private global::Windows.UI.Color EditorBackground() => (BookViewport.Background as SolidColorBrush)?.Color ?? WindowBackground();
    private global::Windows.UI.Color WindowBackground() => (Root.Background as SolidColorBrush)?.Color ??
        (Root.ActualTheme == ElementTheme.Dark ? global::Windows.UI.Color.FromArgb(255,32,32,32) : global::Windows.UI.Color.FromArgb(255,248,248,248));
}
