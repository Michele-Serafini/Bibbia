# Bibbia per Windows

Prima versione di un lettore biblico offline con interfaccia **WinUI 3 nativa**
e motore **Rust**. Interfaccia italiana, Mica, tema di Windows o chiaro/scuro,
caratteri ridimensionabili e navigazione adattiva. Nessun browser incorporato,
server o account. È un progetto indipendente, ispirato al flusso di lettura delle
app bibliche; non è un client ufficiale Bible.com/YouVersion.

## Funzioni implementate

- Bibbia Diodati 1649 completa in italiano, selezionata al primo avvio; KJV inglese inclusa.
- Navigazione per libro e capitolo; ripresa dell'ultimo versetto selezionato.
- Ricerca senza distinzione maiuscole/minuscole nella traduzione selezionata;
  massimo 200 risultati con indicazione del limite.
- Selezione di un versetto: quattro colori, sottolineatura, segnalibro, nota e copia.
- Raccolta degli appunti con ritorno al versetto originale.
- Importazione di altre traduzioni JSON, anche italiane, senza sovrascrivere
  traduzioni esistenti o mescolare le relative annotazioni.
- Dati personali locali in SQLite, con transazioni e modalità WAL.

Le evidenziazioni e sottolineature si applicano al versetto intero, non a singole
parole. Audio, piani di lettura, sincronizzazione cloud e catalogo online
YouVersion non sono implementati. Non sono stati misurati tempi di avvio o RAM su
Windows; il runtime .NET e WinUI self-contained aumentano la dimensione del download.

## Architettura

`core/` contiene corpus, ricerca, importazione, validazione e persistenza in Rust.
`app/` è un piccolo host C#/.NET 8 che presenta i controlli WinUI 3 e chiama il
motore tramite due funzioni C ABI (`bible_request`, `bible_free`). Non è quindi
un'applicazione interamente Rust: questo confine permette di usare i controlli
WinUI ufficiali con un motore Rust indipendente e testabile.

## Installare su Windows

Il pacchetto previsto è **`Bibbia-Setup-0.1.0-x64.exe`**: un solo installer offline,
con interfaccia italiana. Non richiede Rust, .NET, Visual Studio, account o
download aggiuntivi sul PC destinatario. Supporta Windows 10 versione 2004 o
successiva e Windows 11 a 64 bit.

1. Aprire l'installer e seguire la procedura. Non servono privilegi di amministratore.
2. Avviare **Bibbia** dal menu Start; il collegamento sul desktop è facoltativo.
3. Per aggiornare, chiudere Bibbia e aprire il nuovo installer. I dati personali restano al loro posto.
4. Per rimuovere l'app usare **Impostazioni Windows → App → Bibbia → Disinstalla**.
   Note, segnalibri e traduzioni importate vengono conservati.

Il workflow GitHub Actions **Check and build** genera l'installer nell'artefatto
**Bibbia-Installer-x64**, insieme al checksum SHA-256. Dopo il caricamento di
questi file nel repository, eseguire il workflow dalla scheda **Actions**,
scaricare l'artefatto ed estrarre il `.exe` dallo ZIP di GitHub.
Non viene pubblicata automaticamente una release.

**Stato:** nell'ambiente Linux è stato compilato lo script NSIS con un payload
di prova, eliminato al termine. L'installer della vera applicazione non è ancora
stato generato: richiede l'esecuzione della build e dei test su Windows.
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

Il risultato è `artifacts/installer/Bibbia-Setup-0.1.0-x64.exe`, con un file
`.sha256` accanto. Il flag `-SkipBuild` riutilizza una pubblicazione già completata.
La cartella `artifacts/Bibbia/` viene sempre rigenerata da una pubblicazione pulita,
per evitare di includere file rimasti da vecchie build.

```powershell
pwsh -File ./scripts/smoke-windows.ps1
```

Su un runner Windows pulito, per provare installazione, aggiornamento, interfaccia
reale, blocco con app aperta, disinstallazione e conservazione dei dati:

```powershell
pwsh -File ./scripts/test-installer.ps1 -Installer ./artifacts/installer/Bibbia-Setup-0.1.0-x64.exe
```

Il test rifiuta di avviarsi se trova dati o un'installazione Bibbia preesistenti.
La prova WinUI usa Windows UI Automation e richiede una sessione desktop interattiva.
Non eseguirla su un profilo con appunti reali.

Il workflow `.github/workflows/check.yml` compila il motore su Linux e l'intera
app su Windows e carica un artefatto scaricabile. Il workflow deve essere eseguito
su GitHub: la sua presenza non costituisce una build Windows riuscita.
L'installer viene caricato solo dopo il completamento dei test Windows.

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

La Diodati 1649 italiana è già inclusa e funziona offline senza importazioni.
Le installazioni esistenti mantengono la traduzione scelta nell'ultima lettura;
per cambiarla usare **Aspetto e traduzioni → Traduzione**.

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
sono ricavati dagli array. Sono ammessi anche corpus parziali. Le traduzioni
moderne disponibili su Bible.com non sono automaticamente redistribuibili;
nessuno scraping o download da quel servizio viene effettuato dall'app.

## Dati e backup

Percorso: `%LOCALAPPDATA%\BibbiaRust\bible.sqlite3`.
L'app non trasmette dati. Per il backup chiudere l'app, poi copiare la cartella
`BibbiaRust` intera (compresi eventuali file `-wal` e `-shm`). Per ripristinare,
chiudere l'app e ripristinare la cartella. I file non sono cifrati; usare le
protezioni dell'account Windows per dati sensibili.

## Verifica manuale su Windows

1. Avviare l'app; controllare che la Diodati italiana sia selezionata al primo avvio.
   Aprire Genesi 1, poi Giovanni 3 con libro e capitolo.
2. Cercare `Iddio ha tanto amato il mondo`; aprire Giovanni 3:16 dai risultati.
   Cercare poi una sola lettera: deve apparire un errore senza risultati precedenti.
3. Selezionare il versetto, scegliere un colore, sottolineare, salvare una nota
   con accenti e un segnalibro. Controllare la raccolta e il pulsante Copia.
4. Chiudere e riaprire: controllare note, ultimo versetto selezionato, tema e dimensione testo.
5. Rimuovere colore, spunte e nota: il versetto deve sparire dalla raccolta.
6. Importare un JSON italiano autorizzato con spazi all'inizio e alla fine del nome;
   controllare selezione automatica della traduzione, lettura, ricerca e
   isolamento delle annotazioni tra traduzioni. Un file malformato deve mostrare
   un errore e lasciare intatti i dati.
7. Controllare tema Windows, contrasto elevato, tastiera, ridimensionamento e DPI
   al 100/150/200%. Verificare su un PC Windows senza SDK con la cartella pubblicata.

Questa verifica grafica è necessaria prima di considerare pronta una release.
Vedere [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) per fonti e licenze.
