import SwiftUI
import BibleKit

struct MainView: View {
    @ObservedObject var model: AppModel
    @FocusState private var searchFocused: Bool
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        NavigationSplitView {
            sidebar.navigationSplitViewColumnWidth(min: 230, ideal: 270, max: 340)
        } detail: {
            reader
        }
        .navigationTitle(model.title)
        .toolbar {
            ToolbarItemGroup(placement: .navigation) {
                Button { model.step(-1) } label: { Image(systemName: "chevron.left") }
                    .buttonStyle(BookButtonStyle())
                    .help(model.t("Capitolo precedente (⌥⌘←)", "Previous chapter (⌥⌘←)"))
                    .accessibilityIdentifier("previousChapter")
                Button { model.step(1) } label: { Image(systemName: "chevron.right") }
                    .buttonStyle(BookButtonStyle())
                    .help(model.t("Capitolo successivo (⌥⌘→)", "Next chapter (⌥⌘→)"))
                    .accessibilityIdentifier("nextChapter")
            }
            ToolbarItem(placement: .principal) {
                Picker(model.t("Capitolo", "Chapter"), selection: Binding(
                    get: { model.position.chapter },
                    set: { var target = model.position; target.chapter = $0; target.verse = 0; model.navigate(target) })) {
                        ForEach(0..<model.chapterCount, id: \.self) { Text("\($0 + 1)").tag($0) }
                    }.fixedSize().accessibilityIdentifier("chapterPicker")
            }
            ToolbarItem {
                Menu {
                    Button(model.t("Importa traduzione…", "Import edition…")) { model.importEdition() }
                    Button(model.t("Mostra cartella degli appunti", "Show notes folder")) { NSWorkspace.shared.open(model.directory) }
                    Divider()
                    Button(model.t("Testi e licenze…", "Texts and licenses…")) { model.showNotices = true }
                } label: { Image(systemName: "ellipsis.circle") }
                .help(model.t("Altre opzioni", "More options"))
            }
        }
        .alert(model.t("Impossibile completare l’operazione", "Unable to complete the operation"),
               isPresented: Binding(get: { model.error != nil }, set: { if !$0 { model.error = nil } })) {
            Button("OK") { model.error = nil }
        } message: { Text(model.error ?? "") }
        .sheet(isPresented: $model.showNotices) { NoticesView(model: model) }
        .onChange(of: model.panel) { panel in
            if panel == .search { searchFocused = true }
            if panel == .saved { model.loadSaved(reset: true) }
        }
    }

    private var sidebar: some View {
        VStack(spacing: 0) {
            Picker(model.t("Navigazione", "Navigation"), selection: $model.panel) {
                Label(model.t("Libri", "Books"), systemImage: "books.vertical").tag(AppModel.Panel.books)
                Label(model.t("Cerca", "Search"), systemImage: "magnifyingglass").tag(AppModel.Panel.search)
                Label(model.t("Appunti", "Saved"), systemImage: "bookmark").tag(AppModel.Panel.saved)
            }.pickerStyle(.segmented).labelsHidden().padding(12)
            switch model.panel {
            case .books:
                ScrollViewReader { proxy in
                    List(selection: Binding<Int?>(get: { model.position.book }, set: { index in
                        if let index { var target = model.position; target.book = index; target.chapter = 0; target.verse = 0; model.navigate(target) }
                    })) {
                        ForEach(Array(model.books.enumerated()), id: \.offset) { index, book in
                            HStack {
                                Text(book.name)
                                Spacer()
                                Text("\(book.chapters.count)").font(.caption).foregroundStyle(.secondary).monospacedDigit()
                            }.tag(index).id(index)
                        }
                    }.listStyle(.sidebar).accessibilityIdentifier("bookList")
                        .onChange(of: model.position.book) { proxy.scrollTo($0) }
                        .onAppear { proxy.scrollTo(model.position.book) }
                }
            case .search:
                TextField(model.t("Parola o frase", "Word or phrase"), text: $model.query)
                    .textFieldStyle(.roundedBorder).padding(.horizontal, 12).padding(.bottom, 8)
                    .focused($searchFocused).onSubmit { model.search() }
                    .onChange(of: model.query) { _ in model.search() }
                    .accessibilityIdentifier("bibleSearch")
                if model.searching { ProgressView().controlSize(.small).padding(8) }
                results(model.hits, empty: model.t("Cerca nella traduzione aperta. Almeno due caratteri.", "Search the current edition. At least two characters."))
                if model.truncated { Text(model.t("Primi 200 risultati: affina la ricerca.", "First 200 results: refine your search.")).font(.caption).foregroundStyle(.secondary).padding(10) }
            case .saved:
                results(model.saved, empty: model.t("Le tue note, sottolineature e segnalibri appariranno qui.", "Your notes, highlights and bookmarks will appear here."))
                HStack {
                    Button { model.savedPage -= 1; model.loadSaved() } label: { Image(systemName: "chevron.left") }.disabled(model.savedPage == 0)
                    Spacer()
                    Text(model.t("Pagina", "Page") + " \(model.savedPage + 1)").font(.caption).foregroundStyle(.secondary)
                    Spacer()
                    Button { model.savedPage += 1; model.loadSaved() } label: { Image(systemName: "chevron.right") }.disabled(model.saved.count < 200)
                }.buttonStyle(.borderless).padding(12)
            }
            Divider()
            VStack(alignment: .leading, spacing: 6) {
                Text(model.t("TRADUZIONE", "EDITION")).font(.caption2).foregroundStyle(.secondary)
                Picker(model.t("Traduzione", "Edition"), selection: Binding(
                    get: { model.position.edition }, set: { model.navigate(Position(edition: $0)) })) {
                        ForEach(model.visibleEditions) { Text($0.id).tag($0.id) }
                    }.labelsHidden().frame(maxWidth: .infinity).accessibilityIdentifier("editionPicker")
                Text(model.t("\(model.books.count) libri · disponibile offline", "\(model.books.count) books · available offline"))
                    .font(.caption).foregroundStyle(.secondary)
            }.padding(12)
        }
    }

    private func results(_ hits: [Hit], empty: String) -> some View {
        Group {
            if hits.isEmpty {
                VStack(spacing: 12) {
                    Image(systemName: model.panel == .saved ? "bookmark" : "magnifyingglass").font(.largeTitle).foregroundStyle(.tertiary)
                    Text(empty).font(.callout).foregroundStyle(.secondary).multilineTextAlignment(.center)
                }.padding(24).frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                List(hits) { hit in
                    Button { model.navigate(hit.position) } label: {
                        VStack(alignment: .leading, spacing: 5) {
                            Text(hit.reference).font(.headline).foregroundStyle(.primary)
                            Text(hit.text).font(.callout).foregroundStyle(.secondary).lineLimit(3)
                            if !hit.annotation.note.isEmpty { Label(hit.annotation.note, systemImage: "note.text").font(.caption).lineLimit(2) }
                        }.padding(.vertical, 5).frame(maxWidth: .infinity, alignment: .leading).contentShape(Rectangle())
                    }.buttonStyle(.plain)
                }.listStyle(.sidebar)
            }
        }
    }

    private var reader: some View {
        VStack(spacing: 0) {
            HStack(alignment: .firstTextBaseline) {
                VStack(alignment: .leading, spacing: 5) {
                    Text(model.title).font(.system(size: 30, weight: .semibold, design: .serif))
                        .id(model.title).transition(.opacity)
                    Text(model.position.edition).font(.subheadline).foregroundStyle(.secondary)
                }.animation(reduceMotion ? nil : .easeOut(duration: 0.16), value: model.title)
                Spacer()
                if model.loading { ProgressView().controlSize(.small) }
            }.padding(.horizontal, 32).padding(.top, 26).padding(.bottom, 16)
                .frame(maxWidth: 820)
            ChapterTextView(model: model)
                .frame(maxWidth: 1600, maxHeight: .infinity)
                .accessibilityIdentifier("chapterText")
            Divider()
            HStack {
                Text(model.t("Capitolo \(model.position.chapter + 1) di \(model.chapterCount)", "Chapter \(model.position.chapter + 1) of \(model.chapterCount)"))
                Spacer()
                Text(model.pageInfo).monospacedDigit().accessibilityIdentifier("pageCounter")
                Toggle(model.t("Libro", "Book"), isOn: $model.bookMode)
                    .toggleStyle(.switch).controlSize(.small).accessibilityIdentifier("bookMode")
                    .onChange(of: model.bookMode) { _ in model.bookModeChanged() }
                Text(model.t("Seleziona una frase per annotarla", "Select a phrase to annotate it"))
            }.font(.caption).foregroundStyle(.secondary).padding(.horizontal, 24).padding(.vertical, 10)
            if model.position.edition == "CEI 2008" {
                Button { model.showNotices = true } label: {
                    Text("CEI 2008 · © Fondazione di Religione Santi Francesco d’Assisi e Caterina da Siena · " + model.t("uso didattico non commerciale", "non-commercial educational use"))
                        .font(.system(size: 10)).foregroundStyle(.secondary).multilineTextAlignment(.center)
                }.buttonStyle(.plain).padding(.horizontal, 16).padding(.bottom, 8)
            }
        }.frame(maxWidth: 1640, maxHeight: .infinity)
            .background(Color(nsColor: BookSurface.paperColor))
            .clipShape(RoundedRectangle(cornerRadius: 7))
            .overlay(alignment: .leading) {
                LinearGradient(colors: [.black.opacity(0.08), .clear], startPoint: .leading, endPoint: .trailing)
                    .frame(width: 16).allowsHitTesting(false).accessibilityHidden(true)
            }
            .overlay(RoundedRectangle(cornerRadius: 7).strokeBorder(.primary.opacity(0.09), lineWidth: 0.5))
            .shadow(color: .black.opacity(0.10), radius: 12, y: 5)
            .padding(.horizontal, 20).padding(.vertical, 16)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Color(nsColor: .windowBackgroundColor))
    }
}

private struct BookButtonStyle: ButtonStyle {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    func makeBody(configuration: Configuration) -> some View {
        configuration.label.padding(6)
            .background(Color.primary.opacity(configuration.isPressed ? 0.08 : 0), in: RoundedRectangle(cornerRadius: 6))
            .scaleEffect(reduceMotion ? 1 : configuration.isPressed ? 0.94 : 1)
            .animation(reduceMotion ? nil : .spring(response: 0.24, dampingFraction: 0.82), value: configuration.isPressed)
    }
}
