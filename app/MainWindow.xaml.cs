using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using System.Text.Json;
using Windows.ApplicationModel.DataTransfer;
using Windows.Storage.Pickers;
using Windows.System;
using Windows.UI.Text;

namespace Bibbia.Windows;

public sealed class BookRow
{
    public required string Name { get; init; }
    public int Chapters { get; init; }
}

public sealed class VerseRow
{
    public int Index { get; init; }
    public string Number => (Index + 1).ToString();
    public required string Text { get; init; }
    public double TextSize { get; init; }
    public required Brush Fill { get; init; }
    public TextDecorations Decorations { get; init; }
    public required string Marker { get; init; }
    public required JsonElement Annotation { get; init; }
}

public sealed class SearchRow
{
    public int Book { get; init; }
    public int Chapter { get; init; }
    public int Verse { get; init; }
    public required string Reference { get; init; }
    public required string Text { get; init; }
    public required string Detail { get; init; }
}

public sealed partial class MainWindow : Window
{
    private readonly string dataDirectory = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "BibbiaRust");
    private List<BookRow> books = [];
    private string edition = "KJV";
    private int book, chapter, currentVerse, searchGeneration;
    private double fontSize = 20;
    private bool ready, changing, dialogOpen, closed;
    private string section = "read";

    public MainWindow()
    {
        InitializeComponent();
        Navigation.IsEnabled = false;
        AppWindow.Resize(new global::Windows.Graphics.SizeInt32(1180, 820));
        Root.Loaded += OnLoaded;
        Closed += (_, _) =>
        {
            closed = true;
            ready = false;
            searchGeneration++;
            try { BibleEngine.Call(new { op = "close" }); }
            catch (Exception e) { System.Diagnostics.Debug.WriteLine(e); }
        };
    }

    private void ShowError(Exception e)
    {
        if (closed) return;
        ErrorBar.Message = e.Message;
        ErrorBar.IsOpen = true;
    }
    private void Guard(Action action)
    {
        try { action(); } catch (Exception e) { ShowError(e); }
    }
    private async Task GuardAsync(Func<Task> action)
    {
        try { await action(); } catch (Exception e) { ShowError(e); }
    }

    private async void OnLoaded(object sender, RoutedEventArgs e)
    {
        Root.Loaded -= OnLoaded;
        await GuardAsync(async () =>
        {
            var boot = await Task.Run(() => BibleEngine.Call(new { op = "init", directory = dataDirectory }));
            if (closed) { BibleEngine.Call(new { op = "close" }); return; }
            edition = boot.GetProperty("defaultEdition").GetString()!;
            changing = true;
            try
            {
                var editions = boot.GetProperty("editions").EnumerateArray().Select(x => x.GetString()!).ToList();
                var settings = boot.GetProperty("settings");
                if (settings.TryGetProperty("position", out var p) && editions.Contains(p.GetProperty("edition").GetString()!))
                {
                    edition = p.GetProperty("edition").GetString()!;
                    book = p.GetProperty("book").GetInt32(); chapter = p.GetProperty("chapter").GetInt32();
                    if (p.TryGetProperty("verse", out var v)) currentVerse = v.GetInt32();
                }
                if (settings.TryGetProperty("fontSize", out var f)) fontSize = f.GetDouble();
                if (settings.TryGetProperty("theme", out var t))
                {
                    Root.RequestedTheme = Enum.Parse<ElementTheme>(t.GetString()!);
                    ThemePicker.SelectedIndex = Root.RequestedTheme == ElementTheme.Light ? 1 : Root.RequestedTheme == ElementTheme.Dark ? 2 : 0;
                }
                FontSlider.Value = fontSize;
                EditionPicker.ItemsSource = editions;
                EditionPicker.SelectedItem = edition;
            }
            finally { changing = false; }
            LoadBooks();
            ready = true;
            LoadChapter(currentVerse);
            Navigation.IsEnabled = true;
        });
    }

    private void LoadBooks()
    {
        var data = BibleEngine.Call(new { op = "books", edition });
        books = data.EnumerateArray().Select(x => new BookRow { Name = x.GetProperty("name").GetString()!, Chapters = x.GetProperty("chapters").GetInt32() }).ToList();
        book = Math.Clamp(book, 0, books.Count - 1);
        chapter = Math.Clamp(chapter, 0, books[book].Chapters - 1);
        changing = true;
        try
        {
            BookPicker.ItemsSource = books;
            BookPicker.SelectedIndex = book;
            ChapterPicker.ItemsSource = Enumerable.Range(1, books[book].Chapters).ToList();
            ChapterPicker.SelectedIndex = chapter;
        }
        finally { changing = false; }
    }

    private void LoadChapter(int focusVerse = -1)
    {
        var rows = BibleEngine.Call(new { op = "chapter", edition, book, chapter }).EnumerateArray().Select(x =>
        {
            var a = x.GetProperty("annotation");
            var color = a.GetProperty("highlight").GetString();
            var fill = color switch
            {
                "yellow" => global::Windows.UI.Color.FromArgb(65, 235, 183, 30),
                "green" => global::Windows.UI.Color.FromArgb(65, 75, 180, 100),
                "blue" => global::Windows.UI.Color.FromArgb(65, 65, 155, 235),
                "pink" => global::Windows.UI.Color.FromArgb(65, 220, 100, 160),
                _ => Colors.Transparent
            };
            return new VerseRow { Index = x.GetProperty("verse").GetInt32(), Text = x.GetProperty("text").GetString()!,
                TextSize = fontSize, Fill = new SolidColorBrush(fill), Annotation = a.Clone(),
                Decorations = a.GetProperty("underline").GetBoolean() ? TextDecorations.Underline : TextDecorations.None,
                Marker = a.GetProperty("bookmark").GetBoolean() ? "\uE8A4" : a.GetProperty("note").GetString()!.Length > 0 ? "\uE70B" : "" };
        }).ToList();
        VerseList.ItemsSource = rows;
        currentVerse = Math.Clamp(focusVerse, 0, rows.Count - 1);
        ReadingLabel.Text = $"{books[book].Name} {chapter + 1}";
        StatusLabel.Text = $"Offline · {edition} · {rows.Count} versetti · Seleziona un versetto per annotarlo";
        PreviousButton.IsEnabled = book > 0 || chapter > 0;
        NextButton.IsEnabled = book < books.Count - 1 || chapter < books[book].Chapters - 1;
        SavePosition();
        ScrollToCurrentVerse();
    }

    private void SavePosition() => BibleEngine.Call(new { op = "settings", key = "position", value = new { edition, book, chapter, verse = currentVerse } });

    private void ScrollToCurrentVerse()
    {
        if (VerseList.ItemsSource is not List<VerseRow> rows) return;
        var target = currentVerse;
        // Ignore callbacks queued for a chapter that has since been replaced.
        DispatcherQueue.TryEnqueue(() =>
        {
            if (!closed && section == "read" && ReferenceEquals(VerseList.ItemsSource, rows))
                Guard(() => VerseList.ScrollIntoView(rows[target], ScrollIntoViewAlignment.Leading));
        });
    }

    private void BookChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!ready || changing || BookPicker.SelectedIndex < 0) return;
        Guard(() => { book = BookPicker.SelectedIndex; chapter = 0; LoadBooks(); LoadChapter(); });
    }
    private void ChapterChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!ready || changing || ChapterPicker.SelectedIndex < 0) return;
        Guard(() => { chapter = ChapterPicker.SelectedIndex; LoadChapter(); });
    }
    private void PreviousClick(object sender, RoutedEventArgs e) => Guard(() =>
    {
        if (!ready) return;
        if (chapter > 0) chapter--; else if (book > 0) { book--; chapter = books[book].Chapters - 1; }
        LoadBooks(); LoadChapter();
    });
    private void NextClick(object sender, RoutedEventArgs e) => Guard(() =>
    {
        if (!ready) return;
        if (chapter + 1 < books[book].Chapters) chapter++; else if (book + 1 < books.Count) { book++; chapter = 0; }
        LoadBooks(); LoadChapter();
    });

    private void NavigationChanged(NavigationView sender, NavigationViewSelectionChangedEventArgs args)
    {
        if (args.SelectedItem is NavigationViewItem item && item.Tag is string tag && ReadPanel != null)
            Guard(() => ShowSection(tag));
    }
    private void ShowSection(string tag)
    {
        section = tag;
        searchGeneration++;
        ReadPanel.Visibility = tag == "read" ? Visibility.Visible : Visibility.Collapsed;
        ResultsPanel.Visibility = tag is "search" or "saved" ? Visibility.Visible : Visibility.Collapsed;
        SettingsPanel.Visibility = tag == "settings" ? Visibility.Visible : Visibility.Collapsed;
        SearchBar.Visibility = tag == "search" ? Visibility.Visible : Visibility.Collapsed;
        PageTitle.Text = tag switch { "search" => "Trova le parole.", "saved" => "Portale con te.", "settings" => "Come piace a te.", _ => "Leggi con calma." };
        if (tag == "search")
        {
            ResultList.ItemsSource = null;
            ResultStatus.Text = $"Cerca in {edition}. Fino a 200 risultati per ricerca.";
            SearchBox.Focus(FocusState.Programmatic);
        }
        if (tag == "saved" && ready) LoadSaved();
        if (tag == "read" && ready) ScrollToCurrentVerse();
    }
    private void AppearanceClick(object sender, RoutedEventArgs e) => Navigation.SelectedItem = Navigation.FooterMenuItems[0];

    private static SearchRow ToSearchRow(JsonElement x, bool annotated)
    {
        var b = x.GetProperty("book").GetInt32(); var c = x.GetProperty("chapter").GetInt32(); var v = x.GetProperty("verse").GetInt32();
        var detail = "";
        if (annotated)
        {
            var a = x.GetProperty("annotation");
            List<string> labels = [];
            if (a.GetProperty("bookmark").GetBoolean()) labels.Add("Segnalibro");
            if (a.GetProperty("highlight").ValueKind == JsonValueKind.String) labels.Add("Evidenziato");
            if (a.GetProperty("underline").GetBoolean()) labels.Add("Sottolineato");
            if (a.GetProperty("note").GetString() is { Length: > 0 } note) labels.Add(note);
            detail = string.Join(" · ", labels);
        }
        return new SearchRow { Book = b, Chapter = c, Verse = v, Reference = $"{x.GetProperty("name").GetString()} {c + 1}:{v + 1}", Text = x.GetProperty("text").GetString()!, Detail = detail };
    }
    private void LoadSaved()
    {
        var rows = BibleEngine.Call(new { op = "saved", edition }).EnumerateArray().Select(x => ToSearchRow(x, true)).ToList();
        ResultList.ItemsSource = rows;
        ResultStatus.Text = rows.Count == 0 ? $"Nessun appunto in {edition}. Seleziona un versetto durante la lettura per iniziare." : $"{rows.Count} versetti nella tua raccolta · {edition}";
    }
    private async void SearchClick(object sender, RoutedEventArgs e) => await GuardAsync(Search);
    private async void SearchKeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (e.Key == VirtualKey.Enter) { e.Handled = true; await GuardAsync(Search); }
    }
    private async Task Search()
    {
        if (!ready) return;
        var generation = ++searchGeneration;
        var query = SearchBox.Text;
        var selectedEdition = edition;
        ResultList.ItemsSource = null;
        ResultStatus.Text = "Ricerca in corso…";
        try
        {
            var result = await Task.Run(() => BibleEngine.Call(new { op = "search", edition = selectedEdition, query }));
            if (closed || generation != searchGeneration || section != "search") return;
            var rows = result.GetProperty("hits").EnumerateArray().Select(x => ToSearchRow(x, false)).ToList();
            ResultList.ItemsSource = rows;
            ResultStatus.Text = result.GetProperty("truncated").GetBoolean() ? "Primi 200 risultati. Affina la ricerca per trovarne altri." : $"{rows.Count} risultati · {selectedEdition}";
        }
        catch (Exception e)
        {
            if (!closed && generation == searchGeneration && section == "search")
                ResultStatus.Text = e.Message;
        }
    }
    private void ResultClicked(object sender, ItemClickEventArgs e)
    {
        if (e.ClickedItem is not SearchRow row) return;
        Guard(() =>
        {
            book = row.Book; chapter = row.Chapter;
            Navigation.SelectedItem = Navigation.MenuItems[0];
            LoadBooks(); LoadChapter(row.Verse);
        });
    }

    private async void VerseSelected(object sender, SelectionChangedEventArgs e)
    {
        if (!ready || dialogOpen || VerseList.SelectedItem is not VerseRow row) return;
        dialogOpen = true;
        await GuardAsync(async () =>
        {
            currentVerse = row.Index;
            SavePosition();
            var a = row.Annotation;
            var colors = new[] { "Nessuna", "Giallo", "Verde", "Blu", "Rosa" };
            var keys = new string?[] { null, "yellow", "green", "blue", "pink" };
            var color = new ComboBox { Header = "Evidenziazione", ItemsSource = colors, SelectedIndex = Array.IndexOf(keys, a.GetProperty("highlight").GetString()), MinWidth = 260 };
            var underline = new CheckBox { Content = "Sottolinea", IsChecked = a.GetProperty("underline").GetBoolean() };
            var bookmark = new CheckBox { Content = "Aggiungi ai segnalibri", IsChecked = a.GetProperty("bookmark").GetBoolean() };
            var note = new TextBox { Header = "La tua nota", Text = a.GetProperty("note").GetString(), AcceptsReturn = true, TextWrapping = TextWrapping.Wrap, MinHeight = 100, MaxLength = 8000 };
            var copy = new Button { Content = "Copia versetto" };
            copy.Click += (_, _) => Guard(() =>
            {
                var data = new DataPackage();
                data.SetText($"{books[book].Name} {chapter + 1}:{row.Index + 1} ({edition})\n{row.Text}");
                Clipboard.SetContent(data);
                copy.Content = "Copiato";
            });
            var content = new StackPanel { Spacing = 12, MinWidth = 300 };
            content.Children.Add(new TextBlock { Text = row.Text, TextWrapping = TextWrapping.Wrap, MaxHeight = 150 });
            content.Children.Add(color); content.Children.Add(underline); content.Children.Add(bookmark); content.Children.Add(note); content.Children.Add(copy);
            var dialog = new ContentDialog { XamlRoot = Root.XamlRoot, RequestedTheme = Root.RequestedTheme, Title = $"{books[book].Name} {chapter + 1}:{row.Index + 1}",
                Content = new ScrollViewer { Content = content }, PrimaryButtonText = "Salva", CloseButtonText = "Annulla", DefaultButton = ContentDialogButton.Primary };
            if (await dialog.ShowAsync() == ContentDialogResult.Primary)
            {
                BibleEngine.Call(new { op = "annotate", edition, book, chapter, verse = row.Index,
                    annotation = new { highlight = keys[Math.Max(0, color.SelectedIndex)], underline = underline.IsChecked == true, bookmark = bookmark.IsChecked == true, note = note.Text } });
                LoadChapter(row.Index);
            }
        });
        VerseList.SelectedItem = null;
        dialogOpen = false;
    }

    private void ThemeChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!ready || changing) return;
        Guard(() =>
        {
            var theme = ((ComboBoxItem)ThemePicker.SelectedItem).Tag.ToString()!;
            BibleEngine.Call(new { op = "settings", key = "theme", value = theme });
            Root.RequestedTheme = Enum.Parse<ElementTheme>(theme);
        });
    }
    private void FontChanged(object sender, Microsoft.UI.Xaml.Controls.Primitives.RangeBaseValueChangedEventArgs e)
    {
        if (!ready || changing) return;
        Guard(() => { fontSize = e.NewValue; BibleEngine.Call(new { op = "settings", key = "fontSize", value = (int)fontSize }); LoadChapter(currentVerse); });
    }
    private void EditionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!ready || changing || EditionPicker.SelectedItem is not string selected) return;
        Guard(() => { searchGeneration++; edition = selected; book = chapter = 0; LoadBooks(); LoadChapter(); });
    }
    private async void OpenDataClick(object sender, RoutedEventArgs e) => await GuardAsync(async () =>
    {
        if (!await Launcher.LaunchFolderPathAsync(dataDirectory)) throw new InvalidOperationException("Impossibile aprire la cartella dati.");
    });
    private async void ImportClick(object sender, RoutedEventArgs e) => await GuardAsync(async () =>
    {
        if (!ready) return;
        var picker = new FileOpenPicker();
        WinRT.Interop.InitializeWithWindow.Initialize(picker, WinRT.Interop.WindowNative.GetWindowHandle(this));
        picker.FileTypeFilter.Add(".json");
        var file = await picker.PickSingleFileAsync();
        if (file == null) return;
        var name = new TextBox { Header = "Nome della traduzione", Text = Path.GetFileNameWithoutExtension(file.Name), MaxLength = 80 };
        var rights = new CheckBox { Content = "Ho il diritto di utilizzare questo testo." };
        var content = new StackPanel { Spacing = 12 };
        content.Children.Add(new TextBlock { Text = "Formato: elenco di libri con name, abbrev e chapters (elenchi di versetti). Massimo 32 MB. La traduzione sarà copiata nel database locale.", TextWrapping = TextWrapping.Wrap });
        content.Children.Add(name); content.Children.Add(rights);
        var dialog = new ContentDialog { XamlRoot = Root.XamlRoot, RequestedTheme = Root.RequestedTheme, Title = "Importa traduzione", Content = content, PrimaryButtonText = "Importa", CloseButtonText = "Annulla", IsPrimaryButtonEnabled = false };
        rights.Checked += (_, _) => dialog.IsPrimaryButtonEnabled = true;
        rights.Unchecked += (_, _) => dialog.IsPrimaryButtonEnabled = false;
        if (await dialog.ShowAsync() != ContentDialogResult.Primary) return;
        var requestedId = name.Text;
        var path = file.Path;
        Navigation.IsEnabled = false;
        JsonElement imported;
        try { imported = await Task.Run(() => BibleEngine.Call(new { op = "import", id = requestedId, path })); }
        finally { if (!closed) Navigation.IsEnabled = true; }
        if (closed) return;
        var id = imported.GetProperty("id").GetString()!;
        var boot = BibleEngine.Call(new { op = "bootstrap" });
        changing = true;
        try
        {
            EditionPicker.ItemsSource = boot.GetProperty("editions").EnumerateArray().Select(x => x.GetString()!).ToList();
            EditionPicker.SelectedItem = id;
        }
        finally { changing = false; }
        edition = id; book = chapter = 0;
        LoadBooks(); LoadChapter();
        StatusLabel.Text = $"{id} importata. Apri Leggi per iniziare.";
    });
}
