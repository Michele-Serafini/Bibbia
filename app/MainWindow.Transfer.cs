using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Automation;
using Windows.Storage.Pickers;
namespace Bibbia.Windows;
public sealed partial class MainWindow {
    private readonly StackPanel transferButtons=new(){Spacing=10};
    private void InitializeTransfer(){
        var box=(StackPanel)SettingsPanel.Content;box.Children.Insert(box.Children.Count-1,transferButtons);RelabelTransfer();
    }
    private void RelabelTransfer(){
        transferButtons.Children.Clear();transferButtons.Children.Add(new TextBlock{Text=N("Esportazione e backup","Export and backup"),FontSize=20});
        foreach(var (action,it,en) in new[]{("markdown","Esporta appunti Markdown…","Export notes as Markdown…"),("pdf","Esporta appunti PDF…","Export notes as PDF…"),("backup","Crea backup…","Create backup…"),("restore","Ripristina backup…","Restore backup…")}){
            var button=new Button{Content=N(it,en)};AutomationProperties.SetAutomationId(button,"Transfer_"+action);button.Click+=async(_,_)=>await GuardAsync(()=>TransferClick(action));transferButtons.Children.Add(button);
        }
        transferButtons.Children.Add(new TextBlock{Text=N("Un solo backup trasferisce appunti, cartelle, etichette, piani e traduzioni importate tra tutti i sistemi. Il PDF usa Microsoft Print to PDF di Windows.","One backup transfers annotations, folders, tags, plans and imported editions between systems. PDF export uses Windows Microsoft Print to PDF."),TextWrapping=TextWrapping.Wrap});
    }
    private async Task TransferClick(string action){
        if(!ready)return;var handle=WinRT.Interop.WindowNative.GetWindowHandle(this);string? path;
        if(action=="restore"){
            var picker=new FileOpenPicker();WinRT.Interop.InitializeWithWindow.Initialize(picker,handle);picker.FileTypeFilter.Add(".bibbia-backup");var file=await picker.PickSingleFileAsync();if(file==null)return;path=file.Path;
            var info=await Task.Run(()=>BibleEngine.Call(new{op="transfer",action="inspect",path}));var dialog=new ContentDialog{XamlRoot=Root.XamlRoot,RequestedTheme=Root.RequestedTheme,Title=N("Ripristinare il backup?","Restore backup?"),Content=$"{info.GetProperty("annotations")} "+N("appunti","annotations")+" · "+info.GetProperty("reading_plans")+" "+N("piani","plans")+"\n\n"+N("Sostituisce i dati al riavvio. Una copia dei dati attuali sarà conservata nella cartella dell’app. L’app si chiuderà.","Replaces your data on restart. A copy of your current data will be kept in the app’s folder. The app will close."),PrimaryButtonText=N("Ripristina e chiudi","Restore and close"),CloseButtonText=T("Annulla"),DefaultButton=ContentDialogButton.Close};
            if(await dialog.ShowAsync()!=ContentDialogResult.Primary)return;await Task.Run(()=>BibleEngine.Call(new{op="transfer",action="restore",path}));Close();return;
        }
        var extension=action=="markdown"?".md":action=="pdf"?".pdf":".bibbia-backup";var save=new FileSavePicker{SuggestedFileName="Bibbia-"+DateTime.Now.ToString("yyyy-MM-dd")};save.FileTypeChoices.Add(action,new List<string>{extension});WinRT.Interop.InitializeWithWindow.Initialize(save,handle);var target=await save.PickSaveFileAsync();if(target==null)return;path=target.Path;
        if(action=="pdf")await Task.Run(()=>BibleEngine.Call(new{op="transfer",action="check_destination",path}));
        var busy=new ContentDialog{XamlRoot=Root.XamlRoot,RequestedTheme=Root.RequestedTheme,Title=N("Preparazione del file…","Preparing file…"),Content=new ProgressRing{IsActive=true,Width=40,Height=40}};var shown=busy.ShowAsync();
        try{await Task.Run(()=>{
            if(action=="pdf") {var temp=Path.Combine(dataDirectory,".export-"+Guid.NewGuid()+".txt");try{BibleEngine.Call(new{op="export",format="text",path=temp});NotesPdf.Write(temp,path);}finally{File.Delete(temp);}}
            else BibleEngine.Call(action=="backup"?(object)new{op="transfer",action="backup",path}:new{op="export",format="markdown",path});
        });}finally{busy.Hide();await shown;}
        StatusLabel.Text=N("File salvato: ","File saved: ")+path;
    }
    private async Task TransferSmoke(){
        var root=Environment.GetEnvironmentVariable("BIBBIA_TRANSFER_SMOKE_DIR");if(root==null)return;
        if(!Path.GetFileName(root).StartsWith("bibbia-transfer-") || !File.Exists(Path.Combine(root,".bibbia-transfer-fixture")))throw new InvalidOperationException("PDF test requires a marked fixture directory.");
        await Task.Run(()=>{
            BibleEngine.Call(new{op="transfer",action="backup",path=Path.Combine(root,"fixture.bibbia-backup")});
            BibleEngine.Call(new{op="export",format="markdown",path=Path.Combine(root,"notes.md")});
            var text=Path.Combine(root,"pages.txt");File.WriteAllLines(text,Enumerable.Range(0,360).Select(i=>$"Riga {i:000} · perché · Ω · luce 🌅 · una nota da leggere."));
            var pages=NotesPdf.Write(text,Path.Combine(root,"notes.pdf"));if(pages<3)throw new InvalidOperationException("PDF did not paginate the document.");File.WriteAllText(Path.Combine(root,"passed.txt"),"passed");
        });
    }

}
