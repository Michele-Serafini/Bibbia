import SwiftUI
import BibleKit

struct MainView: View {
    @ObservedObject var model: AppModel
    @FocusState private var searchFocused: Bool
    @State private var columns: NavigationSplitViewVisibility = .automatic
    @State private var previousColumns: NavigationSplitViewVisibility = .automatic
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        NavigationSplitView(columnVisibility: $columns) {
            sidebar.navigationSplitViewColumnWidth(min: 230, ideal: 270, max: 340)
        } detail: {
            reader
        }
        .navigationTitle(model.title)
        .toolbar {
            if !model.preferences.focusMode {
            ToolbarItemGroup(placement: .navigation) {
                Button { model.step(-1) } label: { Image(systemName: "chevron.left") }
                    .buttonStyle(BookButtonStyle())
                    .help(model.bookMode && !model.comparing ? model.t("Pagina precedente (⌥⌘←)", "Previous page (⌥⌘←)")
                          : model.t("Capitolo precedente (⌥⌘←)", "Previous chapter (⌥⌘←)"))
                    .accessibilityIdentifier("previousChapter")
                Button { model.step(1) } label: { Image(systemName: "chevron.right") }
                    .buttonStyle(BookButtonStyle())
                    .help(model.bookMode && !model.comparing ? model.t("Pagina successiva (⌥⌘→)", "Next page (⌥⌘→)")
                          : model.t("Capitolo successivo (⌥⌘→)", "Next chapter (⌥⌘→)"))
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
                Toggle(isOn: $model.comparing) { Label(model.t("Confronta","Compare"),systemImage: "rectangle.split.2x1") }
                    .toggleStyle(.button).accessibilityIdentifier("comparisonToggle")
                    .onChange(of: model.comparing) { _ in model.comparisonChanged() }
            }
            ToolbarItem {
                Button { model.showBookmarks() } label: { Label(model.t("Segnalibri","Bookmarks"),systemImage:"bookmark.fill") }
                    .help(model.t("Mostra tutti i segnalibri","Show all bookmarks")).accessibilityIdentifier("showBookmarks")
            }
            ToolbarItem {
                Button { model.showReadingPlans = true } label: { Label(model.t("Piani di lettura","Reading plans"),systemImage:"calendar") }.accessibilityIdentifier("readingPlans")
            }
            ToolbarItem {
                Menu {
                    Button(model.t("Importa traduzione…", "Import edition…")) { model.importEdition() }
                    Button(model.t("Esportazione e backup…","Export and backup…")){model.showTransfer=true}
                    Button(model.t("Mostra cartella degli appunti", "Show notes folder")) { NSWorkspace.shared.open(model.directory) }
                    Divider()
                    Button(model.t("Testi e licenze…", "Texts and licenses…")) { model.showNotices = true }
                } label: { Image(systemName: "ellipsis.circle") }
                .help(model.t("Altre opzioni", "More options"))
            }
            }
            ToolbarItem {
                Button { model.toggleFocus() } label: { Image(systemName: model.preferences.focusMode ? "arrow.down.right.and.arrow.up.left" : "viewfinder") }
                    .help(model.t("Concentrazione · ⇧⌘F · Esc per uscire", "Focus · ⇧⌘F · Esc to leave"))
                    .accessibilityIdentifier("focusToggle")
            }
        }
        .overlay(alignment:.bottom) { AnnotationFeedbackView(model:model) }
        .alert(model.t("Impossibile completare l’operazione", "Unable to complete the operation"),
               isPresented: Binding(get: { model.error != nil }, set: { if !$0 { model.error = nil } })) {
            Button("OK") { model.error = nil }
        } message: { Text(model.error ?? "") }
        .sheet(isPresented: $model.showNotices) { NoticesView(model: model) }
        .sheet(item: $model.organizingNote) { hit in NoteOrganizationSheet(model: model, hit: hit) }
        .sheet(isPresented: $model.showTransfer){TransferView(model:model)}
        .sheet(isPresented: $model.showReadingPlans) { ReadingPlansView(model:model) }
        .sheet(item: $model.editingNote) { hit in SavedNoteEditor(model:model,hit:hit) }
        .sheet(isPresented: $model.showCollections) { NoteCollectionsSheet(model: model) }
        .onChange(of: model.preferences.focusMode) { active in
            if active { previousColumns=columns; columns = .detailOnly }
            else { columns=previousColumns }
        }
        .onChange(of: columns) { value in
            if model.preferences.focusMode && value != .detailOnly { model.toggleFocus() }
        }
        .onChange(of: model.noteFilter) { _ in model.loadSaved(reset: true) }
        .onChange(of: model.panel) { panel in
            if model.preferences.focusMode { model.toggleFocus() }
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
                NoteFiltersView(model: model)
                results(model.saved, empty: model.noteFilter.kind=="bookmark" ? model.t("Seleziona un versetto e premi Aggiungi segnalibro: potrai ritrovarlo e riaprirlo qui.","Select a verse and choose Add bookmark: find and reopen it here.") : model.t("Le tue note, sottolineature e segnalibri appariranno qui.", "Your notes, highlights and bookmarks will appear here."))
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
                    HStack(alignment: .top) {
                    Button { model.navigate(hit.position) } label: {
                        VStack(alignment: .leading, spacing: 5) {
                            Text(hit.reference).font(.headline).foregroundStyle(.primary)
                            Text(hit.text).font(.callout).foregroundStyle(.secondary).lineLimit(3)
                            if hit.annotation.underline || hit.annotation.highlight != nil || !hit.annotation.ranges.isEmpty { Label(model.t("Evidenziato o sottolineato","Highlighted or underlined"),systemImage:"highlighter").font(.caption).foregroundStyle(.secondary) }
                            if hit.annotation.bookmark { Label(model.t("Segnalibro salvato","Saved bookmark"),systemImage:"bookmark.fill").font(.caption.bold()).foregroundStyle(Color.accentColor) }
                            if !hit.annotation.note.isEmpty { Label(hit.annotation.note, systemImage: hit.annotation.noteStyle == "sticky" ? "pin.fill":"note.text").font(.caption).lineLimit(3).padding(7).frame(maxWidth:.infinity,alignment:.leading).background(noteTint(hit.annotation.noteColor).opacity(hit.annotation.noteStyle == "sticky" ? 0.25:0.10),in:RoundedRectangle(cornerRadius:7)) }
                            if !hit.organization.summary.isEmpty { Text(hit.organization.summary).font(.caption2).foregroundStyle(.secondary).lineLimit(2) }
                        }.padding(.vertical, 5).frame(maxWidth: .infinity, alignment: .leading).contentShape(Rectangle())
                    }.buttonStyle(.plain)
                        .contextMenu { if model.panel == .saved { Button(model.t("Organizza…", "Organize…")) { model.organizingNote = hit } } }
                    if model.panel == .saved {
                        Button { model.editingNote = hit } label: { Image(systemName:"square.and.pencil") }.buttonStyle(.borderless).help(model.t("Modifica nota","Edit note"))
                        Button { model.organizingNote = hit } label: { Image(systemName: "folder") }
                            .buttonStyle(.borderless).help(model.t("Organizza…", "Organize…")).accessibilityLabel(model.t("Organizza…", "Organize…")).accessibilityIdentifier("organizeNote")
                    }
                    }
                }.listStyle(.sidebar)
            }
        }
    }

    private var reader: some View {
        VStack(spacing: 0) {
            if !model.preferences.focusMode {
            HStack(alignment: .firstTextBaseline) {
                VStack(alignment: .leading, spacing: 5) {
                    Text(model.title).font(.system(size: 30, weight: .semibold, design: .serif))
                        .id(model.title).transition(.opacity)
                    Text(model.position.edition+(model.annotationSummary.isEmpty ? "":" · "+model.annotationSummary)).font(.subheadline).foregroundStyle(.secondary)
                }.animation(reduceMotion ? nil : .easeOut(duration: 0.16), value: model.title)
                Spacer()
                if model.loading { ProgressView().controlSize(.small) }
            }.padding(.horizontal, 32).padding(.top, 26).padding(.bottom, 16)
                .frame(maxWidth: 820)
            }
            Group {
                if model.comparing { ComparisonView(model: model) }
                else { ChapterTextView(model: model).accessibilityIdentifier("chapterText") }
            }.frame(maxWidth: 1600, maxHeight: .infinity)
            if !model.preferences.focusMode {
            Divider()
            HStack {
                Text(model.t("Capitolo \(model.position.chapter + 1) di \(model.chapterCount)", "Chapter \(model.position.chapter + 1) of \(model.chapterCount)"))
                Spacer()
                Text(model.comparing ? "" : model.pageInfo).monospacedDigit().accessibilityIdentifier("pageCounter")
                Toggle(model.t("Libro", "Book"), isOn: $model.bookMode)
                    .toggleStyle(.switch).controlSize(.small).accessibilityIdentifier("bookMode")
                    .disabled(model.comparing)
                    .onChange(of: model.bookMode) { _ in model.bookModeChanged() }
                Text(model.annotationSummary.isEmpty ? model.t("Seleziona una frase per annotarla", "Select a phrase to annotate it") : model.annotationSummary)
            }.font(.caption).foregroundStyle(.secondary).padding(.horizontal, 24).padding(.vertical, 10)
            }
                if model.preferences.focusMode {
                    HStack(spacing: 10) {
                        Button { model.step(-1) } label: { Image(systemName:"chevron.left") }.help(model.t("Pagina precedente","Previous page"))
                        Button { model.step(1) } label: { Image(systemName:"chevron.right") }.help(model.t("Pagina successiva","Next page"))
                        Button { model.toggleFocus() } label: { Image(systemName:"xmark") }.help(model.t("Esci dalla concentrazione · Esc","Leave focus · Esc")).keyboardShortcut(.cancelAction)
                    }.buttonStyle(.borderless).padding(10).background(Color(nsColor:model.windowColor),in:Capsule()).padding(.horizontal,22).padding(.vertical,8).frame(maxWidth:.infinity,alignment:.trailing)
                }
            if model.position.edition == "CEI 2008" || (model.comparing && model.preferences.comparisonEdition == "CEI 2008") {
                Button { model.showNotices = true } label: {
                    Text("CEI 2008 · © Fondazione di Religione Santi Francesco d’Assisi e Caterina da Siena · " + model.t("uso didattico non commerciale", "non-commercial educational use"))
                        .font(.system(size: 10)).foregroundStyle(.secondary).multilineTextAlignment(.center)
                }.buttonStyle(.plain).padding(.horizontal, 16).padding(.bottom, 8)
            }
        }.frame(maxWidth: 1640, maxHeight: .infinity)
            .background(Color(nsColor: BookSurface.paperColor(sepia: model.sepia)))
            .clipShape(RoundedRectangle(cornerRadius: 7))
            .overlay(alignment: .leading) {
                LinearGradient(colors: [.black.opacity(0.08), .clear], startPoint: .leading, endPoint: .trailing)
                    .frame(width: 16).allowsHitTesting(false).accessibilityHidden(true)
            }
            .overlay(RoundedRectangle(cornerRadius: 7).strokeBorder(.primary.opacity(0.09), lineWidth: 0.5))
            .shadow(color: .black.opacity(0.10), radius: 12, y: 5)
            .padding(.horizontal, 20).padding(.vertical, 16)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Color(nsColor: model.windowColor))

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
