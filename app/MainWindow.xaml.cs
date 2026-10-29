using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Windowing;
using System.Text.Json;
using Windows.Storage.Pickers;
using Windows.System;

namespace Bibbia.Windows;

public sealed class BookRow
{
    public required string Name { get; init; }
    public int Chapters { get; init; }
    public required int[] ChapterNumbers { get; init; }
}

public sealed class VerseRow : System.ComponentModel.INotifyPropertyChanged
{
    public event System.ComponentModel.PropertyChangedEventHandler? PropertyChanged;
    private JsonElement annotation;
    private bool focused;
    public bool Focused {get=>focused;set{if(focused==value)return;focused=value;PropertyChanged?.Invoke(this,new(nameof(MenuVisibility)));}}
    public Visibility MenuVisibility=>focused?Visibility.Collapsed:Visibility.Visible;
    public Visibility NoteVisibility => Annotation.TryGetProperty("note",out var note) && note.GetString() is {Length:>0} ? Visibility.Visible : Visibility.Collapsed;
    public string NoteHint => NoteVisibility==Visibility.Visible ? NoteLabel + " · " + Annotation.GetProperty("note").GetString()![..Math.Min(120,Annotation.GetProperty("note").GetString()!.Length)] : NoteLabel;
    private string noteLabel="Apri nota";
    public string NoteLabel {get=>noteLabel;set{if(noteLabel==value)return;noteLabel=value;PropertyChanged?.Invoke(this,new(nameof(NoteHint)));}}
    public string NoteId => $"VerseNote_{Index}";
    public int Index { get; init; }
    public required string Number { get; init; }
    public string MenuId => $"VerseMenu_{Index}";
    public required string Text { get; init; }
    public double TextSize {get;set;}
    public required JsonElement Annotation { get => annotation; set { annotation=value; PropertyChanged?.Invoke(this,new(nameof(NoteVisibility))); PropertyChanged?.Invoke(this,new(nameof(NoteHint))); } }
}

public sealed class SearchRow
{
    public int Book { get; init; }
    public int Chapter { get; init; }
    public int Verse { get; init; }
    public required string Reference { get; init; }
    public required string Text { get; init; }
    public required string Detail { get; init; }
    public string Note {get;init;}="";
    public JsonElement Annotation {get;init;}
    public Microsoft.UI.Xaml.Media.Brush NoteBackground {get;init;}=new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.Colors.Transparent);
    public string Folder {get;init;}="";
    public string[] Tags {get;init;}=[];
    public string OrganizeLabel {get;init;}="";
    public Visibility OrganizationVisibility {get;init;}=Visibility.Collapsed;
}

