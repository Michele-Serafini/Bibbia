using System.ComponentModel;
using System.Text.Json;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;

namespace Bibbia.Windows;

public sealed class VerseRow : INotifyPropertyChanged {
    public event PropertyChangedEventHandler? PropertyChanged;
    private JsonElement annotation;
    private bool focused,english;
    public int Index {get;init;}
    public required string Number {get;init;}
    public required string Text {get;init;}
    public double TextSize {get;set;}
    public bool Focused {get=>focused;set{if(focused==value)return;focused=value;Changed(nameof(MenuVisibility));}}
    public bool English {get=>english;set{if(english==value)return;english=value;Changed(nameof(NoteBadge),nameof(NoteHint),nameof(BookmarkHint));}}
    public bool HasNote=>Annotation.ValueKind==JsonValueKind.Object && Annotation.GetProperty("note").GetString() is {Length:>0};
    public bool HasBookmark=>Annotation.ValueKind==JsonValueKind.Object && Annotation.GetProperty("bookmark").GetBoolean();
    public bool HasMarks=>Annotation.ValueKind==JsonValueKind.Object && (Annotation.GetProperty("underline").GetBoolean() || Annotation.GetProperty("highlight").ValueKind==JsonValueKind.String || Annotation.GetProperty("ranges").GetArrayLength()>0);
    public string NoteBadge=>english?"✎ Note":"✎ Nota";
    public string NoteHint {get{var value=HasNote?Annotation.GetProperty("note").GetString()!:"";var n=Math.Min(120,value.Length);if(n>0&&char.IsHighSurrogate(value[n-1]))n--;return (english?"Open note":"Apri nota")+(n>0?" · "+value[..n]:"");}}
    public string BookmarkHint=>english?$"Saved bookmark · verse {Number} · click to manage":$"Segnalibro salvato · versetto {Number} · premi per gestirlo";
    public string MenuId=>$"VerseMenu_{Index}";
    public string NoteId=>$"VerseNote_{Index}";
    public string BookmarkId=>$"VerseBookmark_{Index}";
    public Visibility NoteVisibility=>HasNote?Visibility.Visible:Visibility.Collapsed;
    public Visibility BookmarkVisibility=>HasBookmark?Visibility.Visible:Visibility.Collapsed;
    public Visibility MenuVisibility=>focused?Visibility.Collapsed:Visibility.Visible;
    private global::Windows.UI.Color Tint=>Annotation.TryGetProperty("noteColor",out var c)?c.GetString() switch {
        "green"=>global::Windows.UI.Color.FromArgb(255,151,207,145),"blue"=>global::Windows.UI.Color.FromArgb(255,142,192,237),"pink"=>global::Windows.UI.Color.FromArgb(255,236,168,195),_=>global::Windows.UI.Color.FromArgb(255,225,191,65)
    }:global::Windows.UI.Color.FromArgb(255,225,191,65);
    public Brush AnnotationAccent=>new SolidColorBrush(HasNote?Tint:HasBookmark?global::Windows.UI.Color.FromArgb(255,180,131,37):Microsoft.UI.Colors.Transparent);
    public Brush NoteBackground {get{var c=Tint;c.A=HasNote?(byte)22:(byte)0;return new SolidColorBrush(c);}}
    public required JsonElement Annotation {get=>annotation;set{annotation=value;Changed(nameof(HasNote),nameof(HasBookmark),nameof(HasMarks),nameof(NoteVisibility),nameof(BookmarkVisibility),nameof(NoteHint),nameof(BookmarkHint),nameof(AnnotationAccent),nameof(NoteBackground));}}
    private void Changed(params string[] properties){foreach(var name in properties)PropertyChanged?.Invoke(this,new(name));}
}
