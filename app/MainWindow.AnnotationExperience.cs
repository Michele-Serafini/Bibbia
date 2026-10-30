using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Media;
using System.Text.Json;
namespace Bibbia.Windows;
public sealed partial class MainWindow {
    private readonly NavigationViewItem bookmarkNavigation=new(){Tag="bookmarks",Icon=new SymbolIcon(Symbol.Bookmarks)};
    private Microsoft.UI.Dispatching.DispatcherQueueTimer? annotationToastTimer;
    private bool bookmarkFilterFromShortcut;
    private void InitializeAnnotationExperience(){
        Navigation.MenuItems.Add(bookmarkNavigation);AutomationProperties.SetAutomationId(bookmarkNavigation,"BookmarksNavigation");
        Closed+=(_,_)=>annotationToastTimer?.Stop();
    }
    private void RelabelAnnotationExperience()=>bookmarkNavigation.Content=N("Segnalibri","Bookmarks");
    private void SelectBookmarkFilter(){
        bookmarkFilterFromShortcut=true;notesChanging=true;
        try{notesQuery.Text="";notesFolder.SelectedIndex=notesTag.SelectedIndex=0;notesKind.SelectedIndex=3;savedOffset=0;}finally{notesChanging=false;}
    }
    private void ShowBookmarks(){
        if(focusMode)SetFocus(false);
        if(ReferenceEquals(Navigation.SelectedItem,bookmarkNavigation))ShowSection("bookmarks");else Navigation.SelectedItem=bookmarkNavigation;
    }
    private void AnnotationFeedback(string text){
        if(closed)return;AnnotationToastText.Text=text;AnnotationToast.Visibility=Visibility.Visible;
        annotationToastTimer??=DispatcherQueue.CreateTimer();annotationToastTimer.Stop();annotationToastTimer.Interval=TimeSpan.FromSeconds(3);annotationToastTimer.IsRepeating=false;
        annotationToastTimer.Tick-=HideAnnotationFeedback;annotationToastTimer.Tick+=HideAnnotationFeedback;annotationToastTimer.Start();
    }
    private void HideAnnotationFeedback(Microsoft.UI.Dispatching.DispatcherQueueTimer timer,object args)=>AnnotationToast.Visibility=Visibility.Collapsed;
    private void UpdateAnnotationSummary(){
        if(books.Count<=book)return;
        var notes=readerRows.Count(r=>r.HasNote);var bookmarks=readerRows.Count(r=>r.HasBookmark);var marks=readerRows.Count(r=>r.HasMarks);
        ReadingLabel.Text=T("Capitolo {0} di {1}",chapter+1,books[book].Chapters)+(notes+bookmarks+marks==0?"":N($" · Note: {notes} · Segnalibri: {bookmarks} · Segni: {marks}",$" · Notes: {notes} · Bookmarks: {bookmarks} · Marks: {marks}"));
    }
    private void RefreshCurrentAnnotation(string targetEdition,int targetBook,int targetChapter,int verse,JsonElement annotation){
        if(edition!=targetEdition||book!=targetBook||chapter!=targetChapter||verse<0||verse>=readerRows.Count)return;
        readerRows[verse].Annotation=annotation;
        foreach(var editor in Descendants<RichTextBlock>(BookColumns))if(editor.DataContext is VerseRow row && row.Index==verse){row.Annotation=annotation;ApplyTextStylesCore(editor,row);}
        UpdateAnnotationSummary();ScheduleBookLayout();
    }
    private void BookmarkIndicatorClick(object sender,RoutedEventArgs e)=>Guard(()=>{
        if(sender is not Button{DataContext:VerseRow row} button)return;
        var targetEdition=edition;var targetBook=book;var targetChapter=chapter;
        var menu=new MenuFlyout();
        var open=new MenuFlyoutItem{Text=N("Mostra tutti i segnalibri","Show all bookmarks"),Icon=new SymbolIcon(Symbol.Bookmarks)};open.Click+=(_,_)=>ShowBookmarks();menu.Items.Add(open);
        var remove=new MenuFlyoutItem{Text=N("Rimuovi segnalibro","Remove bookmark"),Icon=new SymbolIcon(Symbol.Remove)};
        remove.Click+=(_,_)=>Guard(()=>{var a=BibleEngine.Call(new{op="delete_annotation",kind="bookmark",edition=targetEdition,book=targetBook,chapter=targetChapter,verse=row.Index});RefreshCurrentAnnotation(targetEdition,targetBook,targetChapter,row.Index,a);AnnotationFeedback(N("Segnalibro rimosso.","Bookmark removed."));});menu.Items.Add(remove);menu.ShowAt(button);
    });
    private void AddAnnotationDeletion(StackPanel content,SearchRow row,string targetEdition,Flyout popup){
        var actions=new StackPanel{Spacing=6};
        foreach(var (kind,label) in new[]{("note",N("Elimina solo la nota…","Delete note only…")),("all",N("Elimina tutti gli appunti del versetto…","Delete all annotations on this verse…"))}){
            if(kind=="note"&&row.Note.Length==0)continue;
            var button=new Button{Content=label,Foreground=new SolidColorBrush(Microsoft.UI.Colors.IndianRed),HorizontalAlignment=HorizontalAlignment.Stretch};
            AutomationProperties.SetAutomationId(button,kind=="note"?"DeleteNoteOnly":"DeleteAnnotation");
            button.Click+=async(_,_)=>{popup.Hide();await GuardAsync(async()=>{
                var dialog=new ContentDialog{XamlRoot=Root.XamlRoot,RequestedTheme=Root.RequestedTheme,Title=kind=="note"?N("Eliminare la nota?","Delete this note?"):N("Eliminare tutti gli appunti del versetto?","Delete every annotation on this verse?"),Content=row.Reference+"\n\n"+(kind=="note"?N("Verrà eliminato il testo della nota. Segnalibro, evidenziazioni e sottolineature saranno conservati.","The note text will be removed. The bookmark, highlights and underlines will be kept."):N("Verranno rimossi nota, segnalibro, segni, cartella ed etichette associati a questo versetto.","This removes the note, bookmark, marks, folder and tags assigned to this verse.")),PrimaryButtonText=N("Elimina","Delete"),CloseButtonText=T("Annulla"),DefaultButton=ContentDialogButton.Close};
                if(await dialog.ShowAsync()!=ContentDialogResult.Primary)return;
                var value=await Task.Run(()=>BibleEngine.Call(new{op="delete_annotation",kind,edition=targetEdition,book=row.Book,chapter=row.Chapter,verse=row.Verse}));
                if(closed)return;RefreshCurrentAnnotation(targetEdition,row.Book,row.Chapter,row.Verse,value);
                if(ResultList.Items.Count==1&&savedOffset>0)savedOffset-=200;LoadSaved();AnnotationFeedback(N("Appunti eliminati.","Annotations deleted."));
            });};actions.Children.Add(button);
        }
        content.Children.Add(actions);
    }
}
