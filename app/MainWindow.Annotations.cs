using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Controls.Primitives;
using System.Text.Json;
using Microsoft.UI.Text;
using System.Runtime.InteropServices;
using Windows.ApplicationModel.DataTransfer;

namespace Bibbia.Windows;

public sealed partial class MainWindow
{
    private readonly HashSet<RichTextBlock> liveEditors = [];
    private Microsoft.UI.Dispatching.DispatcherQueueTimer? selectionTimer;
    private RichTextBlock? selectionEditor;
    private CommandBarFlyout? activeSelectionFlyout;
    private static readonly bool selectionDiagnostics = Environment.GetEnvironmentVariable("BIBBIA_UI_DIAGNOSTICS") == "1";
    private void TraceSelection(string stage, RichTextBlock editor)
    {
        if (!selectionDiagnostics || editor.DataContext is not VerseRow {Index:0}) return;
        try {
            Directory.CreateDirectory(dataDirectory);
            var range = ReaderSelection(editor);
            File.AppendAllText(Path.Combine(dataDirectory,"ui-events.log"),$"{DateTime.UtcNow:O} {stage} start={range.Start} end={range.End} loaded={editor.IsLoaded} font={editor.FontSize} mouse={(GetAsyncKeyState(1)&0x8000)!=0}\n");
        } catch { }
    }
    [DllImport("user32.dll")]
    private static extern short GetAsyncKeyState(int key);
    private static global::Windows.UI.Color MarkColor(string? color) => color switch {
        "yellow"=>global::Windows.UI.Color.FromArgb(255,225,191,65),
        "green"=>global::Windows.UI.Color.FromArgb(255,151,207,145),
        "blue"=>global::Windows.UI.Color.FromArgb(255,142,192,237),
        _=>global::Windows.UI.Color.FromArgb(255,236,168,195)
    };
    private void VerseEditorLoaded(object sender, RoutedEventArgs e)
    {
        if (sender is RichTextBlock editor) {
            editor.RemoveHandler(UIElement.PointerReleasedEvent,new PointerEventHandler(VersePointerReleased));
            editor.AddHandler(UIElement.PointerReleasedEvent,new PointerEventHandler(VersePointerReleased),true);
            editor.RemoveHandler(UIElement.PointerPressedEvent,new PointerEventHandler(VersePointerPressed));
            editor.AddHandler(UIElement.PointerPressedEvent,new PointerEventHandler(VersePointerPressed),true);
        }
        if(sender is RichTextBlock view){view.SizeChanged-=VerseTextSizeChanged;view.SizeChanged+=VerseTextSizeChanged;}
        BindVerseEditor(sender);
    }
    private void VerseTextSizeChanged(object sender,SizeChangedEventArgs args){if(bookMode && !readerStyling)ScheduleBookLayout();}
    private void VerseEditorUnloaded(object sender,RoutedEventArgs e)
    {
        if (sender is not RichTextBlock editor) return;
        liveEditors.Remove(editor);
        if (ReferenceEquals(selectionEditor,editor)) { selectionTimer?.Stop(); selectionEditor=null; }
        editor.ContextFlyout=null; editor.Tag=null;readerRuns.Remove(editor);
    }
    private void VerseEditorDataContextChanged(FrameworkElement sender, DataContextChangedEventArgs e) => BindVerseEditor(sender);
    private void BindVerseEditor(object sender)
    {
        if (sender is not RichTextBlock editor || !editor.IsLoaded || editor.DataContext is not VerseRow row) return;
        liveEditors.Add(editor);
        if (ReferenceEquals(editor.Tag,row)) return;
        editor.Tag = row;
        editor.FontSize=row.TextSize;editor.IsTabStop=true;
        ApplyTextStylesCore(editor,row);
        AutomationProperties.SetAutomationId(editor,$"VerseText_{row.Index}");
        AutomationProperties.SetName(editor,row.Text);
        // The text control dismisses its own SelectionFlyout on focus loss.
        // Own the annotation popup separately so its buttons can receive focus.
        editor.SelectionFlyout = null;
        editor.ContextFlyout = AnnotationToolbar(editor,row,true,true);
        TraceSelection("bound",editor);
    }
    private void VersePointerPressed(object sender, PointerRoutedEventArgs e)
    {
        if (sender is RichTextBlock editor) TraceSelection("pointer pressed",editor);
    }
    private void VersePointerReleased(object sender, PointerRoutedEventArgs e)
    {
        if (sender is not RichTextBlock editor) return;
        TraceSelection("pointer released",editor);
        var point = e.GetCurrentPoint(editor).Position;
        // Native text selection finishes while handling the mouse release.
        DispatcherQueue.TryEnqueue(() => ShowSelectionToolbar(editor,point));
    }
    private void VerseSelectionChanged(object sender, RoutedEventArgs e)
    {
        if (sender is not RichTextBlock editor || bookReflowing || readerStyling) return;
        TraceSelection("selection changed",editor);
        if ((GetAsyncKeyState(1) & 0x8000) == 0) return;
        selectionEditor = editor;
        // Text selection can consume routed pointer releases. Observe the native
        // selection too, and wait for the drag to finish before opening a menu.
        if (selectionTimer == null) {
            selectionTimer = DispatcherQueue.CreateTimer();
            selectionTimer.Interval = TimeSpan.FromMilliseconds(40);
            selectionTimer.Tick += (_,_) => {
                if ((GetAsyncKeyState(1) & 0x8000) != 0) return;
                selectionTimer.Stop();
                if (selectionEditor is { } target) ShowSelectionToolbar(target);
                selectionEditor = null;
            };
        }
        selectionTimer.Start();
    }
    private void ShowSelectionToolbar(RichTextBlock editor, global::Windows.Foundation.Point? point = null) => Guard(() =>
    {
        TraceSelection("show requested",editor);
        if (closed || !editor.IsLoaded || ReaderSelection(editor).Start==ReaderSelection(editor).End || activeSelectionFlyout?.IsOpen == true || editor.DataContext is not VerseRow row) return;
        selectionTimer?.Stop();
        var flyout = AnnotationToolbar(editor,row,true);
        activeSelectionFlyout = flyout;
        flyout.Closed += (_,_) => { if (ReferenceEquals(activeSelectionFlyout,flyout)) activeSelectionFlyout = null; TraceSelection("popup closed",editor); };
        var options = new FlyoutShowOptions {Placement=FlyoutPlacementMode.Top,ShowMode=FlyoutShowMode.Transient};
        if (point is { } position) options.Position = position;
        flyout.ShowAt(editor,options);
        TraceSelection("show complete",editor);
    });
    private void ApplyTextStyles(RichTextBlock editor, VerseRow row) {
        foreach(var view in Descendants<RichTextBlock>(BookColumns)) if(view.DataContext is VerseRow value && value.Index==row.Index) {
            value.Annotation=row.Annotation;ApplyTextStylesCore(view,value);
        }
    }
    private CommandBarFlyout AnnotationToolbar(RichTextBlock editor, VerseRow row, bool selection, bool lazy=false)
    {
        var flyout = new CommandBarFlyout();
        var start = 0; var length = row.Text.Length; string? builtLanguage=null;
        flyout.Opening += (_,_) => {
            if (builtLanguage != language) { flyout.PrimaryCommands.Clear(); flyout.SecondaryCommands.Clear(); BuildCommands(); builtLanguage=language; }
            if (selection) {
                var range=ReaderSelection(editor);
                start=Math.Clamp(range.Start,0,row.Text.Length);
                var end=Math.Clamp(range.End,start,row.Text.Length);
                length = end-start;
                if (length == 0) { start = 0; length = row.Text.Length; }
            }
            currentVerse = row.Index;
            Guard(SavePosition);
        };
        void BuildCommands() {
        AppBarButton Button(string label, Symbol symbol, string id)
        {
            var b = new AppBarButton { Label=T(label), Icon=new SymbolIcon(symbol) };
            AutomationProperties.SetAutomationId(b,id);
            ToolTipService.SetToolTip(b,T(label)); return b;
        }
        void Mark(string action) => Guard(() => {
            row.Annotation = BibleEngine.Call(new {op="mark",edition,book,chapter,verse=row.Index,start,length,action});
            ApplyTextStyles(editor,row); flyout.Hide();
        });
        var underline = Button("Sottolinea",Symbol.Underline,"UnderlineSelection");
        underline.Click += (_,_) => Mark("underline");
        flyout.PrimaryCommands.Add(underline);
        var highlight = Button("Evidenzia",Symbol.Highlight,"HighlightSelection");
        var colors = new MenuFlyout();
        foreach (var (key,label) in new[]{("yellow","Giallo"),("green","Verde"),("blue","Blu"),("pink","Rosa")}) {
            var item = new MenuFlyoutItem { Text=T(label), Icon=new FontIcon {Glyph="●",Foreground=new SolidColorBrush(MarkColor(key))} };
            item.Click += (_,_) => Mark(key); colors.Items.Add(item);
        }
        highlight.Flyout = colors; flyout.PrimaryCommands.Add(highlight);
        var bookmark = Button("Segnalibro",Symbol.Bookmarks,"BookmarkVerse");
        bookmark.Click += (_,_) => Guard(() => {
            var a = row.Annotation;
            row.Annotation = BibleEngine.Call(new {op="annotate",edition,book,chapter,verse=row.Index,annotation=new {
                highlight=a.GetProperty("highlight"),underline=a.GetProperty("underline"),bookmark=!a.GetProperty("bookmark").GetBoolean(),note=a.GetProperty("note"),ranges=a.GetProperty("ranges")}});
            flyout.Hide();
        }); flyout.PrimaryCommands.Add(bookmark);
        var copy = Button("Copia",Symbol.Copy,"CopySelection");
        copy.Click += (_,_) => Guard(() => {
            var data = new DataPackage(); data.SetText(row.Text.Substring(start,length)); Clipboard.SetContent(data); flyout.Hide();
        }); flyout.PrimaryCommands.Add(copy);
        var note = Button("Nota",Symbol.Edit,"NoteVerse");
        note.Click += (_,_) => { flyout.Hide(); ShowNote(editor,row); };
        flyout.PrimaryCommands.Add(note);
        var clear = Button("Rimuovi formattazione",Symbol.Clear,"ClearSelection");
        clear.Click += (_,_) => Mark("clear"); flyout.SecondaryCommands.Add(clear);
        }
        if (!lazy) { BuildCommands(); builtLanguage=language; }
        return flyout;
    }
    private void VerseMenuClick(object sender, RoutedEventArgs e)
    {
        if (sender is not Button button || button.Parent is not Grid parent || parent.Children.OfType<RichTextBlock>().FirstOrDefault() is not { } editor || editor.DataContext is not VerseRow row) return;
        AnnotationToolbar(editor,row,false).ShowAt(button);
    }
    private void ShowNote(RichTextBlock editor, VerseRow row)
    {
        var flyout = new Flyout();
        var content = new StackPanel { Width=300,Spacing=12 };
        content.Children.Add(new TextBlock {Text=$"{books[book].Name} {chapter+1}:{row.Number}",FontWeight=Microsoft.UI.Text.FontWeights.SemiBold});
        var input = new TextBox {Header=T("La tua nota"),Text=row.Annotation.GetProperty("note").GetString(),AcceptsReturn=true,TextWrapping=TextWrapping.Wrap,Height=130,MaxLength=8000};
        AutomationProperties.SetAutomationId(input,"NoteText"); content.Children.Add(input);
        var save = new Button {Content=T("Salva"),HorizontalAlignment=HorizontalAlignment.Right};
        AutomationProperties.SetAutomationId(save,"SaveNote");
        save.Click += (_,_) => Guard(() => {
            var a=row.Annotation;
            row.Annotation=BibleEngine.Call(new {op="annotate",edition,book,chapter,verse=row.Index,annotation=new {
                highlight=a.GetProperty("highlight"),underline=a.GetProperty("underline"),bookmark=a.GetProperty("bookmark"),note=input.Text,ranges=a.GetProperty("ranges")}});
            flyout.Hide();
        }); content.Children.Add(save); flyout.Content=content;
        DispatcherQueue.TryEnqueue(() => flyout.ShowAt(editor));
    }
}
