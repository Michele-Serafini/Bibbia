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
    var boot = BibleEngine.Call(new { op = "init", directory });
    Check(boot.GetProperty("defaultEdition").GetString() == "Diodati 1649", "Traduzione italiana iniziale mancante.");
    var verses = BibleEngine.Call(new { op = "chapter", edition = "Diodati 1649", book = 0, chapter = 0 });
    Check(verses.GetArrayLength() == 31, "Genesi 1 incompleta.");
    Check(verses[0].GetProperty("text").GetString()!.Contains("Iddio creò"), "Testo UTF-8 non corretto.");
    var annotation = new { highlight = "blue", underline = true, bookmark = true, note = "Perché c’è speranza 🌱" };
    BibleEngine.Call(new { op = "annotate", edition = "Diodati 1649", book = 0, chapter = 0, verse = 0, annotation });
    BibleEngine.Call(new { op = "close" });
    BibleEngine.Call(new { op = "init", directory });
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
