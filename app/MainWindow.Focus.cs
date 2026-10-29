using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Automation;
using Windows.System;

namespace Bibbia.Windows;

public sealed partial class MainWindow
{
    private string readingTheme="Default";
    private int readingMargin=24;
    private bool focusMode,applyingFocus,focusApplied;
    private Microsoft.UI.Dispatching.DispatcherQueueTimer? marginTimer;
    private readonly global::Windows.UI.ViewManagement.AccessibilitySettings accessibilitySettings=new();
    private bool IsHighContrast=>accessibilitySettings.HighContrast;
    private void InitializeFocus()
    {
        var shortcut=new KeyboardAccelerator{Key=VirtualKey.F,Modifiers=VirtualKeyModifiers.Control|VirtualKeyModifiers.Shift};
        shortcut.Invoked+=(_,e)=>Guard(()=>{if(!ready)return;Navigation.SelectedItem=Navigation.MenuItems[0];SetFocus(!focusMode);e.Handled=true;});
        Root.KeyboardAccelerators.Add(shortcut);
        Root.KeyDown+=(_,e)=>{if(e.Key==VirtualKey.Escape && focusApplied && !e.Handled){SetFocus(false);e.Handled=true;}};
        accessibilitySettings.HighContrastChanged+=HighContrastChanged;
        Closed+=(_,_)=>{marginTimer?.Stop();accessibilitySettings.HighContrastChanged-=HighContrastChanged;};
    }
    private void HighContrastChanged(global::Windows.UI.ViewManagement.AccessibilitySettings sender,object args)=>DispatcherQueue.TryEnqueue(()=>{if(!closed){appliedSurface=null;UpdateWindowTheme();}});
    private void SetReadingTheme()
    {
        CancelBookTurn();
        Root.RequestedTheme=readingTheme=="Sepia"?ElementTheme.Light:Enum.TryParse<ElementTheme>(readingTheme,out var theme)?theme:ElementTheme.Default;
        appliedSurface=null;
        UpdateWindowTheme();
    }
    private void ApplyPaperPalette()
    {
        var sepia=readingTheme=="Sepia"&&!IsHighContrast;
        var dark=Root.ActualTheme==ElementTheme.Dark;
        var window=sepia?global::Windows.UI.Color.FromArgb(255,238,224,196):dark?global::Windows.UI.Color.FromArgb(255,32,32,32):global::Windows.UI.Color.FromArgb(255,248,248,248);
        var paper=sepia?global::Windows.UI.Color.FromArgb(255,244,231,205):dark?global::Windows.UI.Color.FromArgb(255,36,35,32):global::Windows.UI.Color.FromArgb(255,255,253,247);
        var edge=sepia?global::Windows.UI.Color.FromArgb(255,205,187,152):dark?global::Windows.UI.Color.FromArgb(255,72,70,63):global::Windows.UI.Color.FromArgb(255,221,216,203);
        if(IsHighContrast){var colors=new global::Windows.UI.ViewManagement.UISettings();window=colors.GetColorValue(global::Windows.UI.ViewManagement.UIColorType.Background);paper=window;edge=colors.GetColorValue(global::Windows.UI.ViewManagement.UIColorType.Foreground);}
        void Surface(DependencyObject element,DependencyProperty property,global::Windows.UI.Color color){if(element.GetValue(property) is SolidColorBrush brush && brush.Color==color)return;element.SetValue(property,new SolidColorBrush(color));}
        Surface(Root,Panel.BackgroundProperty,window);Surface(ReaderChrome,Panel.BackgroundProperty,window);
        Surface(Navigation,Control.BackgroundProperty,window);
        Navigation.Resources["NavigationViewDefaultPaneBackground"]=new SolidColorBrush(window);
        Navigation.Resources["NavigationViewContentBackground"]=new SolidColorBrush(window);
        Surface(BookViewport,Panel.BackgroundProperty,paper);Surface(FocusBar,Panel.BackgroundProperty,paper);
        foreach(var border in new[]{PaperBack,PaperFace,ComparisonPanel}){Surface(border,Border.BackgroundProperty,paper);Surface(border,Border.BorderBrushProperty,edge);}
        Surface(PaperEdge,Border.BackgroundProperty,edge);
    }
    private void FocusChanged(object sender,RoutedEventArgs e){if(ready&&!applyingFocus)Guard(()=>SetFocus(FocusToggle.IsChecked==true));}
    private void FocusSettingChanged(object sender,RoutedEventArgs e){if(ready&&!applyingFocus)Guard(()=>SetFocus(FocusSetting.IsOn));}
    private void FocusExitClick(object sender,RoutedEventArgs e)=>Guard(()=>SetFocus(false));
    private void SetFocus(bool value)
    {
        focusMode=value;CancelBookTurn();ApplyFocus();ScheduleBookLayout();
        BibleEngine.Call(new{op="settings",key="focusMode",value});
    }
    private void ApplyFocus()
    {
        applyingFocus=true;
        try {
            FocusToggle.IsChecked=FocusSetting.IsOn=focusMode;
            var active=focusMode&&section=="read";
            focusApplied=active;
            Navigation.IsPaneVisible=!active;Navigation.IsPaneToggleButtonVisible=!active;
            if(active)Navigation.IsPaneOpen=false;
            ReaderHeading.Visibility=ReaderControls.Visibility=ReaderFooter.Visibility=active?Visibility.Collapsed:Visibility.Visible;
            FocusBar.Visibility=active?Visibility.Visible:Visibility.Collapsed;
            // The CEI copyright remains available while concentrating.
            StatusLabel.Visibility=active&&edition!="CEI 2008"?Visibility.Collapsed:Visibility.Visible;
            ReaderChrome.RowSpacing=active?4:16;
        }finally{applyingFocus=false;}
    }
    private void MarginChanged(object sender,Microsoft.UI.Xaml.Controls.Primitives.RangeBaseValueChangedEventArgs e)
    {
        if(!ready||changing)return;
        readingMargin=(int)e.NewValue;CancelBookTurn();
        if(marginTimer==null){
            marginTimer=DispatcherQueue.CreateTimer();marginTimer.Interval=TimeSpan.FromMilliseconds(100);marginTimer.IsRepeating=false;
            marginTimer.Tick+=(_,_)=>Guard(()=>{if(closed)return;ApplyReadingMargins();ScheduleBookLayout();BibleEngine.Call(new{op="settings",key="readingMargin",value=readingMargin});});
        }
        marginTimer.Stop();marginTimer.Start();
    }
    private void ApplyReadingMargins()
    {
        foreach(var view in new[]{VerseList,VerseRight})view.Margin=new Thickness(Math.Max(0,readingMargin-12),8,Math.Max(0,readingMargin-12),8);
        ComparisonPanel.Padding=new Thickness(readingMargin);
    }
    private void RelabelFocus()
    {
        MarginSlider.Header=N("Margini della pagina","Page margins");
        FocusSetting.Header=N("Modalità concentrazione","Focus mode");
        foreach(var (element,label) in new (FrameworkElement,string)[]{(FocusToggle,N("Concentrazione · Ctrl+Shift+F","Focus · Ctrl+Shift+F")),(FocusPrevious,N("Pagina precedente","Previous page")),(FocusNext,N("Pagina successiva","Next page")),(FocusExit,N("Esci dalla concentrazione · Esc","Leave focus · Esc"))}){
            ToolTipService.SetToolTip(element,label);AutomationProperties.SetName(element,label);
        }
        foreach(var row in readerRows)row.NoteLabel=N("Apri nota","Open note");
    }
}
