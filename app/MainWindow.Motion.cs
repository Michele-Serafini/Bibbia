using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Hosting;
using Windows.UI.ViewManagement;
using System.Numerics;

namespace Bibbia.Windows;

public sealed partial class MainWindow
{
    private readonly UISettings motionSettings = new();
    private void AnimateReveal(UIElement element,int milliseconds=180)
    {
        if (element is not FrameworkElement {IsLoaded:true} || (!motionSettings.AnimationsEnabled && Environment.GetEnvironmentVariable("BIBBIA_UI_TEST_MOTION") != "1")) return;
        ElementCompositionPreview.SetIsTranslationEnabled(element,true);
        var compositor=ElementCompositionPreview.GetElementVisual(element).Compositor;
        var easing=compositor.CreateCubicBezierEasingFunction(new Vector2(0.2f,0.8f),new Vector2(0.2f,1));
        var fade=compositor.CreateScalarKeyFrameAnimation();
        fade.Target="Opacity"; fade.Duration=TimeSpan.FromMilliseconds(milliseconds);
        fade.InsertKeyFrame(0,0); fade.InsertKeyFrame(1,1,easing);
        var slide=compositor.CreateScalarKeyFrameAnimation();
        slide.Target="Translation.Y"; slide.Duration=fade.Duration;
        slide.InsertKeyFrame(0,8); slide.InsertKeyFrame(1,0,easing);
        element.StartAnimation(fade); element.StartAnimation(slide);
    }
}
