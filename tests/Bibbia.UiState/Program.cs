using Bibbia.Windows;
using System.Runtime.InteropServices;

if (args.Length != 2) throw new ArgumentException("Indica DLL e cartella dati della prova UI.");
var directory = Path.GetFullPath(args[1]);
if (!File.Exists(Path.Combine(directory, "bible.sqlite3"))) throw new InvalidOperationException("Il database della prova UI non esiste.");
var library = NativeLibrary.Load(Path.GetFullPath(args[0]));
NativeLibrary.SetDllImportResolver(typeof(BibleEngine).Assembly, (name, _, _) => name == "bible_core" ? library : IntPtr.Zero);
void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
try
{
    var boot = BibleEngine.Call(new { op = "init", directory });
    var settings = boot.GetProperty("settings");
    Check(settings.GetProperty("language").GetString() == "en", "Lingua interfaccia non salvata.");
    Check(settings.GetProperty("bibleLanguage").GetString() == "it", "Lingua Bibbia non salvata.");
    Check(settings.GetProperty("theme").GetString() == "Dark", "Tema non salvato.");
    Check(settings.GetProperty("fontSize").GetInt32() == 26, "Dimensione testo non salvata.");
    const string edition = "Martini 1781 (cattolica)";
    Check(settings.GetProperty("position").GetProperty("edition").GetString() == edition, "Edizione cattolica non salvata.");
    var first = BibleEngine.Call(new { op = "chapter", edition, book = 0, chapter = 0 })[0];
    var annotation = first.GetProperty("annotation");
    var ranges = annotation.GetProperty("ranges").EnumerateArray().ToList();
    Check(ranges.Any(r => r.GetProperty("underline").GetBoolean()
        && r.GetProperty("length").GetInt32() > 0
        && r.GetProperty("length").GetInt32() < first.GetProperty("text").GetString()!.Length),
        "La selezione con mouse non ha salvato una sottolineatura parziale.");
    Check(!annotation.GetProperty("underline").GetBoolean(), "La selezione ha sottolineato tutto il versetto.");
    Check(annotation.GetProperty("bookmark").GetBoolean(), "Segnalibro UI non salvato.");
    Check(annotation.GetProperty("note").GetString() == "Nota UI: parole selezionate", "Nota UI non salvata.");
    Console.WriteLine("Dati UI riaperti: sottolineatura parziale, nota, segnalibro, lingue e tema persistono.");
}
finally
{
    BibleEngine.Call(new { op = "close" });
    NativeLibrary.Free(library);
}