public sealed partial class MainWindow : Window
{
    private readonly string dataDirectory = PerformanceProfile() is {} profile?Path.Combine(profile,"notes"):Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "BibbiaRust");
    private List<BookRow> books = [];
    private List<VerseRow> readerRows = [];
    private readonly Dictionary<string,List<BookRow>> bookCatalogs = [];
    private Microsoft.UI.Dispatching.DispatcherQueueTimer? fontTimer;
    private string edition = "KJV";
    private int book, chapter, currentVerse, searchGeneration, savedOffset;
    private double fontSize = 22;
    private bool ready, changing, closed;
    private string section = "read";

    public MainWindow()
    {
        InitializeComponent();
        InitializeBookMotion();
        InitializeOrganization();
        InitializeReadingPlans();
        InitializeTransfer();
        InitializeBookPagination();
        InitializeFocus();
        ConfigureWindowCaption();
        Navigation.IsEnabled = false;
        var area = DisplayArea.GetFromWindowId(AppWindow.Id, DisplayAreaFallback.Primary).WorkArea;
        var width = Math.Min(1180, area.Width - 32);
        var height = Math.Min(820, area.Height - 32);
        AppWindow.MoveAndResize(new global::Windows.Graphics.RectInt32(
            area.X + (area.Width-width)/2, area.Y + (area.Height-height)/2, width, height));
        AppWindow.SetIcon(Path.Combine(AppContext.BaseDirectory, "Assets", "Bibbia.ico"));
        Root.ActualThemeChanged += (_, _) => UpdateWindowTheme();
        Root.Loaded += OnLoaded;
        Closed += (_, _) =>
        {
            if (ready && !comparing && bookChapter == $"{edition}/{book}/{chapter}") {
                try { SaveBookPlace(); } catch (Exception e) { System.Diagnostics.Debug.WriteLine(e); }
            }
            closed = true;
            ready = false;
            searchGeneration++;
            chapterRenderGeneration++;
            CancelBookTurn();
            selectionTimer?.Stop();
            fontTimer?.Stop();
            bookLayoutTimer?.Stop();
            notesTimer?.Stop();notesGeneration++;
            try {
                BibleEngine.Call(new {op="settings",key="fontSize",value=(int)fontSize});
                BibleEngine.Call(new {op="settings",key="readingMargin",value=readingMargin});
                BibleEngine.Call(new { op = "close" });
            }
            catch (Exception e) { System.Diagnostics.Debug.WriteLine(e); }
        };
    }

    private void ShowError(Exception e)
    {
        if (closed) return;
        TraceBook("error " + e);
        ErrorBar.Message = T(e.Message);
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
                readingTheme = settings.TryGetProperty("theme",out var t)?t.GetString()!:"Default";
                SetReadingTheme();
                ThemePicker.SelectedIndex=Array.IndexOf(new[]{"Default","Light","Dark","Sepia"},readingTheme);
                readingMargin=settings.TryGetProperty("readingMargin",out var m)?m.GetInt32():24;
                focusMode=settings.TryGetProperty("focusMode",out var fm)&&fm.GetBoolean();
                MarginSlider.Value=readingMargin;
                FontSlider.Value = fontSize;
                InitializeLanguages(boot);
            }
            finally { changing = false; }
            ApplyLanguage();
            UpdateWindowTheme();
            LoadBooks();
            ready = true;
            ApplyReadingMargins();
            ApplyFocus();
            LoadChapter(currentVerse);
            Navigation.IsEnabled = true;
            await TransferSmoke();
            if(PerformanceProfile() is {} profile)_=GuardAsync(async()=>{
                try {await RunPerformance(profile);}catch(Exception error){File.WriteAllText(Path.Combine(profile,"error.txt"),error.ToString());Close();}
            });
        });
    }

    private void LoadBooks()
    {
        if (!bookCatalogs.TryGetValue(edition,out var catalog)) {
            var data = BibleEngine.Call(new { op = "books", edition });
            catalog = data.EnumerateArray().Select(x => new BookRow { Name=x.GetProperty("name").GetString()!,Chapters=x.GetProperty("chapters").GetInt32(),ChapterNumbers=Enumerable.Range(1,x.GetProperty("chapters").GetInt32()).ToArray() }).ToList();
            bookCatalogs[edition] = catalog;
        }
        books = catalog;
        book = Math.Clamp(book, 0, books.Count - 1);
        chapter = Math.Clamp(chapter, 0, books[book].Chapters - 1);
        changing = true;
        try
        {
            BookPicker.ItemsSource = books;
            BookPicker.SelectedIndex = book;
            ChapterPicker.ItemsSource = books[book].ChapterNumbers;
            ChapterPicker.SelectedIndex = chapter;
        }
        finally { changing = false; }
    }

    private void LoadChapter(int focusVerse = -1) => _ = GuardAsync(() => LoadChapterAsync(focusVerse));
    private async Task LoadChapterAsync(int focusVerse)
    {
        var generation=++chapterRenderGeneration;
        var target=(Edition:edition,Book:book,Chapter:chapter);
        selectionTimer?.Stop();
        selectionEditor = null;
        activeSelectionFlyout?.Hide();
        activeSelectionFlyout = null;
        liveEditors.Clear();
        var animate = !comparing && section=="read" && displayedChapter!=null && displayedChapter!=target;
        var rows = BibleEngine.Call(new { op = "chapter", edition, book, chapter }).EnumerateArray().Select(x =>
            new VerseRow { Index = x.GetProperty("verse").GetInt32(), Number=x.GetProperty("label").GetString()!, Text = x.GetProperty("text").GetString()!,
                Focused=focusApplied, TextSize = fontSize, Annotation = x.GetProperty("annotation").Clone() }).ToList();
        VerseList.IsHitTestVisible=false;
        if (animate) BookPicker.Focus(FocusState.Programmatic);
        var snapshot=animate ? await CaptureBookPage() : null;
        if (closed || generation!=chapterRenderGeneration) return;
        var forward=displayedChapter==null || displayedChapter.Value.Edition!=edition ||
            (displayedChapter.Value.Book,displayedChapter.Value.Chapter).CompareTo((book,chapter))<0;
        readerRows=rows;
        if(!comparing)VerseList.ItemsSource=rows;
        VerseRight.ItemsSource=null;
        displayedChapter=target;
        VerseList.IsHitTestVisible=true;
        currentVerse = Math.Clamp(focusVerse, 0, rows.Count - 1);
        ReadingLabel.Text = T("Capitolo {0} di {1}",chapter+1,books[book].Chapters);
        UpdatePageTitle();
        if (animate && section == "read") AnimateReveal(PageTitle,160);
        UpdateReadingStatus(rows.Count);
        PreviousButton.IsEnabled = book > 0 || chapter > 0;
        NextButton.IsEnabled = book < books.Count - 1 || chapter < books[book].Chapters - 1;
        SavePosition();
        ScrollToCurrentVerse();
        if(animate && section=="read" && bookMode){chapterSnapshot=snapshot;chapterForward=forward;ScheduleBookLayout();}
        if (animate && section == "read" && !bookMode) DispatcherQueue.TryEnqueue(() => {
            if (!closed && generation==chapterRenderGeneration) Guard(() => {
                try { TurnBookPage(snapshot,forward); }
                catch { CancelBookTurn(); throw; }
            });
        });
    }

    private void UpdateReadingStatus(int count)
    {
        QuickEditionButton.Content = edition + " ▾";
        var option = editionOptions.First(x => x.Id == edition);
        var detail = edition == "Raccolta estesa (83 sezioni)" ? T("83 sezioni in inglese: WEB ecumenica, 1 Enoc e Giubilei. Comprende 1–2 Esdra, 3–4 Maccabei, Salmo 151 e Preghiera di Manasse. È una raccolta di più canoni; non esiste un canone universale di tutti i libri.") : T("{0} libri · {1}",option.Books,option.Language == "it" ? "Italiano" : option.Language == "en" ? "English" : T("Tutte"));
        var attribution = edition == "CEI 2008" ? T("CEI 2008 · © 2008–2024 Fondazione di Religione Santi Francesco d’Assisi e Caterina da Siena. Uso formativo e didattico non commerciale. Fonte: BibbiaEDU. Ester: G = greco, E = ebraico.") : "";
        ApplyFocus();
        EditionDetails.Text = detail + (attribution.Length > 0 ? "\n" + attribution : "");
        StatusLabel.Text = T("Offline · {0} · {1} versetti · Seleziona parole o frasi per annotarle", edition, count) + (attribution.Length > 0 ? "\n" + attribution : "");
    }

    private void SavePosition() => BibleEngine.Call(new { op = "settings", key = "position", value = new { edition, book, chapter, verse = currentVerse } });

    private void ScrollToCurrentVerse()
    {
        if(comparing){LoadComparison();return;}
        if(bookMode){ScheduleBookLayout(true);return;}
        var rows=readerRows;if(rows.Count==0)return;
        VerseList.Visibility=Visibility.Visible;VerseList.IsHitTestVisible=true;VerseList.Clip=null;
        if(!ReferenceEquals(VerseList.ItemsSource,rows))VerseList.ItemsSource=rows;
        var target = currentVerse;
        // Ignore callbacks queued for a chapter that has since been replaced.
        DispatcherQueue.TryEnqueue(Microsoft.UI.Dispatching.DispatcherQueuePriority.Low,() =>
        {
            if (!closed && section == "read" && !comparing && ReferenceEquals(readerRows, rows))
                Guard(() => {
                    VerseList.UpdateLayout();
                    var scroll=Descendants<ScrollViewer>(VerseList).FirstOrDefault();
                    if(target==0)scroll?.ChangeView(null,0,null,true);
                    VerseList.ScrollIntoView(rows[target],ScrollIntoViewAlignment.Leading);
                    TraceBook($"reader items={VerseList.Items.Count} rows={rows.Count} visible={VerseList.Visibility} size={VerseList.ActualWidth}/{VerseList.ActualHeight} viewport={scroll?.ViewportHeight} extent={scroll?.ExtentHeight} offset={scroll?.VerticalOffset} target={target}");
                });
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
        if(TryTurnBookPage(-1))return;
        if (chapter > 0) chapter--; else if (book > 0) { book--; chapter = books[book].Chapters - 1; }
        LoadBooks(); LoadChapter();
    });
    private void NextClick(object sender, RoutedEventArgs e) => Guard(() =>
    {
        if (!ready) return;
        if(TryTurnBookPage(1))return;
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
        CancelBookTurn();
        var changed = section != tag;
        section = tag;
        searchGeneration++;
        ReadPanel.Visibility = tag == "read" ? Visibility.Visible : Visibility.Collapsed;
        ResultsPanel.Visibility = tag is "search" or "saved" ? Visibility.Visible : Visibility.Collapsed;
        SettingsPanel.Visibility = tag == "settings" ? Visibility.Visible : Visibility.Collapsed;
        if(readingPlansHost!=null)readingPlansHost.Visibility=tag=="plans"?Visibility.Visible:Visibility.Collapsed;
        if(tag=="plans" && ready)_=GuardAsync(LoadReadingPlans);
        SearchBar.Visibility = tag == "search" ? Visibility.Visible : Visibility.Collapsed;
        UpdatePageTitle();
        ApplyFocus();
        if (tag == "search")
        {
            ResultList.ItemsSource = null;
            ResultStatus.Text = T("Cerca in {0}. Fino a 200 risultati per ricerca.", edition);
            SearchBox.Focus(FocusState.Programmatic);
        }
        notesBar.Visibility=tag=="saved"?Visibility.Visible:Visibility.Collapsed;
        SavedPreviousPage.Visibility = SavedNextPage.Visibility = tag == "saved" ? Visibility.Visible : Visibility.Collapsed;
        if (tag == "saved" && ready) { savedOffset=0; LoadSaved(); }
        if (tag == "read" && ready) ScrollToCurrentVerse();
        if (changed) AnimateReveal(tag == "read" ? ReadPanel : tag == "settings" ? SettingsPanel : ResultsPanel);
    }
    private void AppearanceClick(object sender, RoutedEventArgs e) => Navigation.SelectedItem = Navigation.FooterMenuItems[0];

    private SearchRow ToSearchRow(JsonElement x, bool annotated)
    {
        var b = x.GetProperty("book").GetInt32(); var c = x.GetProperty("chapter").GetInt32(); var v = x.GetProperty("verse").GetInt32();
        var detail = "";
        if (annotated)
        {
            var a = x.GetProperty("annotation");
            List<string> labels = [];
            if (a.GetProperty("bookmark").GetBoolean()) labels.Add(T("Segnalibro"));
            if (a.GetProperty("highlight").ValueKind == JsonValueKind.String || a.GetProperty("ranges").EnumerateArray().Any(m => m.GetProperty("highlight").ValueKind == JsonValueKind.String)) labels.Add(T("Evidenziato"));
            if (a.GetProperty("underline").GetBoolean() || a.GetProperty("ranges").EnumerateArray().Any(m => m.GetProperty("underline").GetBoolean())) labels.Add(T("Sottolineato"));
            if (a.GetProperty("note").GetString() is { Length: > 0 } note) labels.Add(note);
            detail = string.Join(" · ", labels);
        }
        var folder=annotated && x.TryGetProperty("organization",out var org)?org.GetProperty("folder").GetString()!:"";
        var tags=annotated && x.TryGetProperty("organization",out org)?org.GetProperty("tags").EnumerateArray().Select(v=>v.GetString()!).ToArray():[];
        if(annotated)detail=string.Join(" · ",new[]{detail,folder,string.Join(" ",tags.Select(t=>"#"+t))}.Where(s=>s.Length>0));
        return new SearchRow { Note=annotated?x.GetProperty("annotation").GetProperty("note").GetString()!:"",Annotation=annotated?x.GetProperty("annotation"):default,NoteBackground=SavedNoteBrush(annotated?x.GetProperty("annotation"):default),Folder=folder,Tags=tags,OrganizeLabel=N("Organizza…","Organize…"),OrganizationVisibility=annotated?Visibility.Visible:Visibility.Collapsed,Book = b, Chapter = c, Verse = v, Reference = $"{x.GetProperty("name").GetString()} {c + 1}:{x.GetProperty("label").GetString()}", Text = x.GetProperty("text").GetString()!, Detail = detail };
    }
    private void LoadSaved()=>_ = GuardAsync(LoadSavedNotes);
    private void SavedPreviousClick(object sender,RoutedEventArgs e) => Guard(() => { savedOffset=Math.Max(0,savedOffset-200); LoadSaved(); });
    private void SavedNextClick(object sender,RoutedEventArgs e) => Guard(() => { savedOffset+=200; LoadSaved(); });
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
        ResultStatus.Text = T("Ricerca in corso…");
        try
        {
            var result = await Task.Run(() => BibleEngine.Call(new { op = "search", edition = selectedEdition, query }));
            if (closed || generation != searchGeneration || section != "search") return;
            var rows = result.GetProperty("hits").EnumerateArray().Select(x => ToSearchRow(x, false)).ToList();
            ResultList.ItemsSource = rows;
            AnimateReveal(ResultList,140);
            ResultStatus.Text = result.GetProperty("truncated").GetBoolean() ? T("Primi 200 risultati. Affina la ricerca per trovarne altri.") : T("{0} risultati · {1}", rows.Count, selectedEdition);
        }
        catch (Exception e)
        {
            if (!closed && generation == searchGeneration && section == "search")
                ResultStatus.Text = T(e.Message);
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

    private void ThemeChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!ready || changing) return;
        Guard(() =>
        {
            var theme = ((ComboBoxItem)ThemePicker.SelectedItem).Tag.ToString()!;
            BibleEngine.Call(new { op = "settings", key = "theme", value = theme });
            readingTheme=theme;
            SetReadingTheme();
            UpdateWindowTheme();
        });
    }
    private void FontChanged(object sender, Microsoft.UI.Xaml.Controls.Primitives.RangeBaseValueChangedEventArgs e)
    {
        if (!ready || changing) return;
        CancelBookTurn();
        fontSize = e.NewValue;
        if (fontTimer == null) {
            fontTimer = DispatcherQueue.CreateTimer(); fontTimer.Interval = TimeSpan.FromMilliseconds(100); fontTimer.IsRepeating = false;
            fontTimer.Tick += (_,_) => Guard(() => {
                if (closed) return;
                foreach (var row in readerRows) row.TextSize=fontSize;
                foreach(var editor in Descendants<RichTextBlock>(BookColumns)){editor.FontSize=fontSize;editor.LineHeight=fontSize*1.35;}
                ScheduleBookLayout();
                if(comparing)RenderComparison();
                BibleEngine.Call(new {op="settings",key="fontSize",value=(int)fontSize});
            });
        }
        if (!fontTimer.IsRunning) fontTimer.Start();
    }
    private void EditionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!ready || changing || EditionPicker.SelectedItem is not EditionOption selected) return;
        Guard(() => { searchGeneration++; edition = selected.Id; book = chapter = 0; LoadBooks(); LoadChapter(); });
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
        var name = new TextBox { Header = T("Nome della traduzione"), Text = Path.GetFileNameWithoutExtension(file.Name), MaxLength = 80 };
        var rights = new CheckBox { Content = T("Ho il diritto di utilizzare questo testo.") };
        var content = new StackPanel { Spacing = 12 };
        content.Children.Add(new TextBlock { Text = T("Formato: elenco di libri con name, abbrev e chapters (elenchi di versetti). Massimo 32 MB. La traduzione sarà copiata nel database locale."), TextWrapping = TextWrapping.Wrap });
        content.Children.Add(name); content.Children.Add(rights);
        var dialog = new ContentDialog { XamlRoot = Root.XamlRoot, RequestedTheme = Root.RequestedTheme, Title = T("Importa traduzione"), Content = content, PrimaryButtonText = T("Importa"), CloseButtonText = T("Annulla"), IsPrimaryButtonEnabled = false };
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
            edition = id;
            bibleLanguage = "all";
            BibleEngine.Call(new {op="settings",key="bibleLanguage",value=bibleLanguage});
            InitializeEditionOptions(boot);
            RefreshEditionPicker();
        }
        finally { changing = false; }
        edition = id; book = chapter = 0;
        LoadBooks(); LoadChapter();
        StatusLabel.Text = T("{0} importata. Apri Leggi per iniziare.", id);
    });
}
