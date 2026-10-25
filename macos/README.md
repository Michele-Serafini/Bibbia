# Bibbia per macOS

Versione 0.5.0, interamente Swift 6 con SwiftUI e AppKit, senza Rust o framework
di terze parti. macOS 13 Ventura o successivi, Apple Silicon e Intel.

## Installazione

Il workflow **Build macOS** produce `Bibbia-0.5.0-universal.dmg` e una copia ZIP
della stessa `.app`. Aprire il DMG e trascinare Bibbia in **Applicazioni**.
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
Le animazioni dei popover rispettano Riduci movimento di macOS.

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
- Font, controlli e SQLite di macOS, senza runtime distribuiti o font incorporati.
  Build release con ottimizzazione `-Osize` e binario Universal senza simboli debug.

Il workflow misura l’intera app su Mac, con tre profili nuovi, tempo fino al primo
layout/display, RSS e CPU per tre secondi a riposo. I risultati sono negli artefatti
diagnostici; non sono promesse valide per ogni computer e non comprendono WindowServer.

## Compilare e verificare

Xcode con Swift 6 e Python 3.11+ per preparare i testi e i pacchetti:

```sh
python3 scripts/prepare-macos.py
swift test --package-path macos -c release
swift run --package-path macos Bibbia
python3 scripts/package-macos.py
python3 scripts/test-macos.py --dmg artifacts/macos/Bibbia-0.5.0-universal.dmg
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
Il medesimo DMG è provato su Apple Silicon e Intel prima della pubblicazione.

Lo schema SQLite e gli offset UTF-16 sono compatibili con Windows/Linux.
Per trasferire i dati chiudere tutte le istanze e copiare la cartella del database,
compresi i file WAL/SHM se presenti. Non si effettua sincronizzazione automatica.
