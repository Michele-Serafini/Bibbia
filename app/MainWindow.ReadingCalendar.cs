using Microsoft.UI.Xaml.Controls;
using Windows.Storage;
using Windows.Storage.Pickers;
using Windows.System;
namespace Bibbia.Windows;
public sealed partial class MainWindow {
    private async Task ExportPlanCalendar(bool save){
        if(readingPlanPicker.SelectedItem is not PlanChoice plan)return;
        string path;
        if(save){
            var picker=new FileSavePicker{SuggestedFileName="Bibbia-piano-di-lettura"};picker.FileTypeChoices.Add("Calendario iCalendar",new[]{".ics"});
            WinRT.Interop.InitializeWithWindow.Initialize(picker,WinRT.Interop.WindowNative.GetWindowHandle(this));
            var file=await picker.PickSaveFileAsync();if(file==null)return;path=file.Path;
        }else{var directory=Path.Combine(dataDirectory,"calendar");Directory.CreateDirectory(directory);path=Path.Combine(directory,$"Piano-{plan.Id}.ics");}
        var selectedLanguage=language;await Task.Run(()=>BibleEngine.Call(new{op="reading",action="calendar",id=plan.Id,path,language=selectedLanguage}));
        if(closed)return;
        if(save){AnnotationFeedback(N("Calendario salvato · Importalo nella tua app calendario.","Calendar saved · Import it into your calendar app."));return;}
        var opened=await Launcher.LaunchFileAsync(await StorageFile.GetFileFromPathAsync(path));
        if(!opened){var dialog=new ContentDialog{XamlRoot=Root.XamlRoot,RequestedTheme=Root.RequestedTheme,Title=N("Scegli un’app calendario","Choose a calendar app"),Content=N("Non è disponibile un’app associata ai file .ics. Usa «Salva calendario .ics…» e importa il file nel calendario che preferisci.","No app is associated with .ics files. Use Save calendar .ics… and import the file into your preferred calendar."),CloseButtonText=T("Chiudi")};await dialog.ShowAsync();}
    }
}
