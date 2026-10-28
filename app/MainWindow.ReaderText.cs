using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Documents;
using Microsoft.UI.Xaml.Media;
using System.Runtime.CompilerServices;
using Windows.Foundation;
using Windows.UI.Text;

namespace Bibbia.Windows;

public sealed partial class MainWindow {
    private bool readerStyling;
    private sealed record ReaderRun(int Start,int Length,Run Run);
    private readonly ConditionalWeakTable<RichTextBlock,List<ReaderRun>> readerRuns=new();
    private (int Start,int End) ReaderSelection(RichTextBlock editor) {
        if(!readerRuns.TryGetValue(editor,out var spans) || editor.SelectedText.Length==0)return(0,0);
        var nativeStart=Math.Min(editor.SelectionStart.Offset,editor.SelectionEnd.Offset);var nativeEnd=Math.Max(editor.SelectionStart.Offset,editor.SelectionEnd.Offset);
        var first=int.MaxValue;var last=0;
        foreach(var span in spans){
            var a=Math.Max(nativeStart,span.Run.ContentStart.Offset);var b=Math.Min(nativeEnd,span.Run.ContentEnd.Offset);
            if(b<=a)continue;
            first=Math.Min(first,span.Start+a-span.Run.ContentStart.Offset);
            last=Math.Max(last,span.Start+b-span.Run.ContentStart.Offset);
        }
        return first==int.MaxValue?(0,0):(first,last);
    }
    private TextPointer ReaderPointer(RichTextBlock editor,int offset) {
        var spans=readerRuns.GetValue(editor,_=>[]);
        var span=spans.LastOrDefault(s=>s.Start<=offset)??spans[0];
        return span.Run.ContentStart.GetPositionAtOffset(Math.Clamp(offset-span.Start,0,span.Length),LogicalDirection.Forward)??span.Run.ContentEnd;
    }
    private Rect ReaderRect(RichTextBlock editor,int offset)=>ReaderPointer(editor,offset).GetCharacterRect(LogicalDirection.Forward);
    private void ApplyTextStylesCore(RichTextBlock editor,VerseRow row) {
        var selected=ReaderSelection(editor);
        var wasStyling=readerStyling;readerStyling=true;
        try {
        editor.Blocks.Clear();editor.TextHighlighters.Clear();readerRuns.Remove(editor);
        editor.FontSize=row.TextSize;editor.LineHeight=row.TextSize*1.35;
        editor.Foreground=new SolidColorBrush(Root.ActualTheme==ElementTheme.Dark?Colors.White:Colors.Black);
        var paragraph=new Paragraph {Margin=new Thickness(0)};
        var pieces=new List<ReaderRun>();
        var boundaries=new SortedSet<int>{0,row.Text.Length};
        var annotation=row.Annotation;
        foreach(var mark in annotation.GetProperty("ranges").EnumerateArray()){
            var start=mark.GetProperty("start").GetInt32();var length=mark.GetProperty("length").GetInt32();boundaries.Add(start);boundaries.Add(start+length);
        }
        var points=boundaries.ToArray();
        for(var i=0;i+1<points.Length;i++){
            var a=points[i];var b=points[i+1];if(b<=a)continue;
            var color=annotation.GetProperty("highlight").GetString();var underline=annotation.GetProperty("underline").GetBoolean();
            foreach(var mark in annotation.GetProperty("ranges").EnumerateArray()){
                var start=mark.GetProperty("start").GetInt32();var end=start+mark.GetProperty("length").GetInt32();
                if(a<start || b>end)continue;
                underline|=mark.GetProperty("underline").GetBoolean();if(mark.GetProperty("highlight").GetString() is {} value)color=value;
            }
            var run=new Run{Text=row.Text.Substring(a,b-a),TextDecorations=underline?TextDecorations.Underline:TextDecorations.None};
            if(color!=null)run.Foreground=new SolidColorBrush(Colors.Black);
            paragraph.Inlines.Add(run);pieces.Add(new ReaderRun(a,b-a,run));
        }
        editor.Blocks.Add(paragraph);readerRuns.Add(editor,pieces);
        // Native TextPointer offsets account for inline boundaries. Use them
        // for both selection mapping and the native text highlighter ranges.
        foreach(var span in pieces){
            var color=annotation.GetProperty("highlight").GetString();
            foreach(var mark in annotation.GetProperty("ranges").EnumerateArray()){
                var a=mark.GetProperty("start").GetInt32();var b=a+mark.GetProperty("length").GetInt32();
                if(span.Start>=a && span.Start+span.Length<=b && mark.GetProperty("highlight").GetString() is {} value)color=value;
            }
            if(color!=null)editor.TextHighlighters.Add(new TextHighlighter{Background=new SolidColorBrush(MarkColor(color)),Foreground=new SolidColorBrush(Colors.Black),Ranges={new TextRange{StartIndex=span.Run.ContentStart.Offset-editor.ContentStart.Offset,Length=span.Length}}});
        }
        if(selected.End>selected.Start && selected.End<=row.Text.Length)editor.Select(ReaderPointer(editor,selected.Start),ReaderPointer(editor,selected.End));
        } finally {readerStyling=wasStyling;}
    }
    private IEnumerable<PageLine> ReaderLines(RichTextBlock editor,VerseRow row,double top) {
        if(!readerRuns.TryGetValue(editor,out var pieces) || pieces.Count==0)yield break;
        var offset=0;
        while(offset<row.Text.Length){
            var rect=ReaderRect(editor,offset);
            if(rect.Height<=0 || !double.IsFinite(rect.Y))yield break;
            yield return new(top+rect.Y,rect.Height,row.Index,offset);
            var low=offset+1;var high=row.Text.Length;
            while(low<high){var mid=low+(high-low)/2;var next=ReaderRect(editor,mid);if(next.Y>rect.Y+0.5)high=mid;else low=mid+1;}
            if(low>=row.Text.Length)yield break;
            if(low>0 && char.IsLowSurrogate(row.Text[low]) && char.IsHighSurrogate(row.Text[low-1]))low--;
            if(low<=offset)yield break;offset=low;
        }
    }
}
