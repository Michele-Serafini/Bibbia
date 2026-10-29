import Foundation
import CSQLite
#if canImport(Darwin)
import Darwin
#else
import Glibc
#endif

public struct BackupSummary: Sendable {
    public let annotations:Int
    public let editions:Int
    public let plans:Int
    public let groups:Int
}
private let backupID:Int32=0x42494242
private let pendingName="pending-restore.sqlite3"
private let backupTables:[(String,[String])]=[
    ("annotations",["edition","book","chapter","verse","payload"]),("settings",["key","payload"]),
    ("editions",["id","payload","book_count"]),("note_groups",["kind","name"]),
    ("note_organization",["edition","book","chapter","verse","folder","tags"]),
    ("reading_plans",["id","payload"]),("reading_progress",["plan_id","chapter_index"])]
private func sqlError(_ db:OpaquePointer?) -> BibleError {.invalid(db.map{String(cString:sqlite3_errmsg($0))} ?? "SQLite unavailable")}
private func exec(_ db:OpaquePointer?,_ sql:String) throws {guard sqlite3_exec(db,sql,nil,nil,nil)==SQLITE_OK else{throw sqlError(db)}}
private func values(_ db:OpaquePointer?,_ sql:String) throws -> [[String]] {
    var stmt:OpaquePointer?;guard sqlite3_prepare_v2(db,sql,-1,&stmt,nil)==SQLITE_OK else{throw sqlError(db)};defer{sqlite3_finalize(stmt)}
    var result=[[String]]()
    while true {let code=sqlite3_step(stmt);if code==SQLITE_DONE{return result};guard code==SQLITE_ROW else{throw sqlError(db)}
        result.append((0..<sqlite3_column_count(stmt)).map{sqlite3_column_text(stmt,$0).map{String(cString:$0)} ?? ""})
    }
}
private func copyDatabase(_ source:OpaquePointer?,_ target:OpaquePointer?) throws {
    guard let job=sqlite3_backup_init(target,"main",source,"main") else{throw sqlError(target)}
    var code:Int32=SQLITE_OK;var deadline=Date().addingTimeInterval(5)
    repeat {code=sqlite3_backup_step(job,256);if code==SQLITE_OK{deadline=Date().addingTimeInterval(5)};if code==SQLITE_BUSY || code==SQLITE_LOCKED{Thread.sleep(forTimeInterval:0.01)}} while (code==SQLITE_OK || code==SQLITE_BUSY || code==SQLITE_LOCKED) && Date()<deadline
    let finish=sqlite3_backup_finish(job);guard code==SQLITE_DONE,finish==SQLITE_OK else{throw sqlError(target)}
}
private func publish(_ temp:URL,_ target:URL) throws {guard rename(temp.path,target.path)==0 else{throw BibleError.invalid(String(cString:strerror(errno)))}}
private func snapshot(_ source:OpaquePointer?,to target:URL) throws {
    let temp=target.deletingLastPathComponent().appendingPathComponent(".bibbia-\(UUID().uuidString).tmp")
    defer{try? FileManager.default.removeItem(at:temp)}
    guard FileManager.default.createFile(atPath:temp.path,contents:Data(),attributes:[.posixPermissions:0o600]) else{throw BibleError.invalid("Impossibile creare il file / Cannot create file")}
    var dest:OpaquePointer?;guard sqlite3_open(temp.path,&dest)==SQLITE_OK else{sqlite3_close(dest);throw BibleError.invalid("Impossibile aprire il backup / Cannot open backup")}
    do {try copyDatabase(source,dest);try exec(dest,"PRAGMA journal_mode=DELETE;PRAGMA application_id=\(backupID);PRAGMA user_version=1;")}
    catch{sqlite3_close(dest);throw error}
    sqlite3_close(dest);try publish(temp,target)
}
private func inspect(_ file:URL) throws -> (OpaquePointer,BackupSummary) {
    let size=(try FileManager.default.attributesOfItem(atPath:file.path)[.size] as? NSNumber)?.int64Value ?? 0
    guard size>=100 else{throw BibleError.invalid("Dimensione backup non valida / Invalid backup size")}
    var db:OpaquePointer?;guard sqlite3_open_v2(file.path,&db,SQLITE_OPEN_READONLY|SQLITE_OPEN_NOMUTEX,nil)==SQLITE_OK,let db else{sqlite3_close(db);throw BibleError.invalid("Backup non leggibile / Cannot read backup")}
    do {
        try exec(db,"PRAGMA trusted_schema=OFF;PRAGMA query_only=ON;BEGIN;")
        guard try values(db,"PRAGMA application_id").first?.first==String(backupID),try values(db,"PRAGMA user_version").first?.first=="1",try values(db,"PRAGMA quick_check").first?.first=="ok" else{throw BibleError.invalid("Backup Bibbia non valido o danneggiato / Invalid or damaged Bibbia backup")}
        let objects=try values(db,"SELECT type,name,COALESCE(sql,'') FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'")
        let tables=objects.filter{$0[0] != "index"}
        guard tables.count==7,tables.allSatisfy({row in row[0]=="table" && !row[2].lowercased().contains("virtual") && backupTables.contains(where:{$0.0==row[1]})}) else {
            // Explicit loop below also checks all table names and column order.
            throw BibleError.invalid("Schema backup non valido / Invalid backup schema")
        }
        for (table,columns) in backupTables {guard try values(db,"PRAGMA table_info(\(table))").map({$0[1]})==columns else{throw BibleError.invalid("Backup incompleto / Incomplete backup")}}
        for table in ["annotations","settings","editions","reading_plans"]{guard try values(db,"SELECT count(*) FROM \(table) WHERE NOT json_valid(payload)")[0][0]=="0" else{throw BibleError.invalid("Dati backup non validi / Invalid backup data")}}
        let validation=try String(contentsOf:BibleStore.bundledResources.appendingPathComponent("backup-validation.sql"),encoding:.utf8)
        guard try values(db,validation).isEmpty else{throw BibleError.invalid("Dati backup non validi / Invalid backup data")}
        func count(_ table:String) throws -> Int {Int(try values(db,"SELECT count(*) FROM \(table)")[0][0])!}
        return (db,try BackupSummary(annotations:count("annotations"),editions:count("editions"),plans:count("reading_plans"),groups:count("note_groups")))
    } catch {sqlite3_close(db);throw error}
}
extension Database {
    func applyPendingRestore() throws {
        let pending=directory.appendingPathComponent(pendingName)
        guard FileManager.default.fileExists(atPath:pending.path) else{return}
        let (source,_)=try inspect(pending);defer{sqlite3_close(source)}
        let safety=directory.appendingPathComponent("Bibbia-before-restore-\(UUID().uuidString).bibbia-backup")
        try snapshot(handle,to:safety);try copyDatabase(source,handle)
        try? FileManager.default.removeItem(at:directory.appendingPathComponent("reader.json"))
        try FileManager.default.removeItem(at:pending)
    }
    func checkDestination(_ file:URL) throws {
        let url=file.resolvingSymlinksInPath(),parent=url.deletingLastPathComponent().standardizedFileURL
        let reserved=["bible.sqlite3","bible.sqlite3-wal","bible.sqlite3-shm",pendingName,"reader.json"]
        guard !(parent==directory.resolvingSymlinksInPath().standardizedFileURL && reserved.contains(url.lastPathComponent)) else{throw BibleError.invalid("Scegli un file diverso dai dati dell’app / Choose another destination")}
    }
    func backup(to url:URL) throws {try checkDestination(url);try snapshot(handle,to:url)}
    func backupInfo(_ url:URL) throws -> BackupSummary {let(db,summary)=try inspect(url);sqlite3_close(db);return summary}
    func stageRestore(_ url:URL) throws -> BackupSummary {let(db,summary)=try inspect(url);defer{sqlite3_close(db)};try snapshot(db,to:directory.appendingPathComponent(pendingName));return summary}
}
extension BibleStore {
    public func createBackup(to url:URL) throws {try database.backup(to:url)}
    public func inspectBackup(_ url:URL) throws -> BackupSummary {try database.backupInfo(url)}
    public func restoreBackup(_ url:URL) throws -> BackupSummary {try database.stageRestore(url)}
    @discardableResult public func exportNotes(to file:URL,markdown:Bool) throws -> Int {
        try database.checkDestination(file)
        let temp=file.deletingLastPathComponent().appendingPathComponent(".bibbia-\(UUID().uuidString).tmp")
        guard FileManager.default.createFile(atPath:temp.path,contents:Data(),attributes:[.posixPermissions:0o600]) else{throw BibleError.invalid("Impossibile creare il file / Cannot create file")}
        defer{try? FileManager.default.removeItem(at:temp)}
        let out=try FileHandle(forWritingTo:temp);defer{try? out.close()}
        func write(_ s:String) throws {try out.write(contentsOf:Data(s.utf8))}
        func render(_ s:String)->String {markdown ? String(s.flatMap{"\\`*_{}[]<>()#+-.!|~".contains($0) ? [Character("\\"),$0]:[$0]}):s}
        try write("\(markdown ? "# ":"")Bibbia · Appunti\n\n")
        var count=0,cei=false
        try database.scan("SELECT a.edition,a.book,a.chapter,a.verse,a.payload,COALESCE(o.folder,''),COALESCE(o.tags,'[]') FROM annotations a LEFT JOIN note_organization o USING(edition,book,chapter,verse) ORDER BY a.edition,a.book,a.chapter,a.verse") {row in
            try Task.checkCancellation()
            let edition=row[0].string!,b=row[1].integer!,c=row[2].integer!,v=row[3].integer!
            let a=try JSONDecoder().decode(Annotation.self,from:Data(row[4].string!.utf8))
            let source=try corpus(edition),span=try source.chapter(b,c),text=try source.verse(b,c,v)
            let folder=row[5].string ?? "",tags=try JSONDecoder().decode([String].self,from:Data((row[6].string ?? "[]").utf8))
            try write("\(markdown ? "## ":"")\(render(source.books[b].name)) \(c+1):\(render(span.label(v))) · \(render(edition))\n\n")
            try write("\(markdown ? "> ":"")\(render(text))\n\n")
            if !a.note.isEmpty{try write(render(a.note)+"\n\n")}
            var labels=[String]()
            if a.bookmark{labels.append("Segnalibro / Bookmark")};if a.underline{labels.append("Sottolineato / Underlined")}
            if let color=a.highlight{labels.append("Evidenziazione / Highlight: "+color)}
            if let color=a.noteColor{labels.append("Colore nota / Note color: "+color)};if a.noteStyle=="sticky"{labels.append("Post-it")}
            if !folder.isEmpty{labels.append("Cartella / Folder: "+folder)};if !tags.isEmpty{labels.append("Etichette / Tags: "+tags.joined(separator:", "))}
            if !labels.isEmpty{try write(render(labels.joined(separator:" · "))+"\n\n")}
            for mark in a.ranges{try Marking.validate(text:text,start:mark.start,length:mark.length);let selected=(text as NSString).substring(with:NSRange(location:mark.start,length:mark.length));try write("\(markdown ? "- ":"")«\(render(selected))» · \(mark.underline ? "Sottolineato / Underlined ":"")\(mark.highlight ?? "")\n\n")}
            try write("---\n\n");count+=1;cei=cei || edition=="CEI 2008";return true
        }
        if count==0{try write("Nessun appunto salvato / No saved annotations.\n")}
        if cei{try write("CEI 2008 · © Fondazione di Religione Santi Francesco d’Assisi e Caterina da Siena. Uso formativo e didattico non commerciale. Fonte: BibbiaEDU.\n")}
        try out.synchronize();try out.close();try publish(temp,file);return count
    }
}
