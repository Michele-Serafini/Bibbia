namespace Bibbia.Windows;

internal static class PageTurnMath
{
    public const int Strips = 32;
    public const int Milliseconds = 460;
    internal readonly record struct Strip(double Source, double X, double Y, double ScaleX, double ScaleY, double Front, double Shade);
    public static Strip[] Frame(double width, double height, double progress, bool forward)
    {
        if (width <= 0 || height <= 0) return [];
        var t = Math.Clamp(progress,0,1);
        var p = t*t*t*(t*(t*6-15)+10);
        var bend = 0.65*Math.Sin(Math.PI*p);
        var step = width/Strips;
        var depth = width*6;
        double x=0,z=0;
        var result=new Strip[Strips];
        for (var i=0;i<Strips;i++) {
            var angle=Math.PI*p+bend*(2*(i+0.5)/Strips-1);
            var nx=x+step*Math.Cos(angle); var nz=z+step*Math.Sin(angle);
            var a=depth/(depth-z); var b=depth/(depth-nz);
            var left=width/2+(x-width/2)*a; var right=width/2+(nx-width/2)*b;
            var sy=(a+b)/2;
            result[i]=new(forward?i*step:width-(i+1)*step,forward?left:width-right,
                height*(1-sy)/2,(right-left)/step,sy,Math.Cos(angle)>=0?1:0,0.18*Math.Abs(Math.Sin(angle)));
            x=nx; z=nz;
        }
        return result;
    }
}
