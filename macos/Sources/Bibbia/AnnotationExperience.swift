import SwiftUI
import AppKit
import BibleKit

extension AppModel {
    var annotationSummary:String {
        var notes=0,bookmarks=0,marks=0
        for verse in verses {if !verse.annotation.note.isEmpty{notes+=1};if verse.annotation.bookmark{bookmarks+=1};if verse.annotation.underline || verse.annotation.highlight != nil || !verse.annotation.ranges.isEmpty {marks+=1}}
        return notes+bookmarks+marks==0 ? "":t("\(notes) note · \(bookmarks) segnalibri · \(marks) segni","\(notes) notes · \(bookmarks) bookmarks · \(marks) marks")
    }
    func showBookmarks(){
        if preferences.focusMode{toggleFocus()};var filter=NoteFilter();filter.kind="bookmark";noteFilter=filter;panel = .saved;loadSaved(reset:true)
    }
    func announceAnnotation(_ message:String){
        feedbackTask?.cancel();annotationFeedback=message
        feedbackTask=Task{[weak self] in do{try await Task.sleep(for:.seconds(3));self?.annotationFeedback=""}catch{}}
    }
    func deleteAnnotation(_ hit:Hit,kind:String) async throws {
        guard let store else{return};let verses=try await store.deleteAnnotation(at:hit.position,kind:kind);apply(verses,at:hit.position)
        if saved.count==1 && savedPage>0{savedPage-=1};loadSaved();announceAnnotation(t("Appunti eliminati.","Annotations deleted."))
    }
}
struct AnnotationFeedbackView:View {
    @ObservedObject var model:AppModel
    var body:some View {
        if !model.annotationFeedback.isEmpty {Text(model.annotationFeedback).font(.callout).padding(.horizontal,16).padding(.vertical,10).background(Color(nsColor:.controlBackgroundColor),in:RoundedRectangle(cornerRadius:9)).shadow(radius:4,y:2).padding(24).allowsHitTesting(false).accessibilityIdentifier("annotationFeedback")}
    }
}
struct BookmarkActions:View {
    @ObservedObject var model:AppModel
    let target:Position
    let close:()->Void
    var body:some View {
        VStack(alignment:.leading,spacing:10){
            Label(model.t("Segnalibro salvato","Saved bookmark"),systemImage:"bookmark.fill").font(.headline)
            Button(model.t("Mostra tutti i segnalibri","Show all bookmarks")){close();model.showBookmarks()}
            Button(model.t("Rimuovi segnalibro","Remove bookmark"),role:.destructive){close();Task{do{if let verses=try await model.store?.deleteAnnotation(at:target,kind:"bookmark"){model.apply(verses,at:target);model.announceAnnotation(model.t("Segnalibro rimosso.","Bookmark removed."))}}catch{model.error=error.localizedDescription}}}
        }.padding(14).frame(width:280)
    }
}
struct AnnotationDeleteControls:View {
    @ObservedObject var model:AppModel
    let hit:Hit
    let completion:()->Void
    @State private var kind="all"
    @State private var confirming=false
    @State private var deleting=false
    @State private var error=""
    var body:some View {
        VStack(alignment:.leading,spacing:7){
            if !hit.annotation.note.isEmpty {Button(model.t("Elimina solo la nota…","Delete note only…"),role:.destructive){kind="note";confirming=true}.accessibilityIdentifier("deleteNoteOnly")}
            Button(model.t("Elimina tutti gli appunti del versetto…","Delete all annotations on this verse…"),role:.destructive){kind="all";confirming=true}.accessibilityIdentifier("deleteAnnotation")
            if !error.isEmpty{Text(error).font(.caption).foregroundStyle(.red)}
        }.disabled(deleting).confirmationDialog(kind=="note" ? model.t("Eliminare la nota?","Delete this note?") : model.t("Eliminare tutti gli appunti del versetto?","Delete every annotation on this verse?"),isPresented:$confirming){
            Button(model.t("Elimina","Delete"),role:.destructive){deleting=true;Task{do{try await model.deleteAnnotation(hit,kind:kind);completion()}catch{self.error=error.localizedDescription;deleting=false}}}
        }message:{Text(kind=="note" ? model.t("Segnalibro, evidenziazioni e sottolineature saranno conservati.","The bookmark, highlights and underlines will be kept.") : model.t("Verranno rimossi nota, segnalibro, segni, cartella ed etichette associati al versetto.","This removes the note, bookmark, marks, folder and tags assigned to the verse."))}
    }
}
