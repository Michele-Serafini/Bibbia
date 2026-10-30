import AppKit
import UniformTypeIdentifiers
import BibleKit

extension AppModel {
    func importPlanCalendar(_ id:Int,save:Bool) async throws {
        guard let store else{return}
        let file:URL
        if save {
            let panel=NSSavePanel();panel.allowedContentTypes=[UTType(filenameExtension:"ics") ?? .data];panel.nameFieldStringValue="Bibbia-piano-di-lettura.ics"
            let response=await withCheckedContinuation{ continuation in panel.begin{continuation.resume(returning:$0)} }
            guard response == .OK,let target=panel.url else{return};file=target
        }else{
            let folder=directory.appendingPathComponent("calendar",isDirectory:true);try FileManager.default.createDirectory(at:folder,withIntermediateDirectories:true);file=folder.appendingPathComponent("Piano-\(id).ics")
        }
        try await store.exportReadingCalendar(id,to:file,language:preferences.language)
        if save{announceAnnotation(t("Calendario salvato · Importalo nella tua app calendario.","Calendar saved · Import it into your calendar app."))}
        else if !NSWorkspace.shared.open(file){throw BibleError.invalid(t("Scegli un’app per i file .ics, oppure usa Salva calendario .ics… e importa il file nel calendario che preferisci.","Choose an app for .ics files, or use Save calendar .ics… and import the file into your preferred calendar."))}
    }
}
