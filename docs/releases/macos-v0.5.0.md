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
