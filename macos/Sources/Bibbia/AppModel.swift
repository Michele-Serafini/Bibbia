import SwiftUI
import AppKit
import UniformTypeIdentifiers
import BibleKit

@MainActor
final class AppModel: ObservableObject {
    enum Panel: String, CaseIterable { case books, search, saved }
    @Published var editions: [Edition] = []
    @Published var books: [BookIndex] = []
    @Published var verses: [Verse] = []
    @Published var position = Position()
    @Published var preferences = Preferences()
    @Published var panel = Panel.books
    @Published var query = ""
    @Published var hits: [Hit] = []
    @Published var saved: [Hit] = []
    @Published var savedPage = 0
    @Published var truncated = false
    @Published var loading = true
    @Published var searching = false
    @Published var error: String?
    @Published var revision = 0
    @Published var scrollTarget: Int?
    @Published var showNotices = false
    private(set) var store: BibleStore?
    private var generation = 0
    private var searchTask: Task<Void, Never>?
    private var preferencesTask: Task<Void, Never>?
    private var positionTask: Task<Void, Never>?
    let directory: URL

    init() {
        let arguments = ProcessInfo.processInfo.arguments
        if let index = arguments.firstIndex(of: "--data-dir"), arguments.indices.contains(index + 1) {
            directory = URL(fileURLWithPath: arguments[index + 1], isDirectory: true)
        } else {
            directory = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
                .appendingPathComponent("Bibbia", isDirectory: true)
        }
    }

    func t(_ it: String, _ en: String) -> String { preferences.language == "en" ? en : it }
    var title: String {
        books.indices.contains(position.book) ? "\(books[position.book].name) \(position.chapter + 1)" : "Bibbia"
    }
    var chapterCount: Int { books.indices.contains(position.book) ? books[position.book].chapters.count : 1 }
    var visibleEditions: [Edition] {
        editions.filter { preferences.bibleLanguage == "all" || $0.language == preferences.bibleLanguage || $0.id == position.edition }
    }

    func start() async {
        guard store == nil else { return }
        do {
            let directory = directory
            let engine = try await Task.detached(priority: .userInitiated) { try BibleStore(directory: directory) }.value
            store = engine
            let snapshot = try await engine.bootstrap()
            editions = snapshot.editions; books = snapshot.books; verses = snapshot.verses
            position = snapshot.position; preferences = snapshot.preferences
            scrollTarget = position.verse
            applyTheme(); revision += 1; loading = false
            if ProcessInfo.processInfo.arguments.contains("--ui-smoke") { await runSmoke() }
            if ProcessInfo.processInfo.arguments.contains("--ui-reopen") { await runReopenSmoke() }
            if let marker = ProcessInfo.processInfo.environment["BIBBIA_BENCHMARK_READY"] {
                // A one-shot test hook; absent during normal launches.
                try await Task.sleep(for: .milliseconds(100))
                for window in NSApp.windows { window.contentView?.layoutSubtreeIfNeeded(); window.displayIfNeeded() }
                try Data("ready\n".utf8).write(to: URL(fileURLWithPath: marker))
            }
        } catch { self.error = error.localizedDescription; loading = false }
    }

    func navigate(_ target: Position) {
        guard let store else { return }
        generation += 1
        let token = generation
        let editionChanged = target.edition != position.edition
        positionTask?.cancel()
        if editionChanged { searchTask?.cancel(); searching = false; hits = []; saved = [] }
        loading = true
        Task {
            do {
                let books: [BookIndex]
                if editionChanged { books = try await store.books(edition: target.edition) }
                else { books = self.books }
                let verses = try await store.chapter(target)
                guard token == generation else { return }
                try await store.savePosition(target)
                guard token == generation else { return }
                self.position = target; self.books = books; self.verses = verses
                scrollTarget = target.verse; revision += 1; loading = false
                if editionChanged && panel == .saved { loadSaved(reset: true) }
                if editionChanged && panel == .search { search() }
            } catch {
                guard token == generation else { return }
                self.error = error.localizedDescription; loading = false
            }
        }
    }

    func step(_ delta: Int) {
        var target = position; target.verse = 0
        target.chapter += delta
        if target.chapter < 0 {
            guard target.book > 0 else { return }
            target.book -= 1; target.chapter = books[target.book].chapters.count - 1
        } else if target.chapter >= chapterCount {
            guard target.book + 1 < books.count else { return }
            target.book += 1; target.chapter = 0
        }
        navigate(target)
    }

