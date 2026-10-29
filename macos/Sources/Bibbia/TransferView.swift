import SwiftUI
import AppKit
import CoreText
import BibleKit
import UniformTypeIdentifiers

struct TransferView:View {
    @ObservedObject var model:AppModel
    @Environment(\.dismiss) private var dismiss
    @State private var busy=false
    @State private var status=""
    var body:some View {
        VStack(alignment:.leading,spacing:18){
            Text(model.t("Esportazione e backup","Export and backup")).font(.title2.bold())
            Text(model.t("Esporta tutti gli appunti in documenti leggibili, oppure trasferisci il backup su Windows, Linux o un altro Mac.","Export all annotations as readable documents, or transfer your backup to Windows, Linux or another Mac."))
            ForEach([("markdown","Esporta Markdown…","Export Markdown…"),("pdf","Esporta PDF…","Export PDF…"),("backup","Crea backup…","Create backup…"),("restore","Ripristina backup…","Restore backup…")],id:\.0){action in
                Button(model.t(action.1,action.2)){choose(action.0)}.disabled(busy).accessibilityIdentifier("transfer-"+action.0)
            }
            Text(model.t("Il ripristino sostituisce i dati al riavvio e conserva una copia dei dati precedenti nella cartella dell’app.","Restore replaces your data on restart and keeps a copy of your previous data in the app’s folder.")).font(.caption).foregroundStyle(.secondary)
            if busy{ProgressView().controlSize(.small)}
            if !status.isEmpty{Text(status).font(.callout).textSelection(.enabled)}
            HStack{Spacer();Button(model.t("Chiudi","Close")){dismiss()}.disabled(busy)}
        }.padding(24).frame(width:460).interactiveDismissDisabled(busy)
    }
    private func choose(_ action:String){
        if action=="restore" {
            let chooser=NSOpenPanel();chooser.canChooseDirectories=false;chooser.allowsMultipleSelection=false;chooser.message=model.t("Seleziona un file .bibbia-backup","Select a .bibbia-backup file")
            chooser.begin {response in if response == .OK,let url=chooser.url{Task{await run(action,url)}}};return
        }
        let picker=NSSavePanel();let ext=action=="markdown" ? "md":action=="pdf" ? "pdf":"bibbia-backup"
        picker.nameFieldStringValue="Bibbia-\(ReadingDates.key(Date())).\(ext)";picker.allowedContentTypes=[UTType(filenameExtension:ext) ?? .data]
        picker.begin{response in if response == .OK,let url=picker.url{Task{await run(action,url)}}}
    }
    private func run(_ action:String,_ url:URL) async {
        guard let store=model.store else{return};busy=true;defer{busy=false}
        do {
            if action=="restore" {
                let summary=try await store.inspectBackup(url)
                let dialog=NSAlert();dialog.messageText=model.t("Ripristinare il backup?","Restore backup?")
                dialog.informativeText="\(summary.annotations) "+model.t("appunti","annotations")+" · \(summary.plans) "+model.t("piani","plans")+"\n\n"+model.t("L’app si chiuderà. Riaprila per completare il ripristino. Una copia dei dati attuali sarà conservata nella cartella dell’app.","The app will close. Reopen it to complete restoration. A copy of your current data will be kept in the app’s folder.")
                dialog.addButton(withTitle:model.t("Ripristina e chiudi","Restore and close"));dialog.addButton(withTitle:model.t("Annulla","Cancel"))
                guard dialog.runModal() == .alertFirstButtonReturn else{return}
                _ = try await store.restoreBackup(url);NSApp.terminate(nil);return
            }
            if action=="backup" {try await store.createBackup(to:url)}
            else if action=="markdown" {_ = try await store.exportNotes(to:url,markdown:true)}
            else {
                try await store.validateExportDestination(url)
                let text=FileManager.default.temporaryDirectory.appendingPathComponent("bibbia-export-\(UUID().uuidString).txt")
                defer{try? FileManager.default.removeItem(at:text)}
                _ = try await store.exportNotes(to:text,markdown:false)
                try await Task.detached(priority:.userInitiated){try NotesPDF.write(text:text,to:url)}.value
            }
            status=model.t("File salvato.","File saved.")
        }catch{status=error.localizedDescription}
    }
}

/// One UTF-8 line and one native CoreText layout at a time; no offscreen UI pages.
enum NotesPDF {
    static func write(text:URL,to output:URL) throws {
        let temp=output.deletingLastPathComponent().appendingPathComponent(".bibbia-\(UUID().uuidString).pdf")
        defer{try? FileManager.default.removeItem(at:temp)}
        var media=CGRect(x:0,y:0,width:595.28,height:841.89)
        guard let pdf=CGContext(temp as CFURL,mediaBox:&media,[kCGPDFContextTitle:"Bibbia · Appunti",kCGPDFContextCreator:"Bibbia"] as CFDictionary) else{throw BibleError.invalid("Impossibile creare il PDF / Cannot create PDF")}
        let input=try FileHandle(forReadingFrom:text);defer{try? input.close()}
        var buffer=Data(),y:CGFloat=794,first=true
        let bodyFont=CTFontCreateWithName("Helvetica" as CFString,11,nil)
        let titleFont=CTFontCreateWithName("Helvetica-Bold" as CFString,18,nil)
        pdf.textMatrix = .identity
        pdf.beginPDFPage(nil)
        func newPage(){pdf.endPDFPage();pdf.beginPDFPage(nil);y=794}
        func draw(_ line:String){
            let font=first ? titleFont:bodyFont;first=false
            if line.isEmpty{y-=8;return}
            let value=NSAttributedString(string:line,attributes:[NSAttributedString.Key(kCTFontAttributeName as String):font])
            let setter=CTTypesetterCreateWithAttributedString(value as CFAttributedString),length=value.length
            var offset=0
            while offset<length {
                let count=max(1,CTTypesetterSuggestLineBreak(setter,offset,499))
                let row=CTTypesetterCreateLine(setter,CFRange(location:offset,length:count))
                var ascent:CGFloat=0,descent:CGFloat=0,leading:CGFloat=0
                _ = CTLineGetTypographicBounds(row,&ascent,&descent,&leading)
                let height=max(14,ascent+descent+leading)+2
                if y-height<48{newPage()};pdf.textPosition=CGPoint(x:48,y:y-ascent);CTLineDraw(row,pdf);y-=height;offset+=count
            }
        }
        while let data=try input.read(upToCount:8192),!data.isEmpty {
            buffer.append(data)
            while let index=buffer.firstIndex(of:10){let line=String(decoding:buffer[..<index],as:UTF8.self);buffer.removeSubrange(...index);autoreleasepool{draw(line)}}
        }
        if !buffer.isEmpty{draw(String(decoding:buffer,as:UTF8.self))}
        pdf.endPDFPage();pdf.closePDF()
        guard FileManager.default.fileExists(atPath:temp.path) else{throw BibleError.invalid("PDF non creato / PDF not created")}
        if rename(temp.path,output.path) != 0{throw BibleError.invalid("Impossibile salvare il PDF / Cannot save PDF")}

    }
}
