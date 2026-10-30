import SwiftUI
import BibleKit

struct NoteFiltersView: View {
    @ObservedObject var model: AppModel
    private let all = "\0all"
    var body: some View {
        VStack(spacing: 8) {
            TextField(model.t("Cerca nel testo salvato e negli appunti…", "Search saved text and notes…"), text: $model.noteFilter.query)
                .textFieldStyle(.roundedBorder).accessibilityIdentifier("notesQuery")
            Picker(model.t("Cartella", "Folder"), selection: Binding(get: { model.noteFilter.folder ?? all }, set: { model.noteFilter.folder = $0 == all ? nil : $0 })) {
                Text(model.t("Tutte le cartelle", "All folders")).tag(all)
                Text(model.t("Senza cartella", "Unfiled")).tag("")
                ForEach(model.noteCollections.folders, id: \.self) { Text($0).tag($0) }
            }.accessibilityIdentifier("notesFolder")
            Picker(model.t("Etichetta", "Tag"), selection: Binding(get: { model.noteFilter.tag ?? all }, set: { model.noteFilter.tag = $0 == all ? nil : $0 })) {
                Text(model.t("Tutte le etichette", "All tags")).tag(all)
                ForEach(model.noteCollections.tags, id: \.self) { Text($0).tag($0) }
            }.accessibilityIdentifier("notesTag")
            Picker(model.t("Mostra", "Show"), selection: $model.noteFilter.kind) {
                Text(model.t("Tutti gli appunti", "All annotations")).tag("all")
                Text(model.t("Note", "Notes")).tag("note")
                Text(model.t("Evidenziazioni e segni", "Highlights and marks")).tag("marked")
                Text(model.t("Segnalibri", "Bookmarks")).tag("bookmark")
            }
            HStack {
                Text(model.position.edition).font(.caption2).foregroundStyle(.secondary).lineLimit(1)
                Spacer()
                Button(model.t("Gestisci…", "Manage…")) { model.showCollections = true }.buttonStyle(.borderless)
            }
        }.padding(.horizontal, 12).padding(.bottom, 10)
    }
}

struct NoteOrganizationSheet: View {
    @ObservedObject var model: AppModel
    let hit: Hit
    @Environment(\.dismiss) private var dismiss
    @State private var folder: String
    @State private var tags: String
    @State private var error = ""
    @State private var saving = false
    init(model: AppModel, hit: Hit) {
        self.model = model; self.hit = hit
        _folder = State(initialValue: hit.organization.folder); _tags = State(initialValue: hit.organization.tags.joined(separator: ", "))
    }
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(hit.reference).font(.headline)
            Form {
                HStack {
                    TextField(model.t("Cartella", "Folder"), text: $folder).accessibilityIdentifier("organizeFolder")
                    Menu {
                        Button(model.t("Senza cartella", "Unfiled")) { folder = "" }
                        ForEach(model.noteCollections.folders, id: \.self) { value in Button(value) { folder = value } }
                    } label: { Image(systemName: "folder") }.menuStyle(.borderlessButton).fixedSize()
                }
                TextField(model.t("Etichette", "Tags"), text: $tags).accessibilityIdentifier("organizeTags")
            }
            Text(model.t("Scrivi nomi nuovi per crearli. Separa le etichette con virgole; lascia la cartella vuota per rimuoverla.", "Type new names to create them. Separate tags with commas; leave the folder empty to remove it.")).font(.caption).foregroundStyle(.secondary)
            AnnotationDeleteControls(model:model,hit:hit,completion:{dismiss()}).disabled(saving)
            if !error.isEmpty { Text(error).font(.caption).foregroundStyle(.red) }
            HStack {
                Button(model.t("Annulla", "Cancel")) { dismiss() }.keyboardShortcut(.cancelAction)
                Spacer()
                Button(model.t("Salva", "Save")) {
                    saving = true
                    Task { do { try await model.saveOrganization(hit, folder: folder, tags: tags); dismiss() }
                        catch { self.error = error.localizedDescription; saving = false } }
                }.keyboardShortcut(.defaultAction).disabled(saving).accessibilityIdentifier("saveOrganization")
            }
        }.padding(24).frame(width: 400)
    }
}

