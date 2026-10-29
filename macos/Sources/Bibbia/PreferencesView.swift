import SwiftUI
import BibleKit

struct PreferencesView: View {
    @ObservedObject var model: AppModel
    var body: some View {
        Form {
            Section(model.t("Aspetto", "Appearance")) {
                Picker(model.t("Tema", "Theme"), selection: $model.preferences.theme) {
                    Text(model.t("Come il Mac", "Follow Mac appearance")).tag("Default")
                    Text(model.t("Chiaro", "Light")).tag("Light")
                    Text(model.t("Scuro", "Dark")).tag("Dark")
                    Text(model.t("Carta seppia", "Sepia paper")).tag("Sepia")
                }
                HStack {
                    Text(model.t("Dimensione del testo", "Text size"))
                    Slider(value: Binding(get: { Double(model.preferences.fontSize) }, set: { model.preferences.fontSize = Int($0) }), in: 14...32, step: 1)
                    Text("\(model.preferences.fontSize)").monospacedDigit().frame(width: 28)
                }
                HStack {
                    Text(model.t("Margini della pagina", "Page margins"))
                    Slider(value: Binding(get: { Double(model.preferences.readingMargin) }, set: { model.preferences.readingMargin = Int($0) }), in: 16...96, step: 1)
                        .accessibilityIdentifier("readingMargins")
                    Text("\(model.preferences.readingMargin)").monospacedDigit().frame(width:28)
                }
                Toggle(model.t("Modalità concentrazione", "Focus mode"),isOn:$model.preferences.focusMode)
                Text(model.t("Nascondi gli strumenti · ⇧⌘F · Esc per uscire", "Hide tools · ⇧⌘F · Esc to leave")).font(.caption).foregroundStyle(.secondary)
                Text(model.t("Caratteri di sistema Apple: San Francisco per i controlli, disegno serif per la lettura.", "Apple system fonts: San Francisco for controls, serif design for reading."))
                    .font(.caption).foregroundStyle(.secondary)
            }
            Section(model.t("Lingue", "Languages")) {
                Picker(model.t("Lingua dell’app", "App language"), selection: $model.preferences.language) {
                    Text("Italiano").tag("it"); Text("English").tag("en")
                }
                Picker(model.t("Filtra traduzioni", "Filter editions"), selection: $model.preferences.bibleLanguage) {
                    Text(model.t("Tutte", "All")).tag("all")
                    Text("Italiano").tag("it"); Text("English").tag("en")
                }
                Text(model.t("La lingua del testo dipende dalla traduzione scelta.", "The text language depends on the selected edition."))
                    .font(.caption).foregroundStyle(.secondary)
            }
        }.formStyle(.grouped).padding().frame(width: 490)
            .onChange(of: model.preferences) { _ in model.preferencesChanged() }
    }
}

struct NoticesView: View {
    @ObservedObject var model: AppModel
    @Environment(\.dismiss) private var dismiss
    @State private var notice = ""
    @State private var selected = "THIRD_PARTY_NOTICES.md"
    private let files = ["THIRD_PARTY_NOTICES.md", "LICENSE.cei2008.txt", "LICENSE.gutenberg.txt", "LICENSE.bible-data", "LICENSE"]
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(model.t("Testi e licenze", "Texts and licenses")).font(.title2).bold()
            Text(model.t("CEI: uso formativo e didattico non commerciale. La raccolta estesa comprende 83 sezioni di più canoni, in inglese; non è l’intero canone etiopico.", "CEI: non-commercial educational use. The extended collection has 83 sections from multiple canons in English; it is not the entire Ethiopian canon."))
                .font(.callout).foregroundStyle(.secondary)
            Picker(model.t("Documento", "Document"), selection: $selected) {
                ForEach(files, id: \.self) { Text($0).tag($0) }
            }
            ScrollView { Text(notice).font(.system(.body, design: .monospaced)).textSelection(.enabled).frame(maxWidth: .infinity, alignment: .leading).padding(12) }
                .background(Color(nsColor: .textBackgroundColor)).clipShape(RoundedRectangle(cornerRadius: 8))
            HStack { Spacer(); Button(model.t("Chiudi", "Close")) { dismiss() }.keyboardShortcut(.defaultAction) }
        }.padding(24).frame(width: 680, height: 520)
            .task { load() }.onChange(of: selected) { _ in load() }
    }
    private func load() { notice = (try? String(contentsOf: BibleStore.notices.appendingPathComponent(selected), encoding: .utf8)) ?? "Document unavailable" }
}
