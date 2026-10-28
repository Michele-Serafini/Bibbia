# Bibbia per macOS

Versione 0.9.0, interamente Swift 6 con SwiftUI e AppKit, senza Rust o framework
di terze parti. macOS 13 Ventura o successivi, Apple Silicon e Intel.

## Installazione

La [release unificata 0.9.0](https://github.com/Michele-Serafini/Bibbia/releases/tag/v0.9.0)
contiene il [DMG Universal](https://github.com/Michele-Serafini/Bibbia/releases/download/v0.9.0/Bibbia-0.9.0-universal.dmg)
e lo [ZIP della stessa app](https://github.com/Michele-Serafini/Bibbia/releases/download/v0.9.0/Bibbia-0.9.0-universal.zip),
ciascuno con checksum SHA-256. Aprire il DMG e trascinare Bibbia in **Applicazioni**.
Non occorrono runtime, account, librerie aggiuntive o Internet.

La firma è **ad-hoc**, senza certificato Apple Developer ID e senza notarizzazione:
macOS può bloccare il primo avvio di un download. Dopo aver verificato fonte e
checksum, usare **Impostazioni di Sistema → Privacy e sicurezza → Apri comunque**.
Non è necessario disattivare Gatekeeper. La build automatica non dichiara una
firma Apple che non è stata fornita.

Per aggiornare, chiudere l’app e sostituirla in Applicazioni; per rimuoverla,
trascinarla nel Cestino. Appunti e testi importati restano in
`~/Library/Application Support/Bibbia/bible.sqlite3`.

## Uso

Barra laterale per libri, ricerca e appunti; traduzione sotto la barra laterale,
capitolo nella barra degli strumenti, frecce per passare al capitolo adiacente.
La selezione di parole o frasi apre un piccolo popover con quattro evidenziatori,
sottolineatura, segnalibro, nota e copia. Gli appunti sono paginati a 200 elementi.

**⌘F** cerca, **⌘,** apre le impostazioni, **⌥⌘←/→** cambia capitolo,
**⇧⌘I** importa una traduzione JSON. Italiano e inglese per l’interfaccia,
indipendentemente dalla lingua del testo. Tema automatico, chiaro o scuro.
La lettura riproduce una pagina di carta: al cambio di capitolo il foglio
si solleva dall’angolo superiore e si arrotola lungo una piega diagonale
che raggiunge progressivamente il bordo inferiore, in 1.100 ms. La modalità Libro impagina anche i capitoli lunghi. Titoli e pulsanti hanno transizioni brevi native. Tutte queste
animazioni rispettano Riduci movimento di macOS.

Stessi testi verificati della versione 0.4.0: Martini, CEI 2008, Diodati, KJV
e raccolta estesa di 83 sezioni, compresi Enoc e Giubilei. CEI 2008 è inclusa
per uso formativo e didattico non commerciale alle condizioni ufficiali:
attribuzione visibile durante la lettura e condizioni nel menu
**Lettura → Testi e licenze**. La raccolta estesa non è tutto il canone etiopico.

## Memoria e CPU

- Corpus preparati durante la build: testo UTF-8, indici di quattro byte e
  cataloghi piccoli. Nessuna analisi JSON dell’intera Bibbia all’avvio.
- File in sola lettura mappati da Foundation; una sola traduzione mappata alla
  volta. Indice minuscolo aperto soltanto alla prima ricerca di quella traduzione.
- Ricerca sui byte con `memmem`, senza creare una stringa per ogni versetto.
  Risultati limitati a 200 e cancellazione cooperativa delle ricerche superate.
- Un `actor` Swift serializza SQLite e ricerca fuori dal thread grafico.
  Una query carica tutte le annotazioni del capitolo. Statement SQLite riusati,
  WAL, cache di 1 MiB e scrittura delle sole preferenze cambiate.
- Una sola `NSTextView` per capitolo: TextKit gestisce selezione, layout,
  scorrimento, copia e accessibilità. I cambi di font, tema e segni modificano
  gli attributi conservando selezione e posizione di scorrimento.
- Nessun polling, rete o timer ricorrente nell’uso normale. Le modifiche rapide
  di font, preferenze, posizione e query sono accorpate con task cancellabili.
- Lo sfogliamento usa Core Animation, senza timer a ogni frame sul thread
  principale. Una sola immagine temporanea, limitata a due milioni di pixel,
  viene condivisa dalle 64 parti diagonali del foglio e liberata dopo la transizione.
- Font, controlli e SQLite di macOS, senza runtime distribuiti o font incorporati.
  Build release con ottimizzazione `-Osize` e binario Universal senza simboli debug.

Il workflow misura l’intera app su Mac, con tre profili nuovi, tempo fino al primo
layout/display, RSS e CPU per tre secondi dopo 1,5 secondi di assestamento,
poi di nuovo dopo altri cinque secondi senza input. L’hook di disponibilità attende
anche 100 ms prima del layout/display. Non sono avvii a cache del sistema fredda.
I campioni dei due runner non sono un confronto tra architetture; RSS include
librerie e pagine condivise, escludendo il processo WindowServer.

Misure precedenti della release 0.5.0 sui runner macOS 15.7.9, mediane di tre campioni:

| Runner | Primo layout/display | RAM RSS a regime | CPU a regime |
| --- | ---: | ---: | ---: |
| Apple Silicon | 1.448 s | 105.22 MiB | 0.00% |
| Intel | 1.101 s | 77.98 MiB | 0.00% |

I campioni della CPU a regime vanno da 0.00% a 0.32%; la risoluzione di `ps`
e la finestra di tre secondi non dimostrano assenza assoluta di attività.
Nei primi secondi la CPU mediana è circa 1% per il lavoro iniziale di macOS.
L’analisi `sample` mostra il thread principale in attesa degli eventi e
attività sporadiche di AppKit per conservare lo stato della finestra.
I [dati Apple Silicon](../docs/performance/v0.5.0-macos-arm64-ci.json) e
[Intel](../docs/performance/v0.5.0-macos-intel-ci.json) conservano tutti i campioni.
Sono misure su macchine CI virtuali, senza garanzie sugli stessi valori su altri Mac.

DMG di circa **20.5 MB**, ZIP di circa **15.9 MB**; app installata circa
**47.2 MiB**, comprendendo tutte e cinque le traduzioni e i relativi indici.
L’eseguibile Universal è circa 1.2 MB. Non vengono distribuiti runtime aggiuntivi.

## Compilare e verificare

Xcode con Swift 6 e Python 3.11+ per preparare i testi e i pacchetti:

```sh
python3 scripts/prepare-macos.py
swift test --package-path macos -c release
swift run --package-path macos Bibbia
python3 scripts/package-macos.py
python3 scripts/test-macos.py --dmg artifacts/macos/Bibbia-0.9.0-universal.dmg
```

`swift run` serve allo sviluppo; per provare icona, menu, firma locale e risorse
nel percorso definitivo usare la `.app` preparata dallo script.
`Bibbia.app/Contents/MacOS/Bibbia --data-dir /percorso/assoluto` cambia cartella dati.

I test del motore verificano corpora, CEI, apocrifi, Unicode, sovrapposizioni,
transazioni, paginazione, importazioni e compatibilità dei backup delle altre
piattaforme. La prova nativa apre realmente la `.app` ricollocata, seleziona testo,
mostra i popover, applica annotazioni, cerca CEI, naviga, cambia font e tema,
cattura schermate e riapre SQLite. Si usa una cartella temporanea con marcatore
`.bibbia-test-fixture`; `--ui-smoke` rifiuta profili non marcati.
La medesima app Universal è provata su Apple Silicon e Intel. Si verificano
firma ad-hoc, entrambe le architetture Mach-O, dipendenze esclusivamente di
sistema, checksum e integrità del DMG; su Intel si monta il DMG in sola lettura
e si prova la copia in una cartella con spazi nel nome.

**Verifica 0.5.0:** [Build macOS completata](https://github.com/Michele-Serafini/Bibbia/actions/runs/40229905984),
codice `0248dac`: 20 test Swift e prove native sulle due architetture, incluse
selezione con eventi mouse AppKit, popover, note, evidenziatori, segnalibri,
CEI, ricerca, dimensione reale dei font e riapertura in un secondo processo.
Sono passati anche i [controlli Windows, Rust, Debian e Fedora](https://github.com/Michele-Serafini/Bibbia/actions/runs/40229906006).
Il workflow **Publish verified packages** pubblica EXE Windows, DEB/RPM Linux
e DMG/ZIP macOS nella stessa release. Richiede build native riuscite della
stessa revisione, controlla le versioni e ricontrolla tutti i checksum.
`VERSION` definisce la versione comune; `scripts/check-versions.py` verifica
i manifest delle tre piattaforme prima di compilare e creare i pacchetti.

Lo schema SQLite e gli offset UTF-16 sono compatibili con Windows/Linux.
Per trasferire i dati chiudere tutte le istanze e copiare la cartella del database,
compresi i file WAL/SHM se presenti. Non si effettua sincronizzazione automatica.