struct NoteCollectionsSheet: View {
    @ObservedObject var model: AppModel
    @Environment(\.dismiss) private var dismiss
    @State private var kind = "folder"
    @State private var selected = ""
    @State private var name = ""
    @State private var error = ""
    @State private var saving = false
    private var values: [String] { kind == "folder" ? model.noteCollections.folders : model.noteCollections.tags }
    private func change(_ action: String) {
        saving = true
        Task {
            do {
                try await model.changeCollection(kind: kind, action: action, name: action == "create" ? name : selected, newName: name)
                selected = ""; name = ""; error = ""
            } catch { self.error = error.localizedDescription }
            saving = false
        }
    }
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(model.t("Cartelle ed etichette", "Folders and tags")).font(.headline)
            Form {
                Picker(model.t("Tipo", "Kind"), selection: $kind) {
                    Text(model.t("Cartelle", "Folders")).tag("folder"); Text(model.t("Etichette", "Tags")).tag("tag")
                }.onChange(of: kind) { _ in selected = "" }
                Picker(model.t("Esistente", "Existing"), selection: $selected) {
                    Text("—").tag(""); ForEach(values, id: \.self) { Text($0).tag($0) }
                }
                TextField(model.t("Nome nuovo", "New name"), text: $name)
            }
            Text(model.t("Rinominare o eliminare una raccolta conserva tutti gli appunti.", "Renaming or deleting a collection keeps every annotation.")).font(.caption).foregroundStyle(.secondary)
            if !error.isEmpty { Text(error).font(.caption).foregroundStyle(.red) }
            HStack {
                Button(model.t("Crea", "Create")) { change("create") }.disabled(name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                Button(model.t("Rinomina", "Rename")) { change("rename") }.disabled(selected.isEmpty || name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                Button(model.t("Elimina", "Delete"), role: .destructive) { change("delete") }.disabled(selected.isEmpty)
                Spacer()
                Button(model.t("Fine", "Done")) { dismiss() }.keyboardShortcut(.cancelAction)
            }.disabled(saving)
        }.padding(24).frame(width: 430)
    }
}

@MainActor func noteTint(_ color:String?) -> Color { Color(nsColor:ReadingTextView.highlightColor(color ?? "yellow") ?? .systemYellow) }
struct SavedNoteEditor: View {
    @ObservedObject var model:AppModel
    let hit:Hit
    @Environment(\.dismiss) private var dismiss
    @State private var text=""
    @State private var color="yellow"
    @State private var style="plain"
    @State private var error=""
    var body:some View {
        VStack(alignment:.leading,spacing:12){
            Text(hit.reference).font(.headline)
            NoteStylePicker(model:model,color:$color,style:$style)
            TextEditor(text:$text).frame(height:180).padding(6).background(noteTint(color).opacity(style=="sticky" ? 0.25:0.08)).accessibilityIdentifier("savedNoteText")
            AnnotationDeleteControls(model:model,hit:hit,completion:{dismiss()})
            if !error.isEmpty{Text(error).foregroundStyle(.red)}
            HStack{Button(model.t("Annulla","Cancel")){dismiss()};Spacer();Button(model.t("Salva","Save")){Task{do{if let verses=try await model.store?.writeNote(text,color:color,style:style,at:hit.position){model.apply(verses,at:hit.position)};dismiss()}catch{self.error=error.localizedDescription}}}.disabled(text.utf8.count>32768)}
        }.padding(24).frame(width:420).onAppear{text=hit.annotation.note;color=hit.annotation.noteColor ?? "yellow";style=hit.annotation.noteStyle ?? "plain"}
    }
}
struct NoteStylePicker:View {
    @ObservedObject var model:AppModel
    @Binding var color:String
    @Binding var style:String
    var body:some View {
        HStack {
            Picker(model.t("Colore","Color"),selection:$color){ForEach([("yellow","Giallo","Yellow"),("green","Verde","Green"),("blue","Blu","Blue"),("pink","Rosa","Pink")],id:\.0){v in Text(model.t(v.1,v.2)).tag(v.0)}}
            Picker(model.t("Stile","Style"),selection:$style){Text(model.t("Nota","Note")).tag("plain");Text("Post-it").tag("sticky")}
        }
    }
}
