using Bibbia.Windows;
using System.Runtime.InteropServices;
using System.Text.Json;

if (args.Length != 1) throw new ArgumentException("Indica il percorso della libreria Rust compilata.");
var library = NativeLibrary.Load(Path.GetFullPath(args[0]));
NativeLibrary.SetDllImportResolver(typeof(BibleEngine).Assembly, (name, _, _) => name == "bible_core" ? library : IntPtr.Zero);
var directory = Path.Combine(Path.GetTempPath(), "bibbia-interop-" + Guid.NewGuid());
void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
try
{
    var top=PageTurnMath.Project(800,600,800,0,0.22,true);
    var bottom=PageTurnMath.Project(800,600,800,600,0.22,true);
    Check(top.Z>30 && top.X<790 && top.Y>0,"The upper corner must lift first.");
    Check(Math.Abs(bottom.X-800)<0.001 && Math.Abs(bottom.Y-600)<0.001 && bottom.Z==0,"The bottom must follow the upper corner.");
    foreach (var (w,h) in new[]{(800d,600d),(420d,900d),(1400d,400d)}) {
        foreach (var forward in new[]{false,true}) {
            var sign=forward?1d:-1d;
            for (var k=0;k<=100;k++) {
                var f=PageTurnMath.Frame(w,h,k/100d,forward); var l=f.Layout;
                Check(f.Strips.Length==PageTurnMath.Strips,"Incomplete page.");
                for (var i=0;i<f.Strips.Length-1;i++) {
                    var a=f.Strips[i];var b=f.Strips[i+1];
                    Check(double.IsFinite(a.X) && double.IsFinite(a.Y) && double.IsFinite(a.ScaleX),"Nonfinite curl.");
                    Check(Math.Abs(a.X+sign*l.NX*l.Step*a.ScaleX-b.X)<0.001 && Math.Abs(a.Y+l.NY*l.Step*a.ScaleX-b.Y)<0.001,"A gap opened in the fold.");
                }
            }
            foreach (var x in new[]{0d,w/2,w}) foreach (var y in new[]{0d,h/2,h}) {
                var flat=PageTurnMath.Project(w,h,x,y,0,forward);
                Check(Math.Abs(flat.X-x)<0.001 && Math.Abs(flat.Y-y)<0.001,"Flat page was distorted.");
                var end=PageTurnMath.Project(w,h,x,y,1,forward);
                Check(end.X<0 || end.X>w || end.Y<0 || end.Y>h,"Turned page did not leave the viewport.");
            }
        }
    }
    var boot = BibleEngine.Call(new { op = "init", directory });
    Check(boot.GetProperty("defaultEdition").GetString() == "Martini 1781 (cattolica)", "Traduzione italiana iniziale mancante.");
    const string catholic = "Martini 1781 (cattolica)";
    Check(BibleEngine.Call(new {op="books",edition=catholic}).GetArrayLength() == 73, "Canone cattolico incompleto.");
    var mark = BibleEngine.Call(new {op="mark",edition=catholic,book=0,chapter=0,verse=0,start=3,length=9,action="underline"});
    Check(mark.GetProperty("ranges")[0].GetProperty("length").GetInt32() == 9, "Selezione parziale non conservata dal ponte nativo.");
    var verses = BibleEngine.Call(new { op = "chapter", edition = "Diodati 1649", book = 0, chapter = 0 });
    Check(verses.GetArrayLength() == 31, "Genesi 1 incompleta.");
    Check(verses[0].GetProperty("text").GetString()!.Contains("Iddio creò"), "Testo UTF-8 non corretto.");
    var annotation = new { highlight = "blue", underline = true, bookmark = true, note = "Perché c’è speranza 🌱" };
    BibleEngine.Call(new { op = "annotate", edition = "Diodati 1649", book = 0, chapter = 0, verse = 0, annotation });
    BibleEngine.Call(new { op = "close" });
    BibleEngine.Call(new { op = "init", directory });
    Check(BibleEngine.Call(new {op="chapter",edition=catholic,book=0,chapter=0})[0].GetProperty("annotation").GetProperty("ranges")[0].GetProperty("underline").GetBoolean(), "Sottolineatura parziale persa alla riapertura.");
    var saved = BibleEngine.Call(new { op = "saved", edition = "Diodati 1649" });
    Check(saved[0].GetProperty("annotation").GetProperty("note").GetString() == annotation.note, "Nota persa dopo riapertura.");
    var source = Path.Combine(directory, "traduzione.json");
    File.WriteAllText(source, """[{"abbrev":"gen","name":"Genesi","chapters":[["Una parola di speranza."]]}]""");
    var imported = BibleEngine.Call(new { op = "import", id = "  Italiano locale  ", path = source });
    var id = imported.GetProperty("id").GetString();
    Check(id == "Italiano locale", "Il motore non restituisce il nome canonico.");
    Check(BibleEngine.Call(new { op = "books", edition = id }).GetArrayLength() == 1, "Traduzione importata non leggibile.");
    var hits = BibleEngine.Call(new { op = "search", edition = id, query = "SPERANZA" }).GetProperty("hits");
    Check(hits.GetArrayLength() == 1, "Ricerca sulla traduzione importata non funzionante.");
    var rejected = false;
    try { BibleEngine.Call(new { op = "search", edition = id, query = "a" }); }
    catch (InvalidOperationException) { rejected = true; }
    Check(rejected, "Una richiesta errata deve produrre un errore gestito.");
    Check(BibleEngine.Call(new { op = "books" }).GetArrayLength() == 66, "Motore inutilizzabile dopo un errore.");
    BibleEngine.Call(new { op = "close" });
    Directory.Delete(directory, recursive: true);
    Console.WriteLine("C# → Rust: italiano, UTF-8, note, riapertura, import con spazi e ricerca verificati.");
}
finally
{
    BibleEngine.Call(new { op = "close" });
    NativeLibrary.Free(library);
    if (Directory.Exists(directory)) Directory.Delete(directory, recursive: true);
}
