using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using System.Text.Json;
namespace Bibbia.Windows;
public sealed partial class MainWindow {
    private static readonly string[] NoteColors=["yellow","green","blue","pink"];
    private static Brush SavedNoteBrush(JsonElement a){
        if(a.ValueKind!=JsonValueKind.Object || a.GetProperty("note").GetString() is not {Length:>0})return new SolidColorBrush(Microsoft.UI.Colors.Transparent);
        var color=a.TryGetProperty("noteColor",out var c)?c.GetString():"yellow";
        var tint=MarkColor(color??"yellow");tint.A=(byte)(a.TryGetProperty("noteStyle",out var style) && style.GetString()=="sticky"?65:24);return new SolidColorBrush(tint);
    }
    private (ComboBox Color,ComboBox Style) NoteChoices(StackPanel content,JsonElement a){
        var selected=a.ValueKind==JsonValueKind.Object && a.TryGetProperty("noteColor",out var c)?c.GetString():"yellow";
        var color=new ComboBox{Header=N("Colore della nota","Note color"),ItemsSource=new[]{T("Giallo"),T("Verde"),T("Blu"),T("Rosa")},SelectedIndex=Math.Max(0,Array.IndexOf(NoteColors,selected)),HorizontalAlignment=Microsoft.UI.Xaml.HorizontalAlignment.Stretch};
        var style=new ComboBox{Header=N("Stile","Style"),ItemsSource=new[]{T("Nota"),"Post-it"},SelectedIndex=a.ValueKind==JsonValueKind.Object && a.TryGetProperty("noteStyle",out var s) && s.GetString()=="sticky"?1:0,HorizontalAlignment=Microsoft.UI.Xaml.HorizontalAlignment.Stretch};
        content.Children.Add(color);content.Children.Add(style);return(color,style);
    }
}
