# Bibbia per Windows

Lettore biblico offline con interfaccia **WinUI 3 nativa**
e motore **Rust**. Interfaccia italiana o inglese, tema di Windows o chiaro/scuro,
caratteri ridimensionabili e navigazione adattiva. Nessun browser incorporato,
server o account. È un progetto indipendente, ispirato al flusso di lettura delle
app bibliche; non è un client ufficiale Bible.com/YouVersion.

## Funzioni implementate

- **CEI 2008**, Bibbia cattolica italiana completa di 73 libri, offline, per uso
  formativo e didattico non commerciale secondo le condizioni della fonte ufficiale.
- **Martini 1781**, cattolica italiana di 73 libri, selezionata al primo avvio;
  Diodati 1649 e KJV inglese, ciascuna di 66 libri.
- **Raccolta estesa di 83 libri/sezioni in inglese**: WEB ecumenica con apocrifi,
  più 1 Enoc e Giubilei (Charles 1917). Comprende 1–2 Esdra, 3–4 Maccabei,
  Salmo 151 e Preghiera di Manasse. Non è un canone universale né tutto il canone
  etiopico: non esiste un’unica Bibbia con tutti i canoni. Selettore rapido sotto la lettura.
- Scelta indipendente della lingua dell’app (italiano/inglese) e della Bibbia.
- Icona dedicata; barra del titolo dello stesso colore della finestra, in chiaro e scuro.
- Transizioni brevi su navigazione, capitoli e risultati; rispettano le animazioni
  disattivate nelle impostazioni Windows. Liste virtualizzate e menu creati all’apertura.
- Navigazione per libro e capitolo; ripresa dell'ultimo versetto selezionato.
- Ricerca senza distinzione maiuscole/minuscole nella traduzione selezionata;
  massimo 200 risultati con indicazione del limite.
- Selezione di parole o frasi con il mouse: barra contestuale compatta, quattro colori,
  sottolineatura, copia, segnalibro e nota in un piccolo popup. Il pulsante ⋯ offre
  anche le azioni sul versetto intero.
- Raccolta degli appunti con ritorno al versetto originale.
- Importazione di altre traduzioni JSON, anche italiane, senza sovrascrivere
  traduzioni esistenti o mescolare le relative annotazioni.
- Dati personali locali in SQLite, con transazioni e modalità WAL.

La Martini è una traduzione cattolica storica dalla Vulgata, di pubblico dominio;
conserva il linguaggio e la numerazione della sua edizione. Audio, piani di lettura,
sincronizzazione cloud e catalogo online
YouVersion non sono implementati. Non sono stati misurati tempi di avvio o RAM su
Windows; i benchmark del solo motore sono documentati sotto. Il runtime .NET e WinUI self-contained aumentano la dimensione del download.

## Architettura

`core/` contiene corpus, ricerca, importazione, validazione e persistenza in Rust.
`app/` è un piccolo host C#/.NET 8 che presenta i controlli WinUI 3 e chiama il
motore tramite due funzioni C ABI (`bible_request`, `bible_free`). Non è quindi
un'applicazione interamente Rust: questo confine permette di usare i controlli
WinUI ufficiali con un motore Rust indipendente e testabile.

## Installare su Windows

