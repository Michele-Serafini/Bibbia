# Bibbia 0.5.0 per macOS

Nuova applicazione interamente Swift, con motore Swift e interfaccia SwiftUI/AppKit.
Richiede macOS 13 Ventura o successivi, su Apple Silicon e Intel.

Aprire **Bibbia-0.5.0-universal.dmg** e trascinare Bibbia in Applicazioni.
È disponibile anche lo ZIP della stessa app. Nessun runtime o download aggiuntivo.

- Barra laterale nativa, strumenti macOS, font Apple di sistema e lettura serif.
- Tema automatico/chiaro/scuro, testo ridimensionabile e popover compatti.
- CEI 2008 cattolica italiana (73 libri), Martini, Diodati, KJV e raccolta estesa
  di 83 sezioni in inglese con Enoc e Giubilei.
- Ricerca cancellabile, evidenziatori, sottolineature per frase, note, segnalibri,
  ripresa della lettura e importazione JSON.
- Corpus mappati, una sola traduzione in memoria, un actor per ricerca/SQLite,
  una sola NSTextView per capitolo, nessun polling quando inattiva.
- Database locale compatibile con lo schema Windows/Linux; aggiornare o rimuovere
  l’app conserva gli appunti.

CEI: uso formativo e didattico non commerciale secondo le condizioni ufficiali,
incluse nella app. La raccolta estesa non comprende tutto il canone etiopico.

**Firma:** ad-hoc, senza certificato Apple Developer ID e senza notarizzazione.
macOS può bloccare il primo avvio. Dopo aver controllato provenienza e checksum,
usare **Impostazioni di Sistema → Privacy e sicurezza → Apri comunque**.

Le versioni Windows e Linux 0.4.0 restano disponibili nella rispettiva release.

Verificata la stessa app Universal su Apple Silicon e Intel, su macOS 15.7.9:
20 test Swift, selezione con eventi mouse AppKit, popover, salvataggio e riapertura
in un secondo processo, CEI, ricerca, font e temi. Verificati firma locale,
risorse dopo ricollocazione, dipendenze di sistema e montaggio del DMG su Intel.

[Pipeline macOS riuscita](https://github.com/Michele-Serafini/Bibbia/actions/runs/40229905984),
codice `0248dac`. [Controlli delle altre piattaforme riusciti](https://github.com/Michele-Serafini/Bibbia/actions/runs/40229906006).

Misure dell’intera app sulle macchine CI, tre profili nuovi per runner:
RAM RSS a regime mediana **78 MiB su Intel**, **105 MiB su Apple Silicon**;
CPU a regime mediana **0.00%**, campioni **0.00–0.32%** su finestre di tre secondi.
I runner sono macchine diverse: questi numeri non confrontano le architetture
e non promettono gli stessi consumi su ogni Mac. WindowServer è escluso.
Il DMG è circa **20.5 MB**; l’app installata circa **47.2 MiB** con tutti i testi.
