using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.Text.Json;

namespace Bibbia.Windows;

public sealed record ComparedPosition(string Edition,int Book,int Chapter,int Verse);
public sealed class ComparisonRowView {
    public int Index {get;init;}
    public int LastVerse {get;init;}
    public string LeftReference {get;init;}="";
    public string RightReference {get;init;}="";
    public string LeftText {get;init;}="";
    public string RightText {get;init;}="";
    public double TextSize {get;init;}
    public ComparedPosition? LeftPosition {get;init;}
    public ComparedPosition? RightPosition {get;init;}
    public bool RightAvailable=>RightPosition!=null;
    public string OpenLabel {get;init;}="";
}
public sealed partial class MainWindow {
    private bool comparing,comparisonChanging;
    private int comparisonGeneration;
    private string comparisonEdition="CEI 2008",comparisonKey="";
    private JsonElement? comparisonData;
    private void ComparisonChanged(object sender,RoutedEventArgs e) => Guard(()=>{
        if(ComparisonToggle==null)return;
        comparing=ComparisonToggle.IsChecked==true;comparisonGeneration++;CancelBookTurn();
        if(!ready)return;
        BookStage.Visibility=comparing?Visibility.Collapsed:Visibility.Visible;
        ComparisonPanel.Visibility=comparing?Visibility.Visible:Visibility.Collapsed;
        BookModeToggle.IsEnabled=!comparing;
        if(comparing){
            chapterSnapshot=null;chapterRenderGeneration++;
            VerseList.ItemsSource=null;VerseRight.ItemsSource=null;
            if(measurementEditor!=null)UpdateDocument(measurementEditor,()=>measurementEditor.Document.SetText(Microsoft.UI.Text.TextSetOptions.None,""));
            BookPageLabel.Text="";LoadComparison();
        }
        else {
            ComparisonList.ItemsSource=null;comparisonData=null;comparisonKey="";
            // Ordinary catalog reads release imported editions outside the
            // active reader, including the now hidden comparison partner.
            BibleEngine.Call(new{op="books",edition});
            ScrollToCurrentVerse();
        }
    });
    private void ComparisonEditionChanged(object sender,SelectionChangedEventArgs e) {
        if(!ready || comparisonChanging || ComparisonEdition.SelectedItem is not EditionOption option)return;
        comparisonEdition=option.Id;
        Guard(()=>{BibleEngine.Call(new{op="settings",key="comparisonEdition",value=comparisonEdition});LoadComparison();});
    }
    private void LoadComparison() => _=GuardAsync(async()=>{
        if(!comparing || !ready || closed)return;
        comparisonChanging=true;
        try {
            var options=editionOptions.Where(x=>x.Id!=edition).ToList();
            if(!options.Any(x=>x.Id==comparisonEdition))comparisonEdition=options.Any(x=>x.Id=="CEI 2008")?"CEI 2008":options[0].Id;
            ComparisonEdition.ItemsSource=options;
            ComparisonEdition.SelectedItem=options.First(x=>x.Id==comparisonEdition);
        }finally{comparisonChanging=false;}
        ComparisonPrimary.Text=edition;
        PreviousButton.IsEnabled=book>0 || chapter>0;
        NextButton.IsEnabled=book<books.Count-1 || chapter<books[book].Chapters-1;
        var key=$"{edition}/{book}/{chapter}/{comparisonEdition}";
        if(comparisonKey!=key || comparisonData==null){
            var generation=++comparisonGeneration;
            ComparisonList.ItemsSource=null;ComparisonNotice.Text=T("Caricamento confronto…");
            var request=new{op="compare",edition,other=comparisonEdition,book,chapter};
            var data=await Task.Run(()=>BibleEngine.Call(request));
            if(closed || !comparing || generation!=comparisonGeneration)return;
            comparisonKey=key;comparisonData=data;
        }
        RenderComparison();
    });
    private void RenderComparison() {
        if(!comparing || comparisonData is not {} data)return;
        string Join(JsonElement values,string property)=>string.Join("\n\n",values.EnumerateArray().Select(v=>v.GetProperty(property).GetString()));
        ComparedPosition? Position(JsonElement values)=>values.GetArrayLength()==0?null:new(values[0].GetProperty("edition").GetString()!,values[0].GetProperty("book").GetInt32(),values[0].GetProperty("chapter").GetInt32(),values[0].GetProperty("verse").GetInt32());
        var rows=data.GetProperty("rows").EnumerateArray().Select(row=>{
            var left=row.GetProperty("left");var right=row.GetProperty("right");
            return new ComparisonRowView {Index=left[0].GetProperty("verse").GetInt32(),LastVerse=left[left.GetArrayLength()-1].GetProperty("verse").GetInt32(),
                LeftReference=Join(left,"reference"),RightReference=Join(right,"reference"),LeftText=Join(left,"text"),RightText=right.GetArrayLength()==0?T("Nessun versetto corrispondente verificato."):Join(right,"text"),
                TextSize=fontSize,LeftPosition=Position(left),RightPosition=Position(right),OpenLabel=T("Apri e annota")};
        }).ToList();
        ComparisonList.ItemsSource=rows;
        ComparisonNotice.Text=T(data.GetProperty("status").GetString() switch{
            "missingBook"=>"Questo libro non è presente nella seconda traduzione.",
            "differentNumbering"=>"Numerazione diversa: corrispondenza non verificata per questo capitolo.",
            "partial"=>"Alcuni riferimenti non hanno una corrispondenza verificata. I riferimenti originali sono conservati.",
            _=>"Versetti allineati · scorrimento sincronizzato · ↗ per leggere e annotare."});
        if(edition=="CEI 2008" || comparisonEdition=="CEI 2008")ComparisonNotice.Text+="\nCEI 2008 · © Fondazione di Religione Santi Francesco d’Assisi e Caterina da Siena · "+T("Uso formativo e didattico non commerciale.");
        var target=rows.FirstOrDefault(r=>r.Index<=currentVerse && r.LastVerse>=currentVerse);
        if(target!=null)DispatcherQueue.TryEnqueue(()=>{if(comparing && ReferenceEquals(ComparisonList.ItemsSource,rows))ComparisonList.ScrollIntoView(target,ScrollIntoViewAlignment.Leading);});
    }
    private void OpenComparedVerse(object sender,RoutedEventArgs e) => Guard(()=>{
        if(sender is not Button {Tag:ComparedPosition position})return;
        ComparisonToggle.IsChecked=false;
        edition=position.Edition;book=position.Book;chapter=position.Chapter;currentVerse=position.Verse;
        bibleLanguage=editionOptions.First(x=>x.Id==edition).Language;
        if(bibleLanguage=="und")bibleLanguage="all";
        RefreshEditionPicker();LoadBooks();LoadChapter(currentVerse);
    });
}
