using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.Text.Json;

namespace Bibbia.Windows;

public sealed record EditionOption(string Id, string Language, bool Catholic, int Books, string Canon)
{
    public string Name => Id;
    public override string ToString() => Name;
}

public sealed partial class MainWindow
{
    private string language = "it", bibleLanguage = "it";
    private List<EditionOption> editionOptions = [];
    private string T(string text, params object[] args) { var value = language == "en" ? English.GetValueOrDefault(text, text) : text; return args.Length == 0 ? value : string.Format(value,args); }
    private static readonly Dictionary<string, string> English = new()
    {
        ["Carta seppia"]="Sepia paper",
        ["Confronta"]="Compare", ["Seconda traduzione"]="Second translation", ["Caricamento confronto…"]="Loading comparison…",
        ["Apri e annota"]="Open and annotate",
        ["Nessun versetto corrispondente verificato."]="No verified matching verse.",
        ["Questo libro non è presente nella seconda traduzione."]="This book is absent from the second translation.",
        ["Numerazione diversa: corrispondenza non verificata per questo capitolo."]="Different numbering: no verified alignment for this chapter.",
        ["Alcuni riferimenti non hanno una corrispondenza verificata. I riferimenti originali sono conservati."]="Some references have no verified counterpart. Original references are preserved.",
        ["Versetti allineati · scorrimento sincronizzato · ↗ per leggere e annotare."]="Aligned verses · synchronized scrolling · ↗ to read and annotate.",
        ["Uso formativo e didattico non commerciale."]="Educational noncommercial use.",
        ["Appunti {0}–{1} · {2}"]="Annotations {0}–{1} · {2}",
        ["Pagina precedente di appunti"]="Previous annotations page", ["Pagina successiva di appunti"]="Next annotations page",
        ["Capitolo {0} di {1}"]="Chapter {0} of {1}",
        ["{0} libri · {1}"]="{0} books · {1}",
        ["83 sezioni in inglese: WEB ecumenica, 1 Enoc e Giubilei. Comprende 1–2 Esdra, 3–4 Maccabei, Salmo 151 e Preghiera di Manasse. È una raccolta di più canoni; non esiste un canone universale di tutti i libri."]="83 English sections: ecumenical WEB, 1 Enoch and Jubilees. Includes 1–2 Esdras, 3–4 Maccabees, Psalm 151 and Prayer of Manasseh. A collection across canons; there is no universal canon of all books.",
        ["CEI 2008 · © 2008–2024 Fondazione di Religione Santi Francesco d’Assisi e Caterina da Siena. Uso formativo e didattico non commerciale. Fonte: BibbiaEDU. Ester: G = greco, E = ebraico."]="CEI 2008 · © 2008–2024 Fondazione di Religione Santi Francesco d’Assisi e Caterina da Siena. Educational noncommercial use. Source: BibbiaEDU. Esther: G = Greek, E = Hebrew.",
        ["Leggi"]="Read", ["Cerca"]="Search", ["La mia raccolta"]="My collection", ["Aspetto e traduzioni"]="Appearance and translations",
        ["Leggi con calma."]="Take a moment to read.", ["Trova le parole."]="Find the words.", ["Portale con te."]="Keep them close.", ["Come piace a te."]="Make it yours.",
        ["UNO SPAZIO PER LA PAROLA"]="A PLACE FOR THE WORD", ["Il tuo modo di leggere"]="Your reading experience", ["Lingua dell’app"]="App language", ["Lingua della Bibbia"]="Bible language",
        ["Tema"]="Theme", ["Come Windows"]="Use Windows setting", ["Chiaro"]="Light", ["Scuro"]="Dark", ["Dimensione del testo"]="Text size", ["Traduzione"]="Translation", ["Tutte"]="All languages",
        ["Riduci a icona"]="Minimize", ["Ingrandisci"]="Maximize", ["Ripristina"]="Restore", ["Chiudi"]="Close",
        ["← Precedente"]="← Previous", ["Successivo →"]="Next →", ["Una parola o una frase…"]="A word or a phrase…", ["Libro"]="Book", ["Capitolo"]="Chapter",
        ["Offline · {0} · {1} versetti · Seleziona parole o frasi per annotarle"]="Offline · {0} · {1} verses · Select words or phrases to annotate",
        ["Cerca in {0}. Fino a 200 risultati per ricerca."]="Search {0}. Up to 200 results per search.", ["Ricerca in corso…"]="Searching…",
        ["{0} risultati · {1}"]="{0} results · {1}", ["Primi 200 risultati. Affina la ricerca per trovarne altri."]="First 200 results. Refine your search to see more.",
        ["Nessun appunto in {0}. Seleziona parole o frasi durante la lettura per iniziare."]="No annotations in {0}. Select words or phrases while reading to get started.",
        ["{0} versetti nella tua raccolta · {1}"]="{0} verses in your collection · {1}", ["Segnalibro"]="Bookmark", ["Evidenziato"]="Highlighted", ["Sottolineato"]="Underlined",
        ["Sottolinea"]="Underline", ["Evidenzia"]="Highlight", ["Giallo"]="Yellow", ["Verde"]="Green", ["Blu"]="Blue", ["Rosa"]="Pink", ["Rimuovi formattazione"]="Clear formatting",
        ["Copia"]="Copy", ["Nota"]="Note", ["La tua nota"]="Your note", ["Salva"]="Save", ["Annulla"]="Cancel", ["Annotazioni versetto {0}"]="Verse {0} annotations",
        ["Importa traduzione JSON…"]="Import JSON translation…", ["Apri cartella dati"]="Open data folder", ["I tuoi appunti restano sul dispositivo"]="Your notes stay on your device",
        ["Evidenziazioni, sottolineature, note, segnalibri e ultima lettura sono salvati in SQLite. Nessun account o tracciamento."]="Highlights, underlines, notes, bookmarks and reading position are stored locally. No account or tracking.",
        ["CEI 2008 e Martini: Bibbie cattoliche italiane complete, 73 libri. Diodati e KJV: 66 libri. Raccolta estesa in inglese: 83 libri/sezioni, compresi apocrifi, 1 Enoc e Giubilei. La lingua dell’app è indipendente dal testo."]="CEI 2008 and Martini: complete Italian Catholic Bibles, 73 books. Diodati and KJV: 66 books. Extended English collection: 83 books/sections, including apocrypha, 1 Enoch and Jubilees. App language and text language are independent.",
        ["Bibbia · {0}\nInterfaccia Windows nativa · Motore Rust\nProgetto indipendente, non affiliato a YouVersion."]="Bibbia · {0}\nNative Windows interface · Rust engine\nIndependent project, not affiliated with YouVersion.",
        ["Inserisci almeno due caratteri"]="Enter at least two characters", ["Ricerca troppo lunga"]="Search query is too long", ["Seleziona una parola o una frase"]="Select a word or phrase",
        ["Nome della traduzione"]="Translation name", ["Ho il diritto di utilizzare questo testo."]="I have the right to use this text.", ["Importa traduzione"]="Import translation", ["Importa"]="Import",
        ["Formato: elenco di libri con name, abbrev e chapters (elenchi di versetti). Massimo 32 MB. La traduzione sarà copiata nel database locale."]="Format: an array of books with name, abbrev and chapters (arrays of verses). Maximum 32 MB. The translation will be copied into the local database.",
        ["Impossibile aprire la cartella dati."]="Could not open the data folder.", ["Selezione non valida"]="Invalid text selection", ["Troppe annotazioni nel versetto"]="Too many annotations in this verse", ["Nota troppo lunga"]="Note is too long", ["Versetto non disponibile"]="Verse is not available",
        ["{0} importata. Apri Leggi per iniziare."]="{0} imported. Open Read to get started.", ["Traduzione già presente: scegli un nome diverso"]="Translation already exists: choose another name",
        ["Nome traduzione non valido o riservato"]="Invalid or reserved translation name", ["Struttura Bibbia non valida"]="Invalid Bible structure", ["File troppo grande (massimo 32 MB)"]="File is too large (maximum 32 MB)"
    };

