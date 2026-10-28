using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.Diagnostics;
using System.Text.Json;

namespace Bibbia.Windows;

public sealed partial class MainWindow {
    private bool performanceMotion = true;
    internal static string? PerformanceProfile() {
        var path=Environment.GetEnvironmentVariable("BIBBIA_PERFORMANCE_PROFILE");
        return path!=null && Path.GetFileName(path).StartsWith("bibbia-perf-",StringComparison.Ordinal)
            && File.Exists(Path.Combine(path,".bibbia-perf-fixture"))?path:null;
    }
    private async Task RunPerformance(string profile) {
        using var gate=new EventWaitHandle(false,EventResetMode.AutoReset,$"Local\\Bibbia.Performance.{Environment.ProcessId}");
        performanceMotion=false;
        var clock=Stopwatch.StartNew();
        async Task WaitReady() {
            for(var n=0;n<400;n++) {
                if(!comparing && bookChapter==$"{edition}/{book}/{chapter}" && bookPages.Count>0 && bookLayoutTimer?.IsRunning!=true)break;
                if(comparing && comparisonData!=null && comparisonKey==$"{edition}/{book}/{chapter}/{comparisonEdition}")break;
                await Task.Delay(20);
                if(n==399)throw new InvalidOperationException("Performance fixture did not render");
            }
            await Task.Delay(120);
        }
        async Task Checkpoint(string name) {
            await WaitReady();
            var elapsed=clock.Elapsed.TotalMilliseconds;
            var wait=Task.Run(()=>gate.WaitOne(TimeSpan.FromSeconds(60)));
            File.WriteAllText(Path.Combine(profile,name+".json"),JsonSerializer.Serialize(new{elapsed_ms=elapsed,editors=Descendants<RichEditBox>(Root).Count()}));
            if(!await wait)throw new TimeoutException("Performance controller timed out");
            clock.Restart();
        }
        AppWindow.Resize(new global::Windows.Graphics.SizeInt32(1000,720));
        await Checkpoint("initial");
        edition="CEI 2008";book=22;chapter=118;currentVerse=0;LoadBooks();await LoadChapterAsync(0);
        await Checkpoint("long_chapter");
        comparisonEdition="KJV";ComparisonToggle.IsChecked=true;
        await Checkpoint("comparison");
        ComparisonToggle.IsChecked=false;
        await Checkpoint("returned");
        AppWindow.Resize(new global::Windows.Graphics.SizeInt32(1500,900));
        await Checkpoint("wide");
        AppWindow.Resize(new global::Windows.Graphics.SizeInt32(1000,720));
        await Checkpoint("narrow");
        for(var i=0;i<8;i++) {
            chapter=117+i%2;LoadBooks();await LoadChapterAsync(0);await WaitReady();
            ComparisonToggle.IsChecked=true;await WaitReady();ComparisonToggle.IsChecked=false;await WaitReady();
        }
        await Checkpoint("cycled");
        performanceMotion=true;NextClick(this,new RoutedEventArgs());
        for(var i=0;i<100 && TurnCanvas.Visibility!=Visibility.Visible;i++)await Task.Delay(10);
        await Checkpoint("animation");
        await Task.Delay(1300);
        await Checkpoint("settled");
        Close();
    }
}
