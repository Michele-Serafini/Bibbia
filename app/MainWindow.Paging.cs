using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Text;
using System.Text.Json;
using Windows.Foundation;
using Windows.System;

namespace Bibbia.Windows;

public sealed partial class MainWindow
{
    private sealed class ReaderPlace {
        public bool BookMode {get;set;}=true;
        public string Edition {get;set;}="";
        public int Book {get;set;}
        public int Chapter {get;set;}
        public int Verse {get;set;}
        public int Offset {get;set;}
    }
    private readonly record struct PageLine(double Y,double Height,int Verse,int Offset);
    private readonly List<PageLine> bookPages=[];
    private ReaderPlace readerPlace=new();
    private bool bookMode=true,bookSeek,bookLast,bookReflowing;
    private int bookPage,bookColumns=1,pageRequest;
    private string bookChapter="";
    private PageSnapshot? chapterSnapshot;
    private bool chapterForward;
    private Microsoft.UI.Dispatching.DispatcherQueueTimer? bookLayoutTimer;
    private ItemsPanelTemplate? scrollingPanel,fullChapterPanel;
    private static IEnumerable<T> Descendants<T>(DependencyObject root) where T:DependencyObject {
        for(var i=0;i<VisualTreeHelper.GetChildrenCount(root);i++) {
            var child=VisualTreeHelper.GetChild(root,i);
            if(child is T value) yield return value;
            foreach(var descendant in Descendants<T>(child)) yield return descendant;
        }
    }
    private void InitializeBookPagination() {
        try { if(!File.Exists(Path.Combine(dataDirectory,"pending-restore.sqlite3")))readerPlace=JsonSerializer.Deserialize<ReaderPlace>(File.ReadAllText(Path.Combine(dataDirectory,"reader.json")),
            new JsonSerializerOptions{PropertyNameCaseInsensitive=true})??new(); } catch(IOException){} catch(JsonException){}
        bookMode=readerPlace.BookMode; BookModeToggle.IsChecked=bookMode;
        VerseRight.ItemTemplate=VerseList.ItemTemplate;
        VerseRight.ItemContainerStyle=VerseList.ItemContainerStyle;
        scrollingPanel=VerseList.ItemsPanel;
        fullChapterPanel=(ItemsPanelTemplate)Microsoft.UI.Xaml.Markup.XamlReader.Load("<ItemsPanelTemplate xmlns='http://schemas.microsoft.com/winfx/2006/xaml/presentation'><StackPanel /></ItemsPanelTemplate>");
        BookStage.SizeChanged+=(_,_)=>ScheduleBookLayout();
        Root.AddHandler(UIElement.KeyDownEvent,new Microsoft.UI.Xaml.Input.KeyEventHandler((_,e)=>{
            if(bookMode && !comparing && section=="read" && e.Key is VirtualKey.PageDown or VirtualKey.PageUp){
                if(e.Key==VirtualKey.PageDown) NextClick(this,new RoutedEventArgs());else PreviousClick(this,new RoutedEventArgs());e.Handled=true;
            }
        }),true);
        var wheelTime=DateTime.MinValue;
        BookViewport.AddHandler(UIElement.PointerWheelChangedEvent,new Microsoft.UI.Xaml.Input.PointerEventHandler((_,e)=>{
            if(!bookMode)return;e.Handled=true;
            if((DateTime.UtcNow-wheelTime).TotalMilliseconds<350)return;wheelTime=DateTime.UtcNow;
            if(e.GetCurrentPoint(BookViewport).Properties.MouseWheelDelta<0) NextClick(this,new RoutedEventArgs());else PreviousClick(this,new RoutedEventArgs());
        }),true);
    }
    private void BookModeChanged(object sender,RoutedEventArgs e) {
        if(BookModeToggle==null)return;bookMode=BookModeToggle.IsChecked==true;
        if(!ready)return;CancelBookTurn();pageRequest++;ScheduleBookLayout(true);SaveBookPlace();
    }
    private void ScheduleBookLayout(bool seek=false) {
        bookSeek|=seek;
        if(!ready || closed || bookReflowing)return;
        bookLayoutTimer??=DispatcherQueue.CreateTimer();bookLayoutTimer.Stop();
        bookLayoutTimer.Interval=TimeSpan.FromMilliseconds(80);bookLayoutTimer.IsRepeating=false;
        bookLayoutTimer.Tick-=BookLayoutTick;bookLayoutTimer.Tick+=BookLayoutTick;bookLayoutTimer.Start();
    }
    private void BookLayoutTick(Microsoft.UI.Dispatching.DispatcherQueueTimer sender,object args)=>Guard(ReflowBook);
    private void ReflowBook() {
        if(comparing || section!="read" || bookReflowing || !ready || closed || BookViewport.ActualHeight<80)return;
        bookReflowing=true;
        try {
            CancelBookTurn();pageRequest++;
            bookColumns=bookMode && BookViewport.ActualWidth>=1040?2:1;
            BookColumns.ColumnDefinitions[1].Width=bookColumns==2?new GridLength(1,GridUnitType.Star):new GridLength(0);
            BookColumns.ColumnSpacing=bookColumns==2?24:0;
            VerseRight.Visibility=bookColumns==2?Visibility.Visible:Visibility.Collapsed;
            var panel=bookMode?fullChapterPanel:scrollingPanel;
            var panelChanged=VerseList.ItemsPanel!=panel;
            if(panelChanged){VerseList.ItemsPanel=panel;VerseRight.ItemsPanel=panel;}
            var height=Math.Max(40,BookViewport.ActualHeight-20);
            VerseList.Padding=VerseRight.Padding=bookMode?new Thickness(0,0,0,height):new Thickness(0);
            ScrollViewer.SetVerticalScrollBarVisibility(VerseList,bookMode?ScrollBarVisibility.Hidden:ScrollBarVisibility.Auto);
            ScrollViewer.SetVerticalScrollBarVisibility(VerseRight,ScrollBarVisibility.Hidden);
            VerseList.Clip=null;VerseRight.Clip=null;
            if(!ReferenceEquals(VerseList.ItemsSource,readerRows))VerseList.ItemsSource=readerRows;
            var rightRows=bookColumns==2?readerRows:null;
            if(!ReferenceEquals(VerseRight.ItemsSource,rightRows))VerseRight.ItemsSource=rightRows;
            BookColumns.UpdateLayout();
            if(!bookMode){BookPageLabel.Text="";return;}
            var scroll=Descendants<ScrollViewer>(VerseList).FirstOrDefault();if(scroll==null)return;
            height=Math.Max(40,scroll.ViewportHeight-6);
            if(panelChanged){DispatcherQueue.TryEnqueue(()=>ScheduleBookLayout());return;}
            var lines=new List<PageLine>();var readyReaders=0;
            foreach(var editor in Descendants<RichTextBlock>(VerseList)) {
                if(editor.DataContext is not VerseRow row)continue;
                var top=editor.TransformToVisual(VerseList).TransformPoint(new Point(0,0)).Y+scroll.VerticalOffset;
                var before=lines.Count;lines.AddRange(ReaderLines(editor,row,top));
                if(lines.Count>before)readyReaders++;
            }
            if(selectionDiagnostics){
                var first=Descendants<RichTextBlock>(VerseList).FirstOrDefault();
                TraceBook($"layout readers={Descendants<RichTextBlock>(VerseList).Count()} lines={lines.Count} viewport={height} first={(first==null?default(Rect):ReaderRect(first,0))} map={(first!=null && readerRuns.TryGetValue(first,out var map)?map.Count:0)}");
            }
            if(readyReaders<readerRows.Count){DispatcherQueue.TryEnqueue(()=>ScheduleBookLayout());return;}
            var key=$"{edition}/{book}/{chapter}";
            var anchorVerse=bookPages.Count>bookPage?bookPages[bookPage].Verse:currentVerse;
            var anchorOffset=bookPages.Count>bookPage?bookPages[bookPage].Offset:0;
            if(bookChapter!=key || bookSeek){anchorVerse=currentVerse;anchorOffset=0;
                if(bookChapter.Length==0 && readerPlace.Edition==edition && readerPlace.Book==book && readerPlace.Chapter==chapter && readerPlace.Verse==currentVerse)anchorOffset=readerPlace.Offset;
            }
            bookSeek=false;bookChapter=key;bookPages.Clear();bookPages.Add(new(0,0,0,0));double start=0;
            foreach(var line in lines.OrderBy(l=>l.Y)) {
                if(line.Y+line.Height>start+height && line.Y>start+1){bookPages.Add(line);start=line.Y;}
            }
            var found=bookPages.FindLastIndex(p=>p.Verse<anchorVerse || p.Verse==anchorVerse && p.Offset<=anchorOffset);
            bookPage=Math.Max(0,found)/bookColumns*bookColumns;
            if(bookLast){bookPage=(bookPages.Count-1)/bookColumns*bookColumns;bookLast=false;}
            ShowBookPages();
            if(chapterSnapshot is {} snapshot){chapterSnapshot=null;
                TurnBookPage(new PageSnapshot(snapshot.Bitmap,snapshot.Width,snapshot.Height,bookMotionGeneration),chapterForward);
            }
        } finally {bookReflowing=false;}
    }
    private void ShowBookPages() {
        var lists=new[]{VerseList,VerseRight};
        for(var n=0;n<2;n++) {
            var index=bookPage+n;var list=lists[n];
            list.Visibility=n<bookColumns && index<bookPages.Count?Visibility.Visible:Visibility.Collapsed;
            if(index>=bookPages.Count)continue;
            var scroll=Descendants<ScrollViewer>(list).FirstOrDefault();if(scroll==null)continue;
            var h=index+1<bookPages.Count?Math.Min(scroll.ViewportHeight,bookPages[index+1].Y-bookPages[index].Y):scroll.ViewportHeight;
            list.Clip=new RectangleGeometry{Rect=new Rect(0,0,list.ActualWidth,Math.Max(1,h))};
            scroll.ChangeView(null,bookPages[index].Y,null,true);
        }
        BookPageLabel.Text=$"{bookPage+1}{(bookColumns==2 && bookPage+1<bookPages.Count?$"–{bookPage+2}":"")} / {bookPages.Count}";
        PreviousButton.IsEnabled=bookPage>0 || book>0 || chapter>0;
        NextButton.IsEnabled=bookPage+bookColumns<bookPages.Count || book<books.Count-1 || chapter<books[book].Chapters-1;
        BookModeToggle.Content=T("Libro");
    }
    private bool TryTurnBookPage(int direction) {
        if(comparing || !bookMode || bookPages.Count==0)return false;
        var target=bookPage+direction*bookColumns;
        if(target<0 || target>=bookPages.Count){bookLast=direction<0;return false;}
        activeSelectionFlyout?.Hide();selectionTimer?.Stop();selectionEditor=null;
        bookPage=target;var request=++pageRequest;
        _=GuardAsync(async()=>{
            var image=await CaptureBookPage();if(request!=pageRequest || closed)return;
            ShowBookPages();SaveBookPlace();TurnBookPage(image,direction>0);
        });return true;
    }
    private void SaveBookPlace() {
        var page=bookPages.Count>bookPage?bookPages[bookPage]:new PageLine(0,0,currentVerse,0);
        readerPlace=new ReaderPlace{BookMode=bookMode,Edition=edition,Book=book,Chapter=chapter,Verse=page.Verse,Offset=page.Offset};
        if(bookMode){currentVerse=page.Verse;SavePosition();}
        var path=Path.Combine(dataDirectory,"reader.json");var temp=path+".tmp";
        File.WriteAllText(temp,JsonSerializer.Serialize(readerPlace,new JsonSerializerOptions{PropertyNamingPolicy=JsonNamingPolicy.CamelCase}));File.Move(temp,path,true);
    }
}
