# Testi e dipendenze

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
