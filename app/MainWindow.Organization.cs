using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Automation;
using System.Text.Json;

namespace Bibbia.Windows;

public sealed partial class MainWindow {
    private readonly TextBox notesQuery=new();
    private readonly ComboBox notesFolder=new(),notesTag=new(),notesKind=new();
    private readonly Button notesManage=new();
    private readonly StackPanel notesBar=new(){Spacing=10};
    private string[] noteFolders=[],noteTags=[];
    private bool notesChanging;
    private int notesGeneration;
    private Microsoft.UI.Dispatching.DispatcherQueueTimer? notesTimer;
    private string N(string it,string en)=>language=="en"?en:it;
    private void InitializeOrganization() {
        var filters=new Grid{ColumnSpacing=8};
        for(var i=0;i<3;i++)filters.ColumnDefinitions.Add(new ColumnDefinition{Width=new GridLength(1,GridUnitType.Star)});
        filters.ColumnDefinitions.Add(new ColumnDefinition{Width=GridLength.Auto});
        notesBar.Children.Add(notesQuery);notesBar.Children.Add(filters);
        var controls=new FrameworkElement[]{notesFolder,notesTag,notesKind,notesManage};
        for(var i=0;i<controls.Length;i++){Grid.SetColumn(controls[i],i);filters.Children.Add(controls[i]);}
        foreach(var c in new[]{notesFolder,notesTag,notesKind})c.HorizontalAlignment=HorizontalAlignment.Stretch;
        notesBar.Visibility=Visibility.Collapsed;ResultsPanel.Children.Add(notesBar);
        AutomationProperties.SetAutomationId(notesQuery,"NotesQuery");
        AutomationProperties.SetAutomationId(notesFolder,"NotesFolder");AutomationProperties.SetAutomationId(notesTag,"NotesTag");AutomationProperties.SetAutomationId(notesKind,"NotesKind");
        AutomationProperties.SetAutomationId(notesManage,"ManageCollections");
        notesQuery.MaxLength=256;notesQuery.TextChanged+=(_,_)=>NotesFilterChanged();
        foreach(var combo in new[]{notesFolder,notesTag,notesKind})combo.SelectionChanged+=(_,_)=>NotesFilterChanged();
        notesManage.Click+=(_,_)=>ShowCollectionManager(notesManage);
        RelabelOrganization();
    }
    private void RelabelOrganization() {
        notesQuery.PlaceholderText=N("Cerca nel testo salvato e negli appunti…","Search saved text and notes…");
        notesManage.Content=N("Gestisci…","Manage…");
        var old=notesKind.SelectedIndex;notesChanging=true;
        notesKind.ItemsSource=new[]{N("Tutti gli appunti","All annotations"),N("Note","Notes"),N("Evidenziazioni e segni","Highlights and marks"),N("Segnalibri","Bookmarks")};
        notesKind.SelectedIndex=Math.Max(0,old);notesChanging=false;
        RefreshCollectionChoices();
    }
    private void RefreshCollectionChoices() {
        var folder=notesFolder.SelectedIndex>1?notesFolder.SelectedItem as string:null;
        var unfiled=notesFolder.SelectedIndex==1;
        var tag=notesTag.SelectedIndex>0?notesTag.SelectedItem as string:null;
        notesChanging=true;
        notesFolder.ItemsSource=new[]{N("Tutte le cartelle","All folders"),N("Senza cartella","Unfiled")}.Concat(noteFolders).ToArray();
        notesTag.ItemsSource=new[]{N("Tutte le etichette","All tags")}.Concat(noteTags).ToArray();
        notesFolder.SelectedIndex=unfiled?1:folder!=null && Array.IndexOf(noteFolders,folder)>=0?Array.IndexOf(noteFolders,folder)+2:0;
        notesTag.SelectedIndex=tag!=null && Array.IndexOf(noteTags,tag)>=0?Array.IndexOf(noteTags,tag)+1:0;
        notesChanging=false;
    }
    private void NotesFilterChanged() {
        if(!ready || notesChanging || section!="saved")return;
        notesGeneration++;savedOffset=0;
        notesTimer??=DispatcherQueue.CreateTimer();notesTimer.Stop();notesTimer.IsRepeating=false;notesTimer.Interval=TimeSpan.FromMilliseconds(250);
        notesTimer.Tick-=NotesFilterTick;notesTimer.Tick+=NotesFilterTick;notesTimer.Start();
    }
    private void NotesFilterTick(Microsoft.UI.Dispatching.DispatcherQueueTimer timer,object args)=>LoadSaved();
    private async Task LoadSavedNotes() {
        var generation=++notesGeneration;var selected=edition;var offset=savedOffset;
        var folder=notesFolder.SelectedIndex==1?"":notesFolder.SelectedIndex>1?notesFolder.SelectedItem as string:null;
        var tag=notesTag.SelectedIndex>0?notesTag.SelectedItem as string:null;
        var kind=new[]{"all","note","marked","bookmark"}[Math.Max(0,notesKind.SelectedIndex)];var query=notesQuery.Text;
        var result=await Task.Run(()=>{
            var groups=BibleEngine.Call(new{op="collections"});
            var items=BibleEngine.Call(new{op="saved",edition=selected,limit=200,offset,folder,tag,kind,query});
            return (groups,items);
        });
        if(closed || section!="saved" || edition!=selected || generation!=notesGeneration)return;
        noteFolders=result.groups.GetProperty("folders").EnumerateArray().Select(x=>x.GetString()!).ToArray();
        noteTags=result.groups.GetProperty("tags").EnumerateArray().Select(x=>x.GetString()!).ToArray();RefreshCollectionChoices();
        var rows=result.items.EnumerateArray().Select(x=>ToSearchRow(x,true)).ToList();ResultList.ItemsSource=rows;
        ResultStatus.Text=rows.Count==0?N($"Nessun appunto corrispondente · {edition}",$"No matching annotations · {edition}"):T("Appunti {0}–{1} · {2}",offset+1,offset+rows.Count,edition);
        SavedPreviousPage.IsEnabled=offset>0;SavedNextPage.IsEnabled=rows.Count==200;
    }
    private void OrganizeSavedClick(object sender,RoutedEventArgs e)=>Guard(()=>{
        if(sender is not Button{DataContext:SearchRow row} button)return;
        var selected=edition;var flyout=new Flyout();var content=new StackPanel{Width=310,Spacing=12};
        content.Children.Add(new TextBlock{Text=row.Reference,FontWeight=Microsoft.UI.Text.FontWeights.SemiBold});
        var folder=new ComboBox{Header=N("Cartella (vuota: senza cartella)","Folder (empty: unfiled)"),IsEditable=true,ItemsSource=noteFolders,Text=row.Folder,HorizontalAlignment=HorizontalAlignment.Stretch};
        var tags=new TextBox{Header=N("Etichette separate da virgole","Tags separated by commas"),Text=string.Join(", ",row.Tags),MaxLength=1000};
        AutomationProperties.SetAutomationId(folder,"OrganizeFolder");AutomationProperties.SetAutomationId(tags,"OrganizeTags");
        content.Children.Add(folder);content.Children.Add(tags);
        content.Children.Add(new TextBlock{Text=N("Puoi scrivere nomi nuovi. Le etichette possono essere più di una.","Type new names to create them. An annotation can have several tags."),TextWrapping=TextWrapping.Wrap,FontSize=12});
        var save=new Button{Content=T("Salva"),HorizontalAlignment=HorizontalAlignment.Right};AutomationProperties.SetAutomationId(save,"SaveOrganization");
        save.Click+=(_,_)=>Guard(()=>{
            BibleEngine.Call(new{op="organize",edition=selected,book=row.Book,chapter=row.Chapter,verse=row.Verse,folder=folder.Text,tags=tags.Text.Split(',',StringSplitOptions.TrimEntries|StringSplitOptions.RemoveEmptyEntries)});
            flyout.Hide();LoadSaved();
        });content.Children.Add(save);flyout.Content=content;flyout.ShowAt(button);
    });
    private void ShowCollectionManager(Button anchor)=>Guard(()=>{
        var flyout=new Flyout();var box=new StackPanel{Width=310,Spacing=12};
        var type=new ComboBox{ItemsSource=new[]{N("Cartelle","Folders"),N("Etichette","Tags")},SelectedIndex=0,HorizontalAlignment=HorizontalAlignment.Stretch};
        var existing=new ComboBox{Header=N("Raccolta esistente","Existing collection"),ItemsSource=noteFolders,HorizontalAlignment=HorizontalAlignment.Stretch};
        var name=new TextBox{Header=N("Nome nuovo","New name"),MaxLength=80};
        type.SelectionChanged+=(_,_)=>{existing.ItemsSource=type.SelectedIndex==0?noteFolders:noteTags;existing.SelectedIndex=-1;};
        box.Children.Add(type);box.Children.Add(existing);box.Children.Add(name);
        box.Children.Add(new TextBlock{Text=N("Rinominare o eliminare una raccolta conserva tutti gli appunti.","Renaming or deleting a collection keeps every annotation."),FontSize=12,TextWrapping=TextWrapping.Wrap});
        var buttons=new StackPanel{Orientation=Orientation.Horizontal,Spacing=8};
        foreach(var (action,it,en) in new[]{("create","Crea","Create"),("rename","Rinomina","Rename"),("delete","Elimina","Delete")}){
            var button=new Button{Content=N(it,en)};
            button.Click+=(_,_)=>Guard(()=>{
                var groups=BibleEngine.Call(new{op="collection",kind=type.SelectedIndex==0?"folder":"tag",action,name=action=="create"?name.Text:existing.SelectedItem as string ?? "",newName=name.Text});
                noteFolders=groups.GetProperty("folders").EnumerateArray().Select(x=>x.GetString()!).ToArray();noteTags=groups.GetProperty("tags").EnumerateArray().Select(x=>x.GetString()!).ToArray();
                flyout.Hide();RefreshCollectionChoices();LoadSaved();
            });buttons.Children.Add(button);
        }
        box.Children.Add(buttons);flyout.Content=box;flyout.ShowAt(anchor);
    });
}
