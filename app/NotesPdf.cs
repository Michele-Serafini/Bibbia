using System.Runtime.InteropServices;
using System.ComponentModel;
using System.Text;
namespace Bibbia.Windows;
/// Uses the Windows PDF printer and native Unicode font shaping, without a bundled renderer.
internal static class NotesPdf {
    [StructLayout(LayoutKind.Sequential,CharSet=CharSet.Unicode)] private struct DocInfo{public int Size;public string Name;public string Output;public IntPtr Type;public int Flags;}
    [StructLayout(LayoutKind.Sequential)]private struct Rect{public int Left,Top,Right,Bottom;}
    [StructLayout(LayoutKind.Sequential)]private struct Printer{public IntPtr Server,Name,Share,Port,Driver,Comment,Location,DevMode,Separator,Processor,DataType,Parameters,Security;public uint Attributes,Priority,DefaultPriority,Start,Until,Status,Jobs,Average;}
    [DllImport("winspool.drv",CharSet=CharSet.Unicode,SetLastError=true)]private static extern bool EnumPrintersW(uint flags,string? name,uint level,IntPtr data,uint size,out uint needed,out uint returned);
    [DllImport("gdi32.dll",CharSet=CharSet.Unicode,SetLastError=true)]private static extern IntPtr CreateDCW(string driver,string device,IntPtr output,IntPtr mode);
    [DllImport("gdi32.dll")]private static extern bool DeleteDC(IntPtr dc);
    [DllImport("gdi32.dll")]private static extern int GetDeviceCaps(IntPtr dc,int index);
    [DllImport("gdi32.dll",CharSet=CharSet.Unicode,SetLastError=true)]private static extern int StartDocW(IntPtr dc,ref DocInfo doc);
    [DllImport("gdi32.dll")]private static extern int StartPage(IntPtr dc);
    [DllImport("gdi32.dll")]private static extern int EndPage(IntPtr dc);
    [DllImport("gdi32.dll")]private static extern int EndDoc(IntPtr dc);
    [DllImport("gdi32.dll")]private static extern int AbortDoc(IntPtr dc);
    [DllImport("gdi32.dll",CharSet=CharSet.Unicode)]private static extern IntPtr CreateFontW(int height,int width,int angle,int orientation,int weight,uint italic,uint underline,uint strike,uint charset,uint precision,uint clip,uint quality,uint pitch,string face);
    [DllImport("gdi32.dll")]private static extern IntPtr SelectObject(IntPtr dc,IntPtr obj);
    [DllImport("gdi32.dll")]private static extern bool DeleteObject(IntPtr obj);
    [DllImport("gdi32.dll")]private static extern int SetBkMode(IntPtr dc,int mode);
    [DllImport("gdi32.dll")]private static extern int SaveDC(IntPtr dc);
    [DllImport("gdi32.dll")]private static extern bool RestoreDC(IntPtr dc,int state);
    [DllImport("gdi32.dll")]private static extern int IntersectClipRect(IntPtr dc,int left,int top,int right,int bottom);
    [DllImport("user32.dll",CharSet=CharSet.Unicode)]private static extern int DrawTextW(IntPtr dc,string text,int length,ref Rect rect,uint format);
    [StructLayout(LayoutKind.Sequential)]private struct Metrics{public int Height,Ascent,Descent,InternalLeading,ExternalLeading,Average,MaxWidth,Weight,Overhang,AspectX,AspectY;public ushort First,Last,Default,Break;public byte Italic,Underline,Strike,Pitch,Charset;}
    [DllImport("gdi32.dll",CharSet=CharSet.Unicode)]private static extern bool GetTextMetricsW(IntPtr dc,out Metrics metrics);
    private static string PdfPrinter(){
        EnumPrintersW(2,null,2,IntPtr.Zero,0,out var size,out _);if(size==0)throw new InvalidOperationException("Abilita Microsoft Print to PDF nelle funzionalità Windows. / Enable Microsoft Print to PDF in Windows features.");
        var data=Marshal.AllocHGlobal((int)size);try{if(!EnumPrintersW(2,null,2,data,size,out _,out var count))throw new Win32Exception();for(var i=0;i<count;i++){var p=Marshal.PtrToStructure<Printer>(data+i*Marshal.SizeOf<Printer>());if((Marshal.PtrToStringUni(p.Port) is "PORTPROMPT:" or "FILE:") && string.IsNullOrEmpty(Marshal.PtrToStringUni(p.Server)) && Marshal.PtrToStringUni(p.Driver)?.Equals("Microsoft Print To PDF",StringComparison.OrdinalIgnoreCase)==true)return Marshal.PtrToStringUni(p.Name)!;}}finally{Marshal.FreeHGlobal(data);}throw new InvalidOperationException("Microsoft Print to PDF non disponibile. / Microsoft Print to PDF is unavailable.");
    }
    internal static int Write(string input,string destination){
        var temp=Path.Combine(Path.GetDirectoryName(destination)!,".bibbia-"+Guid.NewGuid()+".pdf");var dc=CreateDCW("WINSPOOL",PdfPrinter(),IntPtr.Zero,IntPtr.Zero);if(dc==IntPtr.Zero)throw new Win32Exception();
        IntPtr font=IntPtr.Zero,old=IntPtr.Zero;var started=false;var pages=1;
        try {
            var dpi=GetDeviceCaps(dc,90);var margin=(int)(dpi*0.67);var width=GetDeviceCaps(dc,8);var height=GetDeviceCaps(dc,10);var fontHeight=Math.Max(1,dpi*11/72);
            font=CreateFontW(-fontHeight,0,0,0,400,0,0,0,1,0,0,0,0,"Segoe UI");if(font==IntPtr.Zero)throw new Win32Exception();old=SelectObject(dc,font);SetBkMode(dc,1);if(GetTextMetricsW(dc,out var metrics))fontHeight=Math.Max(1,metrics.Height);
            var doc=new DocInfo{Size=Marshal.SizeOf<DocInfo>(),Name="Bibbia · Appunti",Output=temp};if(StartDocW(dc,ref doc)<=0)throw new Win32Exception();started=true;if(StartPage(dc)<=0)throw new Win32Exception();var y=margin;
            using(var reader=new StreamReader(input,Encoding.UTF8,true,8192)){
                while(reader.ReadLine() is {} line){if(line.Length==0){y+=fontHeight/2;continue;}var measure=new Rect{Left=margin,Top=0,Right=width-margin,Bottom=0};var total=DrawTextW(dc,line,line.Length,ref measure,0x10|0x400|0x800);if(total<=0)throw new InvalidOperationException("Impossibile impaginare il PDF / Cannot lay out PDF");var offset=0;
                    while(offset<total){if(y+fontHeight>height-margin){if(EndPage(dc)<=0||StartPage(dc)<=0)throw new Win32Exception();pages++;y=margin;}
                        var available=height-margin-y;var slice=Math.Min(total-offset,available);if(slice<total-offset){slice-=slice%fontHeight;if(slice<=0){y=height;continue;}}
                        var state=SaveDC(dc);IntersectClipRect(dc,margin,y,width-margin,y+slice);var rect=new Rect{Left=margin,Top=y-offset,Right=width-margin,Bottom=y-offset+total};DrawTextW(dc,line,line.Length,ref rect,0x10|0x800);RestoreDC(dc,state);offset+=slice;y+=slice;
                    }
                    y+=fontHeight/4;
                }
            }
            if(EndPage(dc)<=0||EndDoc(dc)<=0)throw new Win32Exception();started=false;
            var deadline=DateTime.UtcNow.AddSeconds(30);var magic=new byte[5];while(true){try{using var pdf=new FileStream(temp,FileMode.Open,FileAccess.Read,FileShare.Read);if(pdf.Read(magic)==5 && magic.AsSpan().SequenceEqual("%PDF-"u8))break;}catch(IOException){}if(DateTime.UtcNow>=deadline)throw new IOException("PDF non creato dal servizio di stampa / The print service did not create the PDF");Thread.Sleep(100);}
            File.Move(temp,destination,true);return pages;
        }finally{if(started)AbortDoc(dc);if(old!=IntPtr.Zero)SelectObject(dc,old);if(font!=IntPtr.Zero)DeleteObject(font);DeleteDC(dc);try{File.Delete(temp);}catch(IOException){}}
    }
}