    private void InitializeLanguages(JsonElement boot)
    {
        var settings = boot.GetProperty("settings");
        if(settings.TryGetProperty("comparisonEdition",out var other))comparisonEdition=other.GetString()??"CEI 2008";
        if (settings.TryGetProperty("language", out var saved)) language = saved.GetString()!;
        InitializeEditionOptions(boot);
        bibleLanguage = editionOptions.First(x => x.Id == edition).Language;
        if (bibleLanguage == "und") bibleLanguage = "all";
        if (settings.TryGetProperty("bibleLanguage", out var filter)) bibleLanguage = filter.GetString()!;
        if (!editionOptions.Any(x => x.Id == edition && (bibleLanguage == "all" || x.Language == bibleLanguage))) bibleLanguage = "all";
        AppLanguagePicker.SelectedIndex = language == "en" ? 1 : 0;
        RefreshEditionPicker();
    }
    private void InitializeEditionOptions(JsonElement boot) => editionOptions = boot.GetProperty("editionMetadata").EnumerateArray()
        .Select(x => new EditionOption(x.GetProperty("id").GetString()!, x.GetProperty("language").GetString()!, x.GetProperty("catholic").GetBoolean(), x.GetProperty("books").GetInt32(), x.GetProperty("canon").GetString()!))
        .OrderByDescending(x => x.Catholic).ThenBy(x => x.Id).ToList();

