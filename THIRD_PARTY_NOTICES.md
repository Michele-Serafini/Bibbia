# Testi e dipendenze

## Allineamento dei riferimenti

Le tabelle dei riferimenti dei Salmi derivano da TVTMS di **STEPBible.org,
Tyndale House Cambridge**, [STEPBible-Data](https://github.com/STEPBible/STEPBible-Data/tree/master/Versification),
con licenza [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
L'estratto raggruppa i riferimenti equivalenti e adatta i titoli numerati ai
conteggi dei testi inclusi. Le corrispondenze non verificate restano segnalate,
senza modificare il testo biblico. Provenienza e modifiche sono documentate in
`comparison-source.json`, `LICENSE.comparison.txt` e `scripts/prepare-comparison.py`.

## Applicazione macOS

La versione macOS è sviluppata in Swift con SwiftUI, AppKit, Foundation e SQLite
forniti da macOS. Usa i font di sistema Apple tramite le API del sistema, senza
redistribuirne i file. Non include il motore Rust, GTK, WinUI, .NET o i font GNOME
delle altre versioni. Il sorgente vettoriale dell’icona macOS è parte del progetto,
con la stessa licenza MIT del codice. Le condizioni dei testi biblici sotto
si applicano anche al pacchetto macOS.

## CEI 2008, cattolica italiana

Testo dei **73 libri** dalla fonte ufficiale [BibbiaEDU](https://www.bibbiaedu.it/CEI2008/),
1.328 capitoli. © 2008–2024 Fondazione di Religione Santi Francesco d’Assisi e
Caterina da Siena. Traduzione della Conferenza Episcopale Italiana, 2008.
**Non è pubblico dominio e non è MIT**. L’inclusione nel lettore gratuito per
finalità formative e didattiche non commerciali segue le condizioni pubblicate
nelle [note legali ufficiali](https://www.bibbiaedu.it/site/twig/templates/v3/html/notelegali/).
Il testo delle condizioni è conservato in `core/data/LICENSE.cei2008.txt`, incluso
nell’installer e mostrato nell’installazione. Per altri usi occorre l’autorizzazione
degli aventi diritto; la licenza del codice non concede diritti sul testo.

Sono estratti esclusivamente i versetti, senza commenti, loghi, grafica o software
del sito. Entità HTML decodificate e spazi di impaginazione normalizzati; il testo
non è riscritto. Le etichette conservano le lettere e i gruppi originali. Ester
comprende sia il testo greco sia quello ebraico presentati dalla fonte: i prefissi
**G/E** distinguono i riferimenti e sono spiegati nel lettore. Il manifest
`cei2008-source.json` fissa URL e SHA-256 di ciascuna delle 1.328 pagine e del JSON.
`scripts/import-cei.py` riproduce la conversione da una cache verificata.

## Raccolta estesa in inglese, 83 libri/sezioni

Questa è una **raccolta di testi di più canoni**, non un canone universale né
l’intera Bibbia etiopica. I testi sono in inglese. Comprende:

- Tutti gli **81 libri/sezioni della World English Bible Classic ecumenica**:
  Antico e Nuovo Testamento, deuterocanonici, 1–2 Esdra, 3–4 Maccabei,
  Preghiera di Manasse, Salmo 151 e versioni greche di Ester e Daniele.
  [Testo e diritti ufficiali](https://ebible.org/eng-web/copyright.htm): pubblico
  dominio. World English Bible è un marchio di eBible.org. Il testo WEB rimane
  identico alla fonte XML ufficiale, senza note o titoli intercalati. I record vuoti
  della fonte sono omessi: non vengono inventati versetti per riempire le lacune.
  I versetti combinati mantengono la propria etichetta originale.
- **1 Enoc**, 108 capitoli, traduzione R. H. Charles del 1917, pubblico dominio.
  [Fonte Project Gutenberg, ebook 77935](https://www.gutenberg.org/ebooks/77935),
  digitalizzazione di David King e Distributed Proofreaders. Sono esclusi
  introduzione, apparato critico e numeri di pagina. I capitoli sono riordinati
  numericamente; le etichette originali, comprese quelle con lettere, sono conservate.
  La licenza Project Gutenberg è inclusa in `LICENSE.gutenberg.txt`.
- **Giubilei**, 50 capitoli, traduzione R. H. Charles, Londra 1917, pubblico dominio
  come dichiarato nei metadati della specifica edizione [Sefaria](https://www.sefaria.org/Book_of_Jubilees.1?ven=The_Book_of_Jubilees,_trans._R._H._Charles._London_[1917]).
  Si usa la suddivisione della fonte in 1.758 segmenti, che può differire da altre
  edizioni. Non si usa la traduzione predefinita o un testo moderno diverso.

`extended-source.json` conserva URL e checksum delle fonti e del corpus incluso;
`scripts/import-extended.py` riproduce la conversione. I testi non sono tradotti
artificialmente in italiano. Numeri di record e numero dei versetti di altre
edizioni non sono necessariamente equivalenti.

## Antonio Martini (1781), cattolica

Traduzione italiana storica dalla Vulgata, di Antonio Martini (1720–1809),
arcivescovo di Firenze. Testo di **pubblico dominio**, con 73 libri, 1.334 capitoli
e 35.763 versetti, compresi i deuterocanonici e le aggiunte a Ester e Daniele.
Conserva la numerazione della Vulgata, che può differire dalle edizioni moderne.

Fonte: repository ufficiale LaParola, commit fissato
`207f6a574ffa5913790e27962f5e9e180ce1b47c`:
https://github.com/LPN6/LaParola/tree/207f6a574ffa5913790e27962f5e9e180ce1b47c/Sito/Pagine/app/content/texts/martini

La dichiarazione della fonte «Rights: Dominio pubblico» è in
[about.html](https://github.com/LPN6/LaParola/blob/207f6a574ffa5913790e27962f5e9e180ce1b47c/Sito/Pagine/app/content/texts/martini/about.html).
Scaricato il 24 ottobre 2026. Ogni capitolo HTML è stato verificato rispetto al
proprio hash Git. La conversione estrae i testi dei soli versetti numerati,
decodifica le entità HTML e normalizza gli spazi, escludendo i titoli esterni.
SHA-256 del JSON incluso:
`3865f59c82b3104cb57d6a51974c8c8ed2169eccf662214a617ce8ed283c3d62`.
I metadati sono conservati in `core/data/martini-source.json` e nell'installer.

## Giovanni Diodati (1649)

Testo storico italiano di pubblico dominio. Sono inclusi 66 libri, 1.189 capitoli
e 31.102 versetti. Fonte testuale Unbound Bible, archivio getBible:
https://github.com/getbible/Unbound-Biola/blob/38a275eef7e15a398f6ec08239abc75ed8713456/Italian__Giovanni_Diodati_Bible_(1649)__giovanni__LTR.txt

La dichiarazione `distribution_license: Public Domain` e i dettagli della fonte
sono conservati in `core/data/diodati-source.json`. Metadati di getBible:
https://github.com/getbible/v2/blob/58ccb95982059a3aae36ebde2a84b99fa34742ab/translations.json

Scaricato il 24 ottobre 2026. SHA-256 della fonte:
`b0d13b1d50c5b12665dfedac03432e2be82e17ad3f833d33f6983c6319109a97`.
SHA-256 del JSON incluso:
`76f397aa95e71c8a01bc94b223586fbd15302cb987477f12e681fc1df1082002`.
La conversione riordina i record per libro, capitolo e versetto senza riscrivere
i testi; verifica che tutti gli indici siano consecutivi. I nomi italiani dei
libri provengono dai metadati `giovanni/books.json` di getBible.

## King James Version (KJV)

Il corpus incluso contiene 66 libri, 1.189 capitoli e 31.102 versetti in inglese.
Fonte: https://github.com/thiagobodruk/bible/blob/master/json/en_kjv.json
Scaricato il 24 ottobre 2026. SHA-256 del file distribuito:
`f627b3c24a617013a4514ab24b6dd1bb305b09af237994c018f74cb9c7e71f41`.

Il testo storico KJV è di pubblico dominio nella maggior parte dei Paesi;
nel Regno Unito la sua pubblicazione è soggetta alle prerogative della Corona.
Prima di distribuire in tale territorio, verificare le condizioni applicabili.
La licenza MIT del dataset è conservata in `core/data/LICENSE.bible-data`.
La licenza del contenitore dati non sostituisce i diritti delle singole traduzioni.

Le traduzioni importate dall'utente rimangono locali. Nessun testo, logo, asset o
servizio di Bible.com/YouVersion è incorporato. Questo progetto non è affiliato a
YouVersion. Una futura integrazione richiede accesso ufficiale e i relativi diritti.

## Software

- Rust: https://www.rust-lang.org/ — MIT / Apache-2.0.
- rusqlite: https://github.com/rusqlite/rusqlite — MIT; SQLite incluso, pubblico dominio.
- serde, serde_json: https://serde.rs/ — MIT / Apache-2.0.
- Windows App SDK / WinUI: https://github.com/microsoft/WindowsAppSDK — MIT e condizioni dei componenti redistribuiti.
- .NET: https://github.com/dotnet/runtime — MIT e avvisi delle dipendenze.
- NSIS: https://nsis.sourceforge.io/ — installer e Modern UI, licenze zlib/libpng e avvisi dei componenti. Il compilatore è uno strumento di build, non un prerequisito per l'utente.

Per distribuire il pacchetto self-contained conservare anche gli avvisi forniti
dai runtime e dalle dipendenze. `Cargo.lock` fissa le dipendenze Rust.

## GNOME e font

L’interfaccia Linux usa GTK4, libadwaita, GLib/GIO e Pango di sistema, con i
binding Rust gtk-rs/adwaita-rs distribuiti con licenza MIT. GTK4, libadwaita,
GLib e Pango sono librerie LGPL, collegate dinamicamente e risolte dal gestore
pacchetti. Fontconfig ha la propria licenza permissiva. Le librerie di sistema
non sono incorporate nei pacchetti dell’applicazione.

- **Adwaita Sans 49.0**, font GNOME basato su Inter, SIL Open Font License 1.1.
  Fonte ufficiale: https://download.gnome.org/sources/adwaita-fonts/49/adwaita-fonts-49.0.tar.xz
  L’archivio è stato verificato con il checksum SHA-256 pubblicato da GNOME.
- **Source Serif 4**, font di lettura di Adobe, SIL Open Font License 1.1.
  Fonte fissata: https://github.com/adobe-fonts/source-serif/tree/80d3f8894c09c937bebfa9011247d2e1c79fd6f4

I font sono distribuiti integri, con copyright e licenze originali in
`assets/fonts/LICENSE.Adwaita` e `assets/fonts/LICENSE.SourceSerif`. Il manifest
`assets/fonts/sources.json` conserva origini, versioni e checksum. I pacchetti
Windows e Linux includono questi avvisi. L’app usa i font al proprio interno,
senza sostituire i font del desktop o scrivere nelle directory dei font di sistema.

## Icona

L'icona della Bibbia in `app/Assets/` è stata creata per questo progetto;
è distribuita con la stessa licenza MIT del codice. Non usa il logo YouVersion.
