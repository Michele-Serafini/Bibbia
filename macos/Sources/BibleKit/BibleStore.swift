import Foundation

/// All disk work and search run on the actor executor, away from the UI thread.
/// A single edition is mapped/cached. There are no timers or background polling.
public actor BibleStore {
    let database: Database
    let resources: URL
    private var catalog: [Edition]
    private var cached: (String, Corpus)?
    var comparisonIndex: ComparisonIndex?

    private static var resourceBundle: Bundle {
        // SwiftPM's generated Bundle.module searches next to a command-line
        // executable. Finder apps keep resources inside Contents/Resources.
        if let url = Bundle.main.resourceURL?.appendingPathComponent("BibbiaMac_BibleKit.bundle"),
           let bundle = Bundle(url: url) { return bundle }
        return Bundle.module
    }
    public static var bundledResources: URL {
        resourceBundle.resourceURL!.appendingPathComponent("Corpora", isDirectory: true)
    }
    public static var notices: URL {
        resourceBundle.resourceURL!.appendingPathComponent("Notices", isDirectory: true)
    }

    public init(directory: URL, resources: URL = BibleStore.bundledResources) throws {
        let database = try Database(directory: directory)
        self.database = database
        self.resources = resources
        catalog = try JSONDecoder().decode([Edition].self,
            from: Data(contentsOf: resources.appendingPathComponent("editions.json")))
        let imports = try database.rows("SELECT id,book_count FROM editions ORDER BY id")
        for row in imports {
            guard let original = row[0].string, let count = row[1].integer else { continue }
            var id = original
            if catalog.contains(where: { $0.id == id }) {
                var suffix = 1
                repeat {
                    id = suffix == 1 ? "\(original) (importata)" : "\(original) (importata \(suffix))"
                    suffix += 1
                } while !(try database.rows("SELECT 1 FROM editions WHERE id=? UNION ALL SELECT 1 FROM annotations WHERE edition=? LIMIT 1", [.text(id), .text(id)])).isEmpty
                try database.transaction {
                    _ = try database.rows("UPDATE editions SET id=? WHERE id=?", [.text(id), .text(original)])
                    _ = try database.rows("UPDATE annotations SET edition=? WHERE edition=?", [.text(id), .text(original)])
                    _ = try database.rows("UPDATE note_organization SET edition=? WHERE edition=?", [.text(id), .text(original)])
                    _ = try database.rows("UPDATE settings SET payload=json_set(payload,'$.edition',?) WHERE key='position' AND json_extract(payload,'$.edition')=?", [.text(id), .text(original)])
                }
            }
            catalog.append(Edition(id: id, file: "", language: "und", canon: "custom", books: count))
        }
    }

    public func editions() -> [Edition] { catalog }

    /// Release inactive corpus mappings without loading another translation.
    public func releaseInactiveCorpus(keeping edition: String) {
        if cached?.0 != edition { cached = nil }
    }

    func corpus(_ id: String) throws -> Corpus {
        if let cached, cached.0 == id { return cached.1 }
        guard let edition = catalog.first(where: { $0.id == id }) else { throw BibleError.invalid("Traduzione non disponibile / Edition unavailable") }
        // Release the old mapping/import before loading a different edition.
        cached = nil
        let result: Corpus
        if edition.file.isEmpty {
            guard let json = try database.rows("SELECT payload FROM editions WHERE id=?", [.text(id)]).first?.first?.string else { throw BibleError.invalid("Traduzione mancante") }
            result = Corpus(imported: try JSONDecoder().decode([ImportedBook].self, from: Data(json.utf8)))
        } else { result = try Corpus(root: resources, file: edition.file) }
        cached = (id, result)
        return result
    }

    public func books(edition: String) throws -> [BookIndex] { try corpus(edition).books }

    public func chapter(_ position: Position) throws -> [Verse] {
        let source = try corpus(position.edition)
        let span = try source.chapter(position.book, position.chapter)
        let rows = try database.rows("SELECT verse,payload FROM annotations WHERE edition=? AND book=? AND chapter=?",
                                    [.text(position.edition), .integer(position.book), .integer(position.chapter)])
        var annotations: [Int: Annotation] = [:]
        for row in rows {
            if let verse = row[0].integer, let json = row[1].string {
                annotations[verse] = try JSONDecoder().decode(Annotation.self, from: Data(json.utf8))
            }
        }
        return try (0..<span.count).map { vi in
            Verse(id: vi, label: span.label(vi), text: try source.verse(position.book, position.chapter, vi),
                  annotation: annotations[vi] ?? Annotation())
        }
    }

    public func bootstrap() throws -> Snapshot {
        var preferences = Preferences()
        preferences.language = try database.setting("language", as: String.self) ?? "it"
        preferences.theme = try database.setting("theme", as: String.self) ?? "Default"
        preferences.fontSize = try database.setting("fontSize", as: Int.self) ?? 22
        preferences.bibleLanguage = try database.setting("bibleLanguage", as: String.self) ?? "all"
        preferences.comparisonEdition = try database.setting("comparisonEdition", as: String.self) ?? "CEI 2008"
        if !["it", "en"].contains(preferences.language) { preferences.language = "it" }
        if !["Default", "Light", "Dark"].contains(preferences.theme) { preferences.theme = "Default" }
        preferences.fontSize = min(32, max(14, preferences.fontSize))
        var position = try database.setting("position", as: Position.self) ?? Position()
        if (try? corpus(position.edition).verse(position.book, position.chapter, position.verse)) == nil { position = Position() }
        return Snapshot(editions: catalog, position: position, preferences: preferences,
                        books: try books(edition: position.edition), verses: try chapter(position))
    }

    public func savePosition(_ position: Position) throws {
        _ = try corpus(position.edition).verse(position.book, position.chapter, position.verse)
        try database.set("position", position)
    }

    public func savePreferences(_ preferences: Preferences) throws {
        guard ["it", "en"].contains(preferences.language), (14...32).contains(preferences.fontSize),
              ["Default", "Light", "Dark"].contains(preferences.theme),
              ["all", "it", "en"].contains(preferences.bibleLanguage) else {
            throw BibleError.invalid("Preferenze non valide / Invalid preferences")
        }
        try database.transaction {
            try database.set("language", preferences.language)
            try database.set("theme", preferences.theme)
            try database.set("fontSize", preferences.fontSize)
            try database.set("bibleLanguage", preferences.bibleLanguage)
            try database.set("comparisonEdition", preferences.comparisonEdition)
        }
    }

    private func annotation(_ position: Position) throws -> Annotation {
        let rows = try database.rows("SELECT payload FROM annotations WHERE edition=? AND book=? AND chapter=? AND verse=?",
            [.text(position.edition), .integer(position.book), .integer(position.chapter), .integer(position.verse)])
        guard let json = rows.first?.first?.string else { return Annotation() }
        return try JSONDecoder().decode(Annotation.self, from: Data(json.utf8))
    }

    private func save(_ annotation: Annotation, at position: Position) throws {
        let keys: [SQLValue] = [.text(position.edition), .integer(position.book), .integer(position.chapter), .integer(position.verse)]
        if annotation.isEmpty {
            _ = try database.rows("DELETE FROM note_organization WHERE edition=? AND book=? AND chapter=? AND verse=?", keys)
            _ = try database.rows("DELETE FROM annotations WHERE edition=? AND book=? AND chapter=? AND verse=?", keys)
        } else {
            let json = String(decoding: try JSONEncoder().encode(annotation), as: UTF8.self)
            _ = try database.rows("INSERT OR REPLACE INTO annotations VALUES (?,?,?,?,?)", keys + [.text(json)])
        }
    }

    /// Multi-verse selections are committed together, or rolled back together.
    public func mark(_ position: Position, selection: [SelectedSpan], action: String) throws -> [Verse] {
        let source = try corpus(position.edition)
        guard !selection.isEmpty else { throw BibleError.invalid("Seleziona il testo / Select text") }
        try database.transaction {
            for span in selection {
                var target = position; target.verse = span.verse
                let text = try source.verse(target.book, target.chapter, target.verse)
                let result = try Marking.apply(text: text, annotation: annotation(target),
                                              start: span.start, length: span.length, action: action)
                try save(result, at: target)
            }
        }
        return try chapter(position)
    }

    public func setNote(_ note: String, at position: Position) throws -> [Verse] {
        guard note.utf8.count <= 32768 else { throw BibleError.invalid("Nota troppo lunga / Note too long") }
        _ = try corpus(position.edition).verse(position.book, position.chapter, position.verse)
        var value = try annotation(position); value.note = note
        try save(value, at: position)
        return try chapter(position)
    }

    public func toggleBookmark(at position: Position) throws -> [Verse] {
        _ = try corpus(position.edition).verse(position.book, position.chapter, position.verse)
        var value = try annotation(position); value.bookmark.toggle()
        try save(value, at: position)
        return try chapter(position)
    }

    public func search(edition: String, query: String) throws -> SearchResult {
        let query = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard query.unicodeScalars.count >= 2, query.utf8.count <= 256 else {
            throw BibleError.invalid("Inserisci da 2 a 256 caratteri / Enter 2–256 characters")
        }
        try Task.checkCancellation()
        return try corpus(edition).search(edition: edition, query: query)
    }

    public func importEdition(file: URL, name: String) throws -> [Edition] {
        let name = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty, name.utf8.count <= 80, !catalog.contains(where: { $0.id == name }) else {
            throw BibleError.invalid("Scegli un nome nuovo (1–80 byte) / Choose a new name (1–80 bytes)")
        }
        let size = try file.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? 0
        guard size <= 32 * 1024 * 1024 else { throw BibleError.invalid("File oltre 32 MB / File exceeds 32 MB") }
        var raw = try Data(contentsOf: file, options: .mappedIfSafe)
        if raw.starts(with: [0xef, 0xbb, 0xbf]) { raw.removeFirst(3) }
        let books = try JSONDecoder().decode([ImportedBook].self, from: raw)
        guard !books.isEmpty, books.count <= 100 else { throw BibleError.invalid("Struttura Bibbia non valida / Invalid Bible structure") }
        for book in books {
            guard !book.name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
                  !book.abbrev.isEmpty, !book.chapters.isEmpty, book.chapters.count <= 200,
                  book.labels == nil || book.labels?.count == book.chapters.count else { throw BibleError.invalid("Libro non valido / Invalid book") }
            for (ci, chapter) in book.chapters.enumerated() {
                guard !chapter.isEmpty, chapter.count <= 200,
                      chapter.allSatisfy({ !$0.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }) else { throw BibleError.invalid("Capitolo non valido / Invalid chapter") }
                if let labels = book.labels?[ci] {
                    guard labels.count == chapter.count, Set(labels).count == labels.count,
                          labels.allSatisfy({ !$0.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty && $0.utf8.count <= 24 }) else { throw BibleError.invalid("Etichette non valide / Invalid labels") }
                }
            }
        }
        let payload = String(decoding: try JSONEncoder().encode(books), as: UTF8.self)
        _ = try database.rows("INSERT INTO editions (id,payload,book_count) VALUES (?,?,?)",
                             [.text(name), .text(payload), .integer(books.count)])
        cached = nil
        catalog.append(Edition(id: name, file: "", language: "und", canon: "custom", books: books.count))
        return catalog
    }
}