    private void RefreshEditionPicker()
    {
        var previous = changing;
        changing = true;
        try
        {
            BibleLanguagePicker.SelectedIndex = bibleLanguage == "it" ? 0 : bibleLanguage == "en" ? 1 : 2;
            var options = editionOptions.Where(x => bibleLanguage == "all" || x.Language == bibleLanguage).ToList();
            if (!options.Any(x => x.Id == edition)) edition = options[0].Id;
            EditionPicker.ItemsSource = options;
            EditionPicker.SelectedItem = options.First(x => x.Id == edition);
        }
        finally { changing = previous; }
    }
    private void QuickEditionsOpening(object sender, object e)
    {
        var menu = (MenuFlyout)sender;
        menu.Items.Clear();
        foreach (var option in editionOptions) {
            var item = new MenuFlyoutItem { Text=$"{option.Id} · {option.Books} · {(option.Language == "it" ? "Italiano" : option.Language == "en" ? "English" : T("Tutte"))}" };
            Microsoft.UI.Xaml.Automation.AutomationProperties.SetAutomationId(item,"QuickEdition_"+option.Id.Replace(" ",""));
            item.Click += (_,_) => Guard(() => {
                if (edition == option.Id) return;
                edition=option.Id; bibleLanguage=option.Language == "und" ? "all" : option.Language;
                BibleEngine.Call(new {op="settings",key="bibleLanguage",value=bibleLanguage});
                RefreshEditionPicker(); book=chapter=0; LoadBooks(); LoadChapter();
            });
            menu.Items.Add(item);
        }
    }
    private void UpdatePageTitle() => PageTitle.Text = section == "read" && books.Count > book
        ? $"{books[book].Name} {chapter+1}"
        : T(section switch {"search"=>"Trova le parole.","saved"=>"Portale con te.","settings"=>"Come piace a te.","plans"=>N("Piani di lettura","Reading plans"),_=>"Leggi con calma."});
    private void ApplyLanguage()
    {
        RelabelTransfer();
        RelabelFocus();
        for (int i=0;i<3;i++) ((NavigationViewItem)Navigation.MenuItems[i]).Content = T(new[]{"Leggi","Cerca","La mia raccolta"}[i]);
        ((NavigationViewItem)Navigation.FooterMenuItems[0]).Content = T("Aspetto e traduzioni");
        if(Navigation.MenuItems.OfType<NavigationViewItem>().FirstOrDefault(i=>i.Tag as string=="plans") is {} plansNav)plansNav.Content=N("Piani di lettura","Reading plans");
        EyebrowLabel.Text = T("UNO SPAZIO PER LA PAROLA"); AppearanceLabel.Text = T("Il tuo modo di leggere");
        AppLanguagePicker.Header = T("Lingua dell’app"); BibleLanguagePicker.Header = T("Lingua della Bibbia");
        ((ComboBoxItem)BibleLanguagePicker.Items[2]).Content = T("Tutte");
        ThemePicker.Header = T("Tema");
        for (int i=0;i<4;i++) ((ComboBoxItem)ThemePicker.Items[i]).Content = T(new[]{"Come Windows","Chiaro","Scuro","Carta seppia"}[i]);
        var wasChanging = changing; changing = true;
        try {
            // ComboBox caches its selection display when item content changes.
            var selectedTheme = ThemePicker.SelectedIndex;
            ThemePicker.SelectedIndex = -1; ThemePicker.SelectedIndex = selectedTheme;
            var selectedBibleLanguage = BibleLanguagePicker.SelectedIndex;
            BibleLanguagePicker.SelectedIndex = -1; BibleLanguagePicker.SelectedIndex = selectedBibleLanguage;
        } finally { changing = wasChanging; }
        RefreshCaptionButtons();
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(SavedPreviousPage,T("Pagina precedente di appunti"));
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(SavedNextPage,T("Pagina successiva di appunti"));
        ToolTipService.SetToolTip(SavedPreviousPage,T("Pagina precedente di appunti"));
        ToolTipService.SetToolTip(SavedNextPage,T("Pagina successiva di appunti"));
        FontSlider.Header = T("Dimensione del testo"); EditionPicker.Header = T("Traduzione");
        PreviousButton.Content = T("← Precedente"); NextButton.Content = T("Successivo →");
        BookModeToggle.Content = T("Libro");
        ComparisonToggle.Content=T("Confronta");
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(ComparisonEdition,T("Seconda traduzione"));
        if(comparing)RenderComparison();
        SearchBox.PlaceholderText = T("Una parola o una frase…"); SearchButton.Content = T("Cerca");
        CatalogDescription.Text = T("CEI 2008 e Martini: Bibbie cattoliche italiane complete, 73 libri. Diodati e KJV: 66 libri. Raccolta estesa in inglese: 83 libri/sezioni, compresi apocrifi, 1 Enoc e Giubilei. La lingua dell’app è indipendente dal testo.");
        ImportButton.Content = T("Importa traduzione JSON…"); OpenDataButton.Content = T("Apri cartella dati");
        PrivacyTitle.Text = T("I tuoi appunti restano sul dispositivo");
        PrivacyDescription.Text = T("Evidenziazioni, sottolineature, note, segnalibri e ultima lettura sono salvati in SQLite. Nessun account o tracciamento.");
        RelabelOrganization();
        AboutLabel.Text = T("Bibbia · {0}\nInterfaccia Windows nativa · Motore Rust\nProgetto indipendente, non affiliato a YouVersion.",typeof(App).Assembly.GetName().Version?.ToString(3)??"0.10.0");
        UpdatePageTitle();
        if (books.Count > book) ReadingLabel.Text = T("Capitolo {0} di {1}",chapter+1,books[book].Chapters);
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(BookPicker,T("Libro"));
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(ChapterPicker,T("Capitolo"));
    }
    private void AppLanguageChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!ready || changing || AppLanguagePicker.SelectedItem is not ComboBoxItem item) return;
        Guard(() => { language = item.Tag.ToString()!; BibleEngine.Call(new {op="settings",key="language",value=language}); ApplyLanguage(); UpdateReadingStatus(readerRows.Count); });
    }
    private void BibleLanguageChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!ready || changing || BibleLanguagePicker.SelectedItem is not ComboBoxItem item) return;
        Guard(() => {
            bibleLanguage = item.Tag.ToString()!;
            BibleEngine.Call(new {op="settings",key="bibleLanguage",value=bibleLanguage});
            var old = edition; RefreshEditionPicker();
            if (old != edition) book = chapter = 0;
            LoadBooks(); LoadChapter();
        });
    }
}
