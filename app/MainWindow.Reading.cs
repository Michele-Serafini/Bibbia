using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Automation;
using System.Text.Json;
namespace Bibbia.Windows;
public sealed partial class MainWindow {
    private readonly StackPanel readingPlansPanel=new(){Spacing=12};
    private readonly ComboBox readingPlanPicker=new(){HorizontalAlignment=HorizontalAlignment.Stretch,DisplayMemberPath="Title"};
    private CalendarView planCalendar=null!;
    private bool plansBuilt;
    private readonly TextBlock planSummary=new(){TextWrapping=TextWrapping.Wrap};
    private readonly ProgressBar planProgress=new(){Maximum=100};
    private readonly StackPanel planChapters=new(){Spacing=8};
    private DateTimeOffset planDay=DateTimeOffset.Now;
    private int readingGeneration;private bool planChanging;
    private sealed record PlanChoice(int Id,string Title);
    private void InitializeReadingPlans(){
        var nav=new NavigationViewItem{Content=N("Piani di lettura","Reading plans"),Tag="plans",Icon=new SymbolIcon(Symbol.Calendar)};
        AutomationProperties.SetAutomationId(nav,"ReadingPlansNavigation");Navigation.MenuItems.Add(nav);
        var host=new ScrollViewer{Content=readingPlansPanel,Visibility=Visibility.Collapsed};readingPlansHost=host;
        ((Grid)ReadPanel.Parent).Children.Add(host);
    }
    private void EnsureReadingPlans(){
        if(plansBuilt)return;plansBuilt=true;
        planCalendar=new CalendarView{SelectionMode=CalendarViewSelectionMode.Single,MinDate=new DateTimeOffset(1900,1,1,0,0,0,TimeSpan.Zero),MaxDate=new DateTimeOffset(2199,12,31,0,0,0,TimeSpan.Zero)};
        planCalendar.CalendarViewDayItemChanging+=(_,args)=>{var day=DateOnly.FromDateTime(args.Item.Date.DateTime);args.Item.SetDensityColors(completedPlanDays.Contains(day)?new[]{Microsoft.UI.Colors.ForestGreen}:Array.Empty<global::Windows.UI.Color>());};
        readingPlansPanel.Children.Add(readingPlanPicker);readingPlansPanel.Children.Add(planSummary);readingPlansPanel.Children.Add(planProgress);
        var actions=new StackPanel{Orientation=Orientation.Horizontal,Spacing=10};
        var resume=new Button{Content=N("Riprendi","Resume")};resume.Click+=(_,_)=>Guard(()=>{if(readingPlanPicker.SelectedItem is not PlanChoice p)return;OpenPlanPosition(BibleEngine.Call(new{op="reading",action="resume",id=p.Id}));});
        var create=new Button{Content=N("Nuovo piano…","New plan…")};create.Click+=(_,_)=>ShowNewPlan(create);
        var remove=new Button{Content=N("Elimina piano…","Delete plan…")};remove.Click+=async(_,_)=>await GuardAsync(async()=>{if(readingPlanPicker.SelectedItem is not PlanChoice p)return;var dialog=new ContentDialog{XamlRoot=Root.XamlRoot,Title=N("Eliminare il piano e i suoi progressi?","Delete this plan and its progress?"),PrimaryButtonText=N("Elimina","Delete"),CloseButtonText=T("Annulla"),DefaultButton=ContentDialogButton.Close};if(await dialog.ShowAsync()==ContentDialogResult.Primary){BibleEngine.Call(new{op="reading",action="delete",id=p.Id});await LoadReadingPlans();}});
        actions.Children.Add(resume);actions.Children.Add(create);actions.Children.Add(remove);readingPlansPanel.Children.Add(actions);
        AutomationProperties.SetAutomationId(planSummary,"PlanSummary");
        readingPlansPanel.Children.Add(planCalendar);readingPlansPanel.Children.Add(planChapters);
        readingPlansPanel.Children.Add(new TextBlock{Text=N("Il punto di lettura si salva automaticamente. Segna i capitoli letti per aggiornare i progressi.","Your reading position is saved automatically. Mark finished chapters to update progress."),TextWrapping=TextWrapping.Wrap});
        planCalendar.SelectedDates.Add(planDay);planCalendar.SelectedDatesChanged+=(_,_)=>{if(planChanging||planCalendar.SelectedDates.Count==0)return;planDay=planCalendar.SelectedDates[0];_=GuardAsync(LoadReadingDetail);};
        readingPlanPicker.SelectionChanged+=(_,_)=>{if(!planChanging)_=GuardAsync(LoadReadingDetail);};
        AutomationProperties.SetAutomationId(planCalendar,"PlanCalendar");AutomationProperties.SetAutomationId(resume,"ResumePlan");AutomationProperties.SetAutomationId(create,"NewPlan");
    }
    private ScrollViewer? readingPlansHost;
    private readonly HashSet<DateOnly> completedPlanDays=[];
    private async Task LoadReadingPlans(){
        EnsureReadingPlans();
        var old=(readingPlanPicker.SelectedItem as PlanChoice)?.Id;
        var rows=await Task.Run(()=>BibleEngine.Call(new{op="reading",action="list"}));if(closed||section!="plans")return;
        var choices=rows.EnumerateArray().Select(x=>new PlanChoice(x.GetProperty("id").GetInt32(),x.GetProperty("title").GetString()+" · "+x.GetProperty("edition").GetString())).ToList();
        planChanging=true;readingPlanPicker.ItemsSource=choices;readingPlanPicker.SelectedItem=choices.FirstOrDefault(x=>x.Id==old)??choices.FirstOrDefault();planChanging=false;await LoadReadingDetail();
    }
    private async Task LoadReadingDetail(){
        var generation=++readingGeneration;planChapters.Children.Clear();
        if(readingPlanPicker.SelectedItem is not PlanChoice p){completedPlanDays.Clear();foreach(var day in Descendants<CalendarViewDayItem>(planCalendar))day.SetDensityColors(Array.Empty<global::Windows.UI.Color>());planSummary.Text=N("Crea un piano per iniziare.","Create a plan to start.");planProgress.Value=0;return;}
        var date=planDay;var first=new DateTimeOffset(date.Year,date.Month,1,0,0,0,date.Offset);
        var detail=await Task.Run(()=>BibleEngine.Call(new{op="reading",action="detail",id=p.Id,from=first.ToString("yyyy-MM-dd"),selected=date.ToString("yyyy-MM-dd")}));if(closed||section!="plans"||generation!=readingGeneration)return;
        var total=detail.GetProperty("total").GetInt32();var done=detail.GetProperty("completed").GetInt32();planProgress.Value=100.0*done/Math.Max(1,total);
        planSummary.Text=$"{detail.GetProperty("title").GetString()} · {done}/{total} ({planProgress.Value:0}%) · {detail.GetProperty("duration").GetInt32()} "+N("giorni","days");
        completedPlanDays.Clear();
        foreach(var day in detail.GetProperty("days").EnumerateArray()){if(day.GetProperty("total").GetInt32()>0 && day.GetProperty("total").GetInt32()==day.GetProperty("done").GetInt32())completedPlanDays.Add(DateOnly.FromDateTime(first.AddDays(day.GetProperty("offset").GetInt32()).DateTime));}
        foreach(var item in Descendants<CalendarViewDayItem>(planCalendar))item.SetDensityColors(completedPlanDays.Contains(DateOnly.FromDateTime(item.Date.DateTime))?new[]{Microsoft.UI.Colors.ForestGreen}:Array.Empty<global::Windows.UI.Color>());
        foreach(var item in detail.GetProperty("chapters").EnumerateArray()){
            var b=item.GetProperty("book").GetInt32();var c=item.GetProperty("chapter").GetInt32();var index=item.GetProperty("index").GetInt32();var selectedEdition=detail.GetProperty("edition").GetString()!;
            var row=new StackPanel{Orientation=Orientation.Horizontal,Spacing=12};
            var check=new CheckBox{Content=$"{item.GetProperty("name").GetString()} {c+1}",IsChecked=item.GetProperty("complete").GetBoolean()};
            void Complete(bool value)=>_=GuardAsync(async()=>{await Task.Run(()=>BibleEngine.Call(new{op="reading",action="complete",id=p.Id,index,complete=value}));await LoadReadingDetail();});
            check.Checked+=(_,_)=>Complete(true);check.Unchecked+=(_,_)=>Complete(false);
            var open=new Button{Content=T("Leggi")};open.Click+=(_,_)=>Guard(()=>OpenPlanPosition(JsonSerializer.SerializeToElement(new{edition=selectedEdition,book=b,chapter=c,verse=0})));
            row.Children.Add(check);row.Children.Add(open);planChapters.Children.Add(row);
        }
        if(planChapters.Children.Count==0)planChapters.Children.Add(new TextBlock{Text=N("Nessuna lettura prevista per questo giorno.","No reading scheduled for this day.")});
    }
    private void OpenPlanPosition(JsonElement position){
        edition=position.GetProperty("edition").GetString()!;book=position.GetProperty("book").GetInt32();chapter=position.GetProperty("chapter").GetInt32();currentVerse=position.GetProperty("verse").GetInt32();
        // A plan may use a translation outside the current language filter.
        bibleLanguage="all";RefreshEditionPicker();LoadBooks();LoadChapter(currentVerse);Navigation.SelectedItem=Navigation.MenuItems[0];
    }
    private void ShowNewPlan(Button anchor){
        var popup=new Flyout();var box=new StackPanel{Width=320,Spacing=12};var title=new TextBox{Header=N("Titolo del piano","Plan title"),Text=N("La mia lettura","My reading"),MaxLength=100};
        var start=new DatePicker{Header=N("Data di inizio","Start date"),Date=DateTimeOffset.Now,MinYear=new DateTimeOffset(1900,1,1,0,0,0,TimeSpan.Zero),MaxYear=new DateTimeOffset(2199,12,31,0,0,0,TimeSpan.Zero)};
        var daily=new NumberBox{Header=N("Capitoli al giorno","Chapters per day"),Minimum=1,Maximum=12,Value=3,SmallChange=1,SpinButtonPlacementMode=NumberBoxSpinButtonPlacementMode.Compact};
        var scope=new CheckBox{Content=N("Solo il libro aperto","Current book only")};
        box.Children.Add(title);box.Children.Add(new TextBlock{Text=edition});box.Children.Add(scope);box.Children.Add(start);box.Children.Add(daily);
        var save=new Button{Content=N("Crea piano","Create plan")};AutomationProperties.SetAutomationId(save,"CreatePlan");save.Click+=async(_,_)=>await GuardAsync(async()=>{if(!double.IsFinite(daily.Value)||daily.Value!=Math.Truncate(daily.Value))throw new InvalidOperationException(N("Scegli un numero intero di capitoli.","Choose a whole number of chapters."));BibleEngine.Call(new{op="reading",action="create",edition,title=title.Text,start=start.Date.ToString("yyyy-MM-dd"),daily=(int)daily.Value,book=scope.IsChecked==true?(int?)book:null});popup.Hide();await LoadReadingPlans();});box.Children.Add(save);popup.Content=box;popup.ShowAt(anchor);
    }
}
