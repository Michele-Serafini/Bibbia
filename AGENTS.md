# Istruzioni del progetto

## Modifiche per tutte le piattaforme

Per esplicita richiesta dell’utente, le prossime funzionalità, correzioni,
migliorie grafiche e ottimizzazioni devono essere applicate a **Windows, Linux
e macOS**, salvo diversa indicazione esplicita dell’utente.

- Windows: frontend WinUI 3 in `app/`, motore Rust in `core/`.
- Linux: frontend Rust GTK4/libadwaita in `gnome/`, motore Rust in `core/`;
  mantenere i pacchetti DEB e RPM.
- macOS: applicazione e motore Swift in `macos/`, interfaccia SwiftUI/AppKit;
  mantenere il pacchetto Universal per Apple Silicon e Intel.

Mantenere le stesse funzioni e i dati compatibili sulle tre piattaforme,
rispettando toolkit, font, temi, accessibilità e convenzioni native di ciascun
sistema. Una modifica al motore Rust non aggiorna il motore Swift: verificare
e aggiornare anche quest’ultimo quando pertinente. Conservare compatibilità
delle annotazioni SQLite, offset UTF-16 e condizioni dei testi biblici.

Verificare le modifiche con i controlli appropriati per tutte le piattaforme
interessate, usando le pipeline native disponibili; riportare con precisione
eventuali verifiche non eseguite o limiti specifici di una piattaforma.
