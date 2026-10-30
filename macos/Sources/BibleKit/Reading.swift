import Foundation

public struct ReadingPlanSummary: Sendable, Identifiable { public let id: Int; public let title: String; public let edition: String }
struct StoredPlan: Codable { var title: String; var edition: String; var start: String; var daily: Int; var book: Int?; var resume: Position }
public struct ReadingDay: Sendable { public let offset: Int; public let total: Int; public let done: Int }
public struct ReadingChapter: Sendable, Identifiable { public let index: Int; public let book: Int; public let chapter: Int; public let name: String; public let complete: Bool; public var id: Int { index } }
public struct ReadingDetail: Sendable { public let id: Int; public let title: String; public let edition: String; public let start: String; public let daily: Int; public let total: Int; public let completed: Int; public let duration: Int; public let days: [ReadingDay]; public let chapters: [ReadingChapter]; public let resume: Position }
public enum ReadingDates {
    public static func key(_ date: Date) -> String { let c = Calendar.current.dateComponents([.year,.month,.day], from: date); return String(format:"%04d-%02d-%02d",c.year!,c.month!,c.day!) }
    static func day(_ s: String) throws -> Int {
        let b=Array(s.utf8)
        guard b.count==10,b[4]==45,b[7]==45,b.enumerated().allSatisfy({ $0.offset==4 || $0.offset==7 || (48...57).contains($0.element) }),let yy=Int(s.prefix(4)),let m=Int(s.dropFirst(5).prefix(2)),let d=Int(s.suffix(2)),(1900...2199).contains(yy),(1...12).contains(m) else {throw BibleError.invalid("Data non valida / Invalid date")}
        let maxDay=m==2 ? (yy%4==0 && (yy%100 != 0 || yy%400==0) ? 29:28):([4,6,9,11].contains(m) ? 30:31)
        guard (1...maxDay).contains(d) else {throw BibleError.invalid("Data non valida / Invalid date")}
        let y=yy-(m<=2 ? 1:0),era=y/400,yr=y-era*400,mm=m+(m>2 ? -3:9)
        return era*146097+yr*365+yr/4-yr/100+(153*mm+2)/5+d-1
    }
}
extension BibleStore {
    public func readingPlans() throws -> [ReadingPlanSummary] {
        try database.rows("SELECT id,payload FROM reading_plans ORDER BY id DESC LIMIT 24").map { row in
            let p=try JSONDecoder().decode(StoredPlan.self,from:Data(row[1].string!.utf8));return ReadingPlanSummary(id:row[0].integer!,title:p.title,edition:p.edition)
        }
    }
    @discardableResult public func createReadingPlan(title: String, edition: String, start: String, daily: Int, book: Int?) throws -> Int {
        let title=title.trimmingCharacters(in:.whitespacesAndNewlines)
        guard !title.isEmpty,title.utf8.count<=160,(1...12).contains(daily),(try readingPlans()).count<24 else {throw BibleError.invalid("Piano non valido / Invalid plan")}
        _ = try ReadingDates.day(start);let source=try corpus(edition)
        if let book {guard source.books.indices.contains(book) else {throw BibleError.invalid("Libro non disponibile / Book unavailable")}}
        let p=StoredPlan(title:title,edition:edition,start:start,daily:daily,book:book,resume:Position(edition:edition,book:book ?? 0))
        _ = try database.rows("INSERT INTO reading_plans(payload) VALUES(?)",[.text(String(decoding:try JSONEncoder().encode(p),as:UTF8.self))])
        return try database.rows("SELECT last_insert_rowid()")[0][0].integer!
    }
    func readingPlan(_ id:Int) throws -> StoredPlan {
        guard let text=try database.rows("SELECT payload FROM reading_plans WHERE id=?",[.integer(id)]).first?.first?.string else {throw BibleError.invalid("Piano non disponibile / Plan unavailable")}
        return try JSONDecoder().decode(StoredPlan.self,from:Data(text.utf8))
    }
    public func deleteReadingPlan(_ id:Int) throws { try database.transaction { _ = try database.rows("DELETE FROM reading_plans WHERE id=?",[.integer(id)]); _ = try database.rows("DELETE FROM settings WHERE key='activeReadingPlan' AND CAST(payload AS INTEGER)=?",[.integer(id)]) } }
    public func resumeReadingPlan(_ id:Int) throws -> Position { let p=try readingPlan(id);try database.set("activeReadingPlan",id);return p.resume }
    public func activateReadingPlan(_ id:Int) throws { _ = try readingPlan(id);try database.set("activeReadingPlan",id) }
    func trackReading(_ position:Position) throws {
        let value=String(decoding:try JSONEncoder().encode(position),as:UTF8.self)
        _ = try database.rows("UPDATE reading_plans SET payload=json_set(payload,'$.resume',json(?1)) WHERE id=COALESCE((SELECT CAST(payload AS INTEGER) FROM settings WHERE key='activeReadingPlan'),-1) AND json_extract(payload,'$.edition')=?2 AND (json_extract(payload,'$.book') IS NULL OR json_extract(payload,'$.book')=?3) AND (json_extract(payload,'$.resume.book')<>json_extract(?1,'$.book') OR json_extract(payload,'$.resume.chapter')<>json_extract(?1,'$.chapter') OR json_extract(payload,'$.resume.verse')<>json_extract(?1,'$.verse'))",[.text(value),.text(position.edition),.integer(position.book)])
    }
    func readingChapters(_ p:StoredPlan) throws -> [(Int,Int,String)] {
        let source=try corpus(p.edition)
        return source.books.enumerated().filter {p.book==nil || p.book==$0.offset}.flatMap {b,book in book.chapters.indices.map{(b,$0,book.name)} }
    }
    public func completeReadingChapter(_ id:Int,index:Int,complete:Bool) throws {
        let p=try readingPlan(id),chapters=try readingChapters(p)
        guard chapters.indices.contains(index) else {throw BibleError.invalid("Capitolo non disponibile / Chapter unavailable")}
        try database.transaction {
            if complete {_ = try database.rows("INSERT OR IGNORE INTO reading_progress VALUES(?,?)",[.integer(id),.integer(index)])}
            else {_ = try database.rows("DELETE FROM reading_progress WHERE plan_id=? AND chapter_index=?",[.integer(id),.integer(index)])}
            if complete {
                let done=Set(try database.rows("SELECT chapter_index FROM reading_progress WHERE plan_id=?",[.integer(id)]).compactMap{$0[0].integer})
                if let next=chapters.indices.first(where:{!done.contains($0)}) {
                    let (b,c,_)=chapters[next],resume=Position(edition:p.edition,book:b,chapter:c)
                    _ = try database.rows("UPDATE reading_plans SET payload=json_set(payload,'$.resume',json(?)) WHERE id=?",[.text(String(decoding:try JSONEncoder().encode(resume),as:UTF8.self)),.integer(id)])
                }
            }
        }
    }
    public func readingDetail(_ id:Int,from:String,selected:String) throws -> ReadingDetail {
        let p=try readingPlan(id),chapters=try readingChapters(p),start=try ReadingDates.day(p.start),from=try ReadingDates.day(from),selected=try ReadingDates.day(selected)
        let done=Set(try database.rows("SELECT chapter_index FROM reading_progress WHERE plan_id=?",[.integer(id)]).compactMap{$0[0].integer})
        let days=(0..<42).map { offset in
            let n=from+offset-start,first=max(0,n)*p.daily,end=min(first+p.daily,chapters.count)
            let total=n<0 || first>=chapters.count ? 0:end-first
            return ReadingDay(offset:offset,total:total,done:total==0 ? 0:(first..<end).filter{done.contains($0)}.count)
        }
        let n=selected-start,first=max(0,n)*p.daily,end=min(first+p.daily,chapters.count)
        let items:[ReadingChapter]=n<0 || first>=chapters.count ? []:(first..<end).map {i in let(b,c,name)=chapters[i];return ReadingChapter(index:i,book:b,chapter:c,name:name,complete:done.contains(i))}
        return ReadingDetail(id:id,title:p.title,edition:p.edition,start:p.start,daily:p.daily,total:chapters.count,completed:done.count,duration:(chapters.count+p.daily-1)/p.daily,days:days,chapters:items,resume:p.resume)
    }
    public func writeNote(_ text:String,color:String?,style:String,at position:Position) throws -> [Verse] {
        guard text.utf8.count<=32768, color==nil || Marking.colors.contains(color!),["plain","sticky"].contains(style) else {throw BibleError.invalid("Nota o stile non valido / Invalid note or style")}
        _ = try corpus(position.edition).verse(position.book,position.chapter,position.verse)
        var a=try annotation(position);a.note=text;a.noteColor=text.isEmpty ? nil:color;a.noteStyle=text.isEmpty ? nil:style
        try save(a,at:position);return try chapter(position)
    }
}
