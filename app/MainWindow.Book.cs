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
    private sealed record PageSnapshot(RenderTargetBitmap Bitmap,double Width,double Height,int Generation);
    private CompositionScopedBatch? bookBatch;
    private readonly List<(UIElement Element,ICompositionAnimationBase Animation)> bookMotionElements = [];
    private int chapterRenderGeneration;
    private int bookMotionGeneration;
    private (string Edition,int Book,int Chapter)? displayedChapter;
    private bool BookMotionAllowed => performanceMotion && (motionSettings.AnimationsEnabled || Environment.GetEnvironmentVariable("BIBBIA_UI_TEST_MOTION") == "1");
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
        var owner=new WeakReference<MainWindow>(this);
        motionSettings.AnimationsEnabledChanged += (_,_) => {
            if (owner.TryGetTarget(out var window) && !window.closed) window.DispatcherQueue.TryEnqueue(() => {
                if (!window.motionSettings.AnimationsEnabled) window.CancelBookTurn();
            });
        };
    }

    private async Task<PageSnapshot?> CaptureBookPage()
    {
        CancelBookTurn();
        var generation=bookMotionGeneration;
        if (!BookMotionAllowed || section != "read" || !BookViewport.IsLoaded || BookViewport.ActualWidth<20 || BookViewport.ActualHeight<20) return null;
        var width=BookViewport.ActualWidth; var height=BookViewport.ActualHeight;
        var scale=Math.Min(Root.XamlRoot.RasterizationScale,Math.Min(1600/width,Math.Sqrt(2_000_000/(width*height))));
        var image=new RenderTargetBitmap();
        TraceBook("capture begin");
        try { await image.RenderAsync(BookViewport,Math.Max(1,(int)(width*scale)),Math.Max(1,(int)(height*scale))); }
        catch (Exception e) { System.Diagnostics.Debug.WriteLine(e); return null; }
        TraceBook("capture complete");
        return new(image,width,height,generation);
    }

    private void TurnBookPage(PageSnapshot? snapshot,bool forward)
    {
        if (snapshot!=null && snapshot.Generation!=bookMotionGeneration) return;
        CancelBookTurn();
        if (closed || !BookMotionAllowed || snapshot==null || section!="read" ||
            Math.Abs(snapshot.Width-BookViewport.ActualWidth)>1 || Math.Abs(snapshot.Height-BookViewport.ActualHeight)>1) return;
        var generation=bookMotionGeneration;
        var width=snapshot.Width; var height=snapshot.Height;
        var frames=Enumerable.Range(0,PageTurnMath.Samples+1)
            .Select(i=>PageTurnMath.Frame(width,height,(double)i/PageTurnMath.Samples,forward)).ToArray();
        var layout=frames[0].Layout;
        var rotation=(float)(forward?PageTurnMath.Angle:Math.PI-PageTurnMath.Angle)*180/MathF.PI;
        var compositor=ElementCompositionPreview.GetElementVisual(BookViewport).Compositor;
        var linear=compositor.CreateLinearEasingFunction();
        TurnCanvas.Clip=new RectangleGeometry {Rect=new Rect(0,0,width,height)};
        TurnCanvas.Visibility=Visibility.Visible;
        TraceBook("build strips");
        var castShadow=new Border {Width=width*0.30,Height=layout.SpanV,Opacity=0,
            Background=new LinearGradientBrush {
                StartPoint=new Point(0,0.5),EndPoint=new Point(1,0.5),
                GradientStops={new GradientStop {Color=Microsoft.UI.Colors.Transparent,Offset=0},
                               new GradientStop {Color=global::Windows.UI.Color.FromArgb(26,0,0,0),Offset=0.40},
                               new GradientStop {Color=global::Windows.UI.Color.FromArgb(56,0,0,0),Offset=0.68},
                               new GradientStop {Color=Microsoft.UI.Colors.Transparent,Offset=1}}
            }};
        castShadow.CenterPoint=Vector3.Zero; castShadow.Rotation=rotation;
        castShadow.Scale=new Vector3(1,forward?1:-1,1);
        castShadow.Translation=new Vector3((float)frames[0].Shadow.X,(float)frames[0].Shadow.Y,0);
        TurnCanvas.Children.Add(castShadow);
        var cast=compositor.CreateScalarKeyFrameAnimation(); cast.Target="Opacity";
        var castMove=compositor.CreateVector3KeyFrameAnimation(); castMove.Target="Translation";
        cast.Duration=TimeSpan.FromMilliseconds(PageTurnMath.Milliseconds);
        castMove.Duration=cast.Duration;
        for (var k=0;k<frames.Length;k++) {
            var s=frames[k].Shadow; var t=(float)k/PageTurnMath.Samples;
            cast.InsertKeyFrame(t,(float)s.Opacity,linear);
            castMove.InsertKeyFrame(t,new Vector3((float)s.X,(float)s.Y,0),linear);
        }
        bookMotionElements.AddRange([(castShadow,cast),(castShadow,castMove)]);
        for (var i=0;i<PageTurnMath.Strips;i++) {
            var start=frames[0].Strips[i];
            var slice=new Canvas {Width=layout.Step+0.65,Height=layout.SpanV,
                Clip=new RectangleGeometry {Rect=new Rect(0,0,layout.Step+0.65,layout.SpanV)}};
            // Rotate the one shared page into diagonal bands, clipping both the
            // bands and the physical paper rectangle before any deformation.
            var content=new Grid {Width=width,Height=height,Background=BookViewport.Background,
                Clip=new RectangleGeometry {Rect=new Rect(0,0,width,height)},
                RenderTransform=new MatrixTransform {Matrix=new Matrix(
                    forward?layout.NX:-layout.NX,forward?-layout.NY:layout.NY,layout.NY,layout.NX,
                    (forward?0:layout.NX*width)-start.Source,(forward?0:-layout.NY*width)-layout.MinV)}};
            var front=new Image {Source=snapshot.Bitmap,Width=width,Height=height,Stretch=Stretch.Fill};
            var sx=layout.NX*start.Source-layout.NY*layout.MinV;
            var sy=layout.NY*start.Source+layout.NX*layout.MinV;
            var ex=sx+layout.NX*layout.Step; var ey=sy+layout.NY*layout.Step;
            var gradientStart=new Point((forward?sx:width-sx)/width,sy/height);
            var gradientEnd=new Point((forward?ex:width-ex)/width,ey/height);
            Border Shade(bool reversed) => new() {Opacity=0,Background=new LinearGradientBrush {
                StartPoint=gradientStart,EndPoint=gradientEnd,GradientStops={
                    new GradientStop {Color=reversed?Microsoft.UI.Colors.Transparent:Microsoft.UI.Colors.Black,Offset=0},
                    new GradientStop {Color=reversed?Microsoft.UI.Colors.Black:Microsoft.UI.Colors.Transparent,Offset=1}}}};
            var shadeStart=Shade(false); var shadeEnd=Shade(true);
            content.Children.Add(front); content.Children.Add(shadeStart); content.Children.Add(shadeEnd);
            slice.Children.Add(content);
            TurnCanvas.Children.Add(slice);
            slice.CenterPoint=Vector3.Zero; slice.Rotation=rotation;
            slice.Translation=new Vector3((float)start.X,(float)start.Y,0);
            slice.Scale=new Vector3(1,(float)start.ScaleY,1);
            var scale=compositor.CreateVector3KeyFrameAnimation(); scale.Target="Scale";
            var move=compositor.CreateVector3KeyFrameAnimation(); move.Target="Translation";
            var face=compositor.CreateScalarKeyFrameAnimation(); face.Target="Opacity";
            var lightStart=compositor.CreateScalarKeyFrameAnimation(); lightStart.Target="Opacity";
            var lightEnd=compositor.CreateScalarKeyFrameAnimation(); lightEnd.Target="Opacity";
            scale.Duration=move.Duration=face.Duration=lightStart.Duration=lightEnd.Duration=TimeSpan.FromMilliseconds(PageTurnMath.Milliseconds);
            for (var k=0;k<frames.Length;k++) {
                var f=frames[k].Strips[i]; var t=(float)k/PageTurnMath.Samples;
                scale.InsertKeyFrame(t,new Vector3((float)f.ScaleX,(float)f.ScaleY,1),linear);
                move.InsertKeyFrame(t,new Vector3((float)f.X,(float)f.Y,0),linear);
                face.InsertKeyFrame(t,(float)f.Front,linear);
                lightStart.InsertKeyFrame(t,(float)f.ShadeStart,linear);
                lightEnd.InsertKeyFrame(t,(float)f.ShadeEnd,linear);
            }
            bookMotionElements.AddRange([(slice,scale),(slice,move),(front,face),(shadeStart,lightStart),(shadeEnd,lightEnd)]);
        }
        TurnCanvas.UpdateLayout();
        // Layout may synchronously resize the reader and cancel this turn.
        // Never start or inspect visuals that its SizeChanged handler released.
        if (generation!=bookMotionGeneration || closed) return;
        TraceBook("strips allocated loaded=" + TurnCanvas.IsLoaded);
        bookBatch=compositor.CreateScopedBatch(CompositionBatchTypes.Animation);
        bookBatch.Completed += BookTurnCompleted;
        foreach (var motion in bookMotionElements) motion.Element.StartAnimation(motion.Animation);
        TraceBook("animations started");
        if (selectionDiagnostics) Microsoft.UI.Xaml.Automation.AutomationProperties.SetHelpText(VerseList,"page-turn-active");
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
        bookMotionGeneration++;
        var batch=bookBatch; bookBatch=null;
        if (batch!=null) batch.Completed-=BookTurnCompleted;
        foreach (var motion in bookMotionElements) motion.Element.StopAnimation(motion.Animation);
        batch?.Dispose();
        bookMotionElements.Clear();
        TurnCanvas?.Children.Clear();
        if (TurnCanvas!=null) TurnCanvas.Visibility=Visibility.Collapsed;
        if (selectionDiagnostics && VerseList!=null) Microsoft.UI.Xaml.Automation.AutomationProperties.SetHelpText(VerseList,"page-turn-idle");
    }
}