[Scarica **Bibbia-Setup-0.3.0-x64.exe**](https://github.com/Michele-Serafini/Bibbia/releases/download/v0.3.0/Bibbia-Setup-0.3.0-x64.exe): un solo installer offline,
con interfaccia italiana. Non richiede Rust, .NET, Visual Studio, account o
download aggiuntivi sul PC destinatario. Supporta Windows 10 versione 2004 o
successiva e Windows 11 a 64 bit.

1. Aprire l'installer e seguire la procedura. Non servono privilegi di amministratore.
2. Avviare **Bibbia** dal menu Start; il collegamento sul desktop è facoltativo.
3. Per aggiornare, chiudere Bibbia e aprire il nuovo installer. I dati personali restano al loro posto.
4. Per rimuovere l'app usare **Impostazioni Windows → App → Bibbia → Disinstalla**.
   Note, segnalibri e traduzioni importate vengono conservati.

La [release v0.3.0](https://github.com/Michele-Serafini/Bibbia/releases/tag/v0.3.0)
contiene l'installer e il checksum SHA-256. Per le nuove build il workflow
GitHub Actions **Check and build** produce anche l'artefatto **Bibbia-Installer-x64**.
Il workflow manuale **Publish verified installer** pubblica gli stessi file di una
build completata con successo, dopo averne ricontrollato il checksum, senza ricompilarli.

La pipeline verifica 19 test Rust, il collegamento C# → Rust, installazione,
aggiornamento e rimozione su Windows Server 2022. Le prove UI esercitano
selezione con mouse, popup, note, segnalibri, ricerca, lingue, CEI e testi estesi,
ridimensionamento del testo, animazioni, colore della barra e della superficie,
ingrandimento/ripristino e persistenza dopo la chiusura. L’esito della build
corrente è disponibile nella scheda Actions; le release vengono pubblicate solo
dopo il completamento di queste prove.
La build non è firmata digitalmente; Windows può mostrare un avviso SmartScreen.

## Compilare e avviare su Windows

Richiesti Windows 10 2004 (build 19041) o Windows 11 **x64**, PowerShell 7,
SDK .NET **8.0.426** (fissato in `global.json`), Rust 1.99.1 MSVC e Visual Studio 2022 Build Tools con il workload
**Desktop development with C++**, strumenti MSVC x64, Windows SDK e strumenti
di compilazione **UWP/WinUI** (inclusi i task Appx/PRI di Visual Studio).
Lo script individua e usa MSBuild di Visual Studio; `dotnet publish` da solo
non dispone dei task PRI necessari a questa versione di WinUI.
Usare un terminale Developer PowerShell x64 se il linker non viene trovato.

```powershell
pwsh -File ./scripts/build.ps1 -Run
```

Lo script verifica i test Rust, ripristina le dipendenze NuGet con il lockfile,
genera la DLL x64 e pubblica l'app self-contained
in `artifacts/Bibbia/`. Si può avviare successivamente
`artifacts/Bibbia/Bibbia.Windows.exe`. Conservare **l'intera cartella**, non solo
l'eseguibile. Non occorre installare separatamente .NET o Windows App Runtime
sulla macchina destinataria. Il motore Rust usa il runtime C statico su Windows
per non richiedere il VC++ Redistributable.

Per creare il singolo installer serve anche **NSIS 3**. Su un PC di sviluppo con
tutti i prerequisiti, dalla radice del repository:

```powershell
pwsh -File ./scripts/package.ps1
```

Il risultato è `artifacts/installer/Bibbia-Setup-0.3.0-x64.exe`, con un file
`.sha256` accanto. Il flag `-SkipBuild` riutilizza una pubblicazione già completata.
La cartella `artifacts/Bibbia/` viene sempre rigenerata da una pubblicazione pulita,
per evitare di includere file rimasti da vecchie build.

```powershell
pwsh -File ./scripts/smoke-windows.ps1
```

Su un runner Windows pulito, per provare installazione, aggiornamento, interfaccia
reale, blocco con app aperta, disinstallazione e conservazione dei dati:

```powershell
pwsh -File ./scripts/test-installer.ps1 -Installer ./artifacts/installer/Bibbia-Setup-0.3.0-x64.exe
```

Il test rifiuta di avviarsi se trova dati o un'installazione Bibbia preesistenti.
La prova WinUI usa Windows UI Automation e richiede una sessione desktop interattiva.
Non eseguirla su un profilo con appunti reali.

Il workflow `.github/workflows/check.yml` compila il motore su Linux e l'intera
app su Windows. L'installer viene caricato solo dopo il completamento dei test
Windows. Per pubblicarlo, avviare `publish-release.yml` dalla scheda Actions e
indicare il numero di una build **Check and build** riuscita sul ramo `main`.

## Sviluppare il motore su Linux

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --all -- --check
cargo build --release --locked
python3 scripts/smoke-core.py
dotnet run --project tests/Bibbia.InteropSmoke -- target/release/libbible_core.so
```

Il toolchain è fissato in `rust-toolchain.toml`. L'interfaccia WinUI 3 si compila
e si esegue su Windows; Linux supporta lo sviluppo e i test del motore.
Usare il checkout esistente: ogni task cloud è già isolato, non serve un worktree.

Il test C# usa lo stesso collegamento nativo dell'app e controlla testo italiano,
UTF-8, note dopo la riapertura, importazione con nomi contenenti spazi, ricerca ed
errori gestiti. Chiude il database prima di rimuovere i dati temporanei, anche su Windows.

## Importare altre traduzioni

CEI 2008 e Martini cattoliche italiane sono già incluse e funzionano offline senza importazioni.
Nelle impostazioni puoi scegliere la lingua dell’interfaccia e filtrare le
traduzioni per lingua. Seleziona **Italiano → CEI 2008** oppure **Martini 1781 (cattolica)** per la
Bibbia cattolica completa.
Le installazioni esistenti mantengono la traduzione scelta nell'ultima lettura;
per cambiarla usare il selettore sotto il capitolo oppure **Aspetto e traduzioni → Traduzione**.
Per gli apocrifi e gli altri canoni scegliere **Raccolta estesa (83 sezioni)**: il testo è in inglese.

In **Aspetto e traduzioni → Importa traduzione JSON**, scegliere un testo di cui
si possiedono i diritti e assegnargli un nome univoco. Formato UTF-8, massimo 32 MB:

```json
[
  {
    "abbrev": "gen",
    "name": "Genesi",
    "chapters": [
      ["Testo del versetto 1", "Testo del versetto 2"],
      ["Primo versetto del capitolo 2"]
    ]
  }
]
```

Ogni libro contiene capitoli, ogni capitolo contiene versetti. Numeri e ordine
sono ricavati dagli array. Un campo facoltativo `labels`, con gli stessi array di
capitoli e un’etichetta per versetto, conserva numerazioni come `4a` o `15-16`;
non deve contenere etichette vuote o duplicate nello stesso capitolo. Sono ammessi anche corpus parziali. Le traduzioni
moderne disponibili su Bible.com non sono automaticamente redistribuibili;
nessuno scraping o download da quel servizio viene effettuato dall'app.

## Dati e backup

Percorso: `%LOCALAPPDATA%\BibbiaRust\bible.sqlite3`.
L'app non trasmette dati. Per il backup chiudere l'app, poi copiare la cartella
`BibbiaRust` intera (compresi eventuali file `-wal` e `-shm`). Per ripristinare,
chiudere l'app e ripristinare la cartella. I file non sono cifrati; usare le
protezioni dell'account Windows per dati sensibili.

## Controlli manuali aggiuntivi su Windows

1. Avviare l'app; controllare che la Martini cattolica sia selezionata al primo avvio.
   Verificare la presenza di Tobia, Giuditta, Sapienza, Siracide, Baruc e Maccabei.
   Aprire Genesi 1, poi Giovanni 3 con libro e capitolo.
2. Cercare `talmente amato il mondo`; aprire Giovanni 3:16 dai risultati.
   Cercare poi una sola lettera: deve apparire un errore senza risultati precedenti.
3. Selezionare parole con il mouse: deve apparire un menu compatto.
   Scegliere un colore, sottolineare solo la frase selezionata, salvare una nota
   con accenti e un segnalibro. Controllare la raccolta e il pulsante Copia.
4. Chiudere e riaprire: controllare note, ultimo versetto selezionato, tema e dimensione testo.
5. Rimuovere la formattazione dalla selezione, il segnalibro e la nota:
   quando non restano annotazioni, il versetto deve sparire dalla raccolta.
   Cambiare lingua dell’app senza cambiare la traduzione e provare i filtri italiano/inglese.
   Controllare che anche la barra superiore segua i temi chiaro/scuro e Windows.
6. Importare un JSON italiano autorizzato con spazi all'inizio e alla fine del nome;
   controllare selezione automatica della traduzione, lettura, ricerca e
   isolamento delle annotazioni tra traduzioni. Un file malformato deve mostrare
   un errore e lasciare intatti i dati.
7. Controllare tema Windows, contrasto elevato, tastiera, ridimensionamento e DPI
   al 100/150/200%. Verificare su un PC Windows senza SDK con la cartella pubblicata.

La pipeline verifica già anche selezione con mouse, note, lingue e tema nell’interfaccia reale.
Questi controlli aggiuntivi estendono la verifica ad accessibilità, DPI
e PC Windows 10/11 diversi dal runner Windows Server 2022.
Vedere [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) per fonti e licenze.

## Prestazioni e fonti dei testi

Il motore genera indici UTF-8 e testo per la ricerca durante la compilazione.
I corpus inclusi restano in sezioni di sola lettura della DLL; viene letto solo
il catalogo della traduzione utilizzata. Un capitolo carica tutte le annotazioni
con una query SQLite, le impostazioni identiche non vengono riscritte e viene
mantenuta al massimo una traduzione importata in memoria. Il catalogo delle
importazioni precedenti viene migrato una volta, senza modificare testi o appunti.
Se un vecchio import usa il nome di una nuova traduzione inclusa, riceve il suffisso
«importata»: annotazioni e posizione seguono il testo originale in una transazione.
Il ridimensionamento del testo aggiorna le righe esistenti con un limite di
frequenza, senza ricaricare il capitolo.

Microbenchmark riproducibile del **solo motore nativo su Linux**, build release,
mediana di 7 processi nuovi, file già nella cache del sistema, stessa macchina:

| Misura | v0.2.0 | v0.3.0 |
| --- | ---: | ---: |
| Inizializzazione motore | 11,742 ms | 0,764 ms |
| Genesi 1, media di 200 letture | 0,219 ms | 0,112 ms |
| Ricerca senza risultati, media di 40 scansioni | 10,121 ms | 0,676 ms |
| RSS del processo Python + DLL dopo queste operazioni | 43,637 MiB | 19,469 MiB |

Fixture: Martini 1781, ricerca `zzxyzznonexistent`. Le ricerche risultano circa
15 volte più rapide e il processo di misura usa circa il 55% di memoria residente
in meno, pur includendo due raccolte aggiuntive nella nuova DLL. Questi numeri
**non misurano la RAM o l’avvio dell’intera app WinUI su Windows** e dipendono
dalla macchina e dalla ricerca. Risultati grezzi in `docs/performance/`; comando:

```sh
python3 scripts/benchmark-core.py
python3 scripts/verify-corpora.py
```

Per riprodurre i nuovi corpus, installare `beautifulsoup4 lxml` solo nell’ambiente
di sviluppo e usare `scripts/import-cei.py --cache CARTELLA` e
`scripts/import-extended.py WEB_VPL_ZIP WEB_USFM_ZIP --enoch ENOCH_HTML --jubilees-cache CARTELLA`.
Gli URL e gli hash di ogni input sono nei manifest `*-source.json`; il convertitore
CEI scarica pagine mancanti solo con `--download`. L’app installata non scarica
questi testi: sono già inclusi. I versetti con lettere, quelli combinati e le
lacune della fonte mantengono le etichette originali. In Ester CEI, **G** indica
il testo greco, **E** quello ebraico, entrambi presenti nell’edizione ufficiale.

Il codice è MIT; **i testi hanno licenze separate**. La CEI è riservata agli usi
formativi e didattici non commerciali previsti dalla fonte, con attribuzione
visibile nel lettore. Licenze, condizioni e manifest sono inclusi nell’installer.
Vedere [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
