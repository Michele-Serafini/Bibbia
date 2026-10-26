using Microsoft.UI.Composition;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Hosting;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using System.Numerics;
using Windows.Foundation;

namespace Bibbia.Windows;

public sealed partial class MainWindow
{
    private sealed record PageSnapshot(RenderTargetBitmap Bitmap,double Width,double Height);
    private CompositionScopedBatch? bookBatch;
    private readonly List<(UIElement Element,ICompositionAnimationBase Animation)> bookMotionElements = [];
    private int chapterRenderGeneration;
    private (string Edition,int Book,int Chapter)? displayedChapter;
    private bool BookMotionAllowed => motionSettings.AnimationsEnabled || Environment.GetEnvironmentVariable("BIBBIA_UI_TEST_MOTION") == "1";
    private void TraceBook(string message)
    {
        if (!selectionDiagnostics) return;
        try { File.AppendAllText(Path.Combine(dataDirectory,"ui-events.log"),$"{DateTime.UtcNow:O} book {message}\n"); } catch { }
    }

    private void InitializeBookMotion()
    {
        BookStage.SizeChanged += (_,_) => CancelBookTurn();
        Root.ActualThemeChanged += (_,_) => CancelBookTurn();
        Root.AddHandler(UIElement.PointerPressedEvent,new Microsoft.UI.Xaml.Input.PointerEventHandler((_,_) => CancelBookTurn()),true);
        BookStage.AddHandler(UIElement.PointerWheelChangedEvent,new Microsoft.UI.Xaml.Input.PointerEventHandler((_,_) => CancelBookTurn()),true);
        motionSettings.AnimationsEnabledChanged += (_,_) => DispatcherQueue.TryEnqueue(() => {
            if (!motionSettings.AnimationsEnabled) CancelBookTurn();
        });
    }

    private async Task<PageSnapshot?> CaptureBookPage()
    {
        CancelBookTurn();
        if (!BookMotionAllowed || section != "read" || !BookViewport.IsLoaded || BookViewport.ActualWidth<20 || BookViewport.ActualHeight<20) return null;
        var width=BookViewport.ActualWidth; var height=BookViewport.ActualHeight;
        var scale=Math.Min(Root.XamlRoot.RasterizationScale,Math.Min(1600/width,Math.Sqrt(2_000_000/(width*height))));
        var image=new RenderTargetBitmap();
        TraceBook("capture begin");
        try { await image.RenderAsync(BookViewport,Math.Max(1,(int)(width*scale)),Math.Max(1,(int)(height*scale))); }
        catch (Exception e) { System.Diagnostics.Debug.WriteLine(e); return null; }
        TraceBook("capture complete");
        return new(image,width,height);
    }

    private void TurnBookPage(PageSnapshot? snapshot,bool forward)
    {
        CancelBookTurn();
        if (closed || !BookMotionAllowed || snapshot==null || section!="read" ||
            Math.Abs(snapshot.Width-BookViewport.ActualWidth)>1 || Math.Abs(snapshot.Height-BookViewport.ActualHeight)>1) return;
        var width=snapshot.Width; var height=snapshot.Height;
        var frames=Enumerable.Range(0,33).Select(i=>PageTurnMath.Frame(width,height,i/32.0,forward)).ToArray();
        var compositor=ElementCompositionPreview.GetElementVisual(BookViewport).Compositor;
        var linear=compositor.CreateLinearEasingFunction();
        TurnCanvas.Clip=new RectangleGeometry {Rect=new Rect(0,0,width,height)};
        TurnCanvas.Visibility=Visibility.Visible;
        TraceBook("build strips");
        for (var i=0;i<PageTurnMath.Strips;i++) {
            var start=frames[0][i];
            var slice=new Grid {Width=width/PageTurnMath.Strips+0.7,Height=height,Background=BookViewport.Background,
                Clip=new RectangleGeometry {Rect=new Rect(0,0,width/PageTurnMath.Strips+0.7,height)}};
            var front=new Image {Source=snapshot.Bitmap,Width=width,Height=height,Stretch=Stretch.Fill};
            var content=new Canvas(); Canvas.SetLeft(front,-start.Source); content.Children.Add(front);
            var shade=new Border {Background=new SolidColorBrush(Microsoft.UI.Colors.Black),Opacity=0};
            slice.Children.Add(content); slice.Children.Add(shade);
            TurnCanvas.Children.Add(slice);
            ElementCompositionPreview.SetIsTranslationEnabled(slice,true);
            slice.CenterPoint=Vector3.Zero;
            slice.Translation=new Vector3((float)start.X,(float)start.Y,0);
            var scale=compositor.CreateVector3KeyFrameAnimation(); scale.Target="Scale";
            var move=compositor.CreateVector3KeyFrameAnimation(); move.Target="Translation";
            var face=compositor.CreateScalarKeyFrameAnimation(); face.Target="Opacity";
            var light=compositor.CreateScalarKeyFrameAnimation(); light.Target="Opacity";
            scale.Duration=move.Duration=face.Duration=light.Duration=TimeSpan.FromMilliseconds(PageTurnMath.Milliseconds);
            for (var k=0;k<frames.Length;k++) {
                var f=frames[k][i]; var t=k/32f;
                scale.InsertKeyFrame(t,new Vector3((float)f.ScaleX,(float)f.ScaleY,1),linear);
                move.InsertKeyFrame(t,new Vector3((float)f.X,(float)f.Y,0),linear);
                face.InsertKeyFrame(t,(float)f.Front,linear);
                light.InsertKeyFrame(t,(float)f.Shade,linear);
            }
            bookMotionElements.AddRange([(slice,scale),(slice,move),(front,face),(shade,light)]);
        }
        TurnCanvas.UpdateLayout();
        TraceBook($"strips allocated loaded={TurnCanvas.Children[0] is FrameworkElement {{IsLoaded:true}}}");
        bookBatch=compositor.CreateScopedBatch(CompositionBatchTypes.Animation);
        bookBatch.Completed += BookTurnCompleted;
        foreach (var motion in bookMotionElements) motion.Element.StartAnimation(motion.Animation);
        TraceBook("animations started");
        if (selectionDiagnostics) Microsoft.UI.Xaml.Automation.AutomationProperties.SetHelpText(BookStage,"page-turn-active");
        bookBatch.End();
    }

    private void BookTurnCompleted(object sender,CompositionBatchCompletedEventArgs args)
    {
        TraceBook("completed");
        if (ReferenceEquals(sender,bookBatch)) DispatcherQueue.TryEnqueue(() => {
            if (ReferenceEquals(sender,bookBatch)) CancelBookTurn();
        });
    }

    private void CancelBookTurn()
    {
        var batch=bookBatch; bookBatch=null;
        if (batch!=null) batch.Completed-=BookTurnCompleted;
        foreach (var motion in bookMotionElements) motion.Element.StopAnimation(motion.Animation);
        batch?.Dispose();
        bookMotionElements.Clear();
        TurnCanvas?.Children.Clear();
        if (TurnCanvas!=null) TurnCanvas.Visibility=Visibility.Collapsed;
        if (selectionDiagnostics && BookStage!=null) Microsoft.UI.Xaml.Automation.AutomationProperties.SetHelpText(BookStage,"page-turn-idle");
    }
}
