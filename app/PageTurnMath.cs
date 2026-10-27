namespace Bibbia.Windows;

// The native renderers share this developable cylindrical curl. The diagonal
// crease reaches the upper outside corner first; unturned paper stays flat.
internal static class PageTurnMath
{
    public const int Strips = 64;
    public const int Samples = 60;
    public const int Milliseconds = 1100;
    public const double Angle = -0.42;
    internal readonly record struct Layout(double NX,double NY,double MinU,double SpanU,double MinV,double SpanV)
    {
        public double Step => SpanU/Strips;
    }
    internal readonly record struct Strip(double Source,double X,double Y,double ScaleX,double ScaleY,double Front,double ShadeStart,double ShadeEnd);
    internal readonly record struct Shadow(double X,double Y,double Opacity);
    internal sealed record Sheet(Layout Layout,Strip[] Strips,Shadow Shadow);
    internal readonly record struct Point(double X,double Y,double Z);

    private static Layout Paper(double width,double height) => new(Math.Cos(Angle),Math.Sin(Angle),
        Math.Sin(Angle)*height,Math.Cos(Angle)*width-Math.Sin(Angle)*height,0,
        -Math.Sin(Angle)*width+Math.Cos(Angle)*height);
    private static double Ease(double t) { t=Math.Clamp(double.IsFinite(t)?t:0,0,1); return t*t*(3-2*t); }
    private static (double Q,double Z,double Cos,double Shade) Bend(double u,double fold,double radius)
    {
        var d=u-fold;
        if (d<=0) return (u,0,1,0);
        var angle=Math.Min(Math.PI,d/radius);
        return (fold+radius*Math.Sin(angle)-Math.Max(0,d-Math.PI*radius),
            radius*(1-Math.Cos(angle)),Math.Cos(angle),0.24*Math.Abs(Math.Sin(angle))+(angle>Math.PI/2?0.035:0));
    }
    public static Sheet Frame(double width,double height,double progress,bool forward)
    {
        if (!double.IsFinite(width) || !double.IsFinite(height) || width<=0 || height<=0)
            return new(default,[],default);
        var l=Paper(width,height); var p=Ease(progress);
        var radius=width*(0.075+0.065*Math.Sin(Math.PI*p));
        var fold=l.MinU+l.SpanU-(l.SpanU+width*0.88)*p;
        var strips=new Strip[Strips];
        for (var i=0;i<Strips;i++) {
            var u=l.MinU+i*l.Step;
            var a=Bend(u,fold,radius); var b=Bend(u+l.Step,fold,radius);
            var middle=Bend(u+l.Step/2,fold,radius);
            var x=l.NX*a.Q-l.NY*l.MinV;
            strips[i]=new(u,forward?x:width-x,l.NY*a.Q+l.NX*l.MinV,
                (b.Q-a.Q)/l.Step,forward?1:-1,middle.Cos>=0?1:0,a.Shade,b.Shade);
        }
        var shadowU=fold-radius*0.12;
        var shadowX=l.NX*shadowU-l.NY*l.MinV;
        return new(l,strips,new(forward?shadowX:width-shadowX,l.NY*shadowU+l.NX*l.MinV,Math.Sin(Math.PI*p)));
    }
    // Analytic point samples distinguish a corner curl from a rigid rotation.
    public static Point Project(double width,double height,double x,double y,double progress,bool forward)
    {
        var l=Paper(width,height); var p=Ease(progress);
        if (!forward) x=width-x;
        var u=l.NX*x+l.NY*y; var v=-l.NY*x+l.NX*y;
        var b=Bend(u,l.MinU+l.SpanU-(l.SpanU+width*0.88)*p,width*(0.075+0.065*Math.Sin(Math.PI*p)));
        var px=l.NX*b.Q-l.NY*v;
        return new(forward?px:width-px,l.NY*b.Q+l.NX*v,b.Z);
    }
}