    func selectedVerse(_ verse: Int) {
        guard verses.indices.contains(verse), position.verse != verse else { return }
        position.verse = verse
        positionTask?.cancel()
        let target = position
        positionTask = Task {
            do {
                try await Task.sleep(for: .milliseconds(180))
                try await store?.savePosition(target)
            } catch is CancellationError {} catch { self.error = error.localizedDescription }
        }
    }

    func search() {
        searchTask?.cancel()
        hits = []; truncated = false; searching = false
        guard let store, query.trimmingCharacters(in: .whitespacesAndNewlines).unicodeScalars.count >= 2 else { return }
        let query = query, edition = position.edition
        searching = true
        searchTask = Task {
            do {
                try await Task.sleep(for: .milliseconds(220))
                let result = try await store.search(edition: edition, query: query)
                try Task.checkCancellation()
                hits = result.hits; truncated = result.truncated; searching = false
            } catch is CancellationError {} catch { self.error = error.localizedDescription; searching = false }
        }
    }

    func loadSaved(reset: Bool = false) {
        if reset { savedPage = 0 }
        guard let store else { return }
        let edition = position.edition, page = savedPage
        Task {
            do {
                let result = try await store.saved(edition: edition, offset: page * 200)
                guard self.position.edition == edition, self.savedPage == page else { return }
                saved = result
            } catch { self.error = error.localizedDescription }
        }
    }

    func mark(_ selection: [SelectedSpan], action: String, at target: Position) {
        guard let store else { return }
        Task {
            do { apply(try await store.mark(target, selection: selection, action: action), at: target) }
            catch { self.error = error.localizedDescription }
        }
    }

    func bookmark(at target: Position) {
        guard let store else { return }
        Task {
            do { apply(try await store.toggleBookmark(at: target), at: target) }
            catch { self.error = error.localizedDescription }
        }
    }

    func note(_ text: String, at target: Position) {
        guard let store else { return }
        Task {
            do { apply(try await store.setNote(text, at: target), at: target) }
            catch { self.error = error.localizedDescription }
        }
    }

    private func apply(_ verses: [Verse], at target: Position) {
        if target.edition == position.edition, target.book == position.book, target.chapter == position.chapter {
            self.verses = verses; revision += 1
        }
        if panel == .saved { loadSaved() }
    }

    func preferencesChanged() {
        applyTheme(); preferencesTask?.cancel()
        let value = preferences
        preferencesTask = Task {
            do {
                try await Task.sleep(for: .milliseconds(180))
                try await store?.savePreferences(value)
            } catch is CancellationError {} catch { self.error = error.localizedDescription }
        }
    }

    private func applyTheme() {
        NSApp.appearance = preferences.theme == "Light" ? NSAppearance(named: .aqua)
            : preferences.theme == "Dark" ? NSAppearance(named: .darkAqua) : nil
    }

    func flush() async {
        positionTask?.cancel(); preferencesTask?.cancel(); searchTask?.cancel()
        do {
            try await store?.savePosition(position)
            try await store?.savePreferences(preferences)
        } catch { self.error = error.localizedDescription }
    }

    func importEdition() {
        let chooser = NSOpenPanel()
        chooser.allowedContentTypes = [.json]; chooser.allowsMultipleSelection = false
        chooser.message = t("Importa soltanto testi che hai il diritto di usare. Formato JSON, massimo 32 MB.",
                            "Only import texts you have the right to use. JSON format, maximum 32 MB.")
        chooser.begin { [weak self] response in
            guard let self, response == .OK, let url = chooser.url else { return }
            let dialog = NSAlert()
            dialog.messageText = self.t("Nome della traduzione", "Edition name")
            dialog.informativeText = self.t("Scegli un nome diverso dalle traduzioni già presenti.", "Choose a name different from existing editions.")
            let field = NSTextField(string: url.deletingPathExtension().lastPathComponent)
            field.frame = NSRect(x: 0, y: 0, width: 300, height: 24)
            dialog.accessoryView = field
            dialog.addButton(withTitle: self.t("Importa", "Import")); dialog.addButton(withTitle: self.t("Annulla", "Cancel"))
            guard dialog.runModal() == .alertFirstButtonReturn, let store = self.store else { return }
            let name = field.stringValue
            Task {
                do {
                    self.editions = try await store.importEdition(file: url, name: name)
                    self.navigate(Position(edition: name.trimmingCharacters(in: .whitespacesAndNewlines)))
                } catch { self.error = error.localizedDescription }
            }
        }
    }
}
