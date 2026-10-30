import Foundation

private func calendarDate(_ z:Int)->String {
    let era=z/146097,doe=z-era*146097,yoe=(doe-doe/1460+doe/36524-doe/146096)/365
    var y=yoe+era*400
    let doy=doe-(365*yoe+yoe/4-yoe/100),mp=(5*doy+2)/153,d=doy-(153*mp+2)/5+1,m=mp+(mp<10 ? 3:-9)
    y += m<=2 ? 1:0
    return String(format:"%04d%02d%02d",y,m,d)
}
private func calendarText(_ value:String)->String {
    var result=""
    for c in value.replacingOccurrences(of:"\r\n",with:"\n").replacingOccurrences(of:"\r",with:"\n").unicodeScalars {
        switch c.value {
        case 92:result+="\\\\"
        case 59:result+="\\;"
        case 44:result+="\\,"
        case 10:result+="\\n"
        default:if c.value>=32 && !(127...159).contains(c.value){result.unicodeScalars.append(c)}
        }
    }
    return result
}
private struct CalendarWriter {
    let file:FileHandle
    var buffer=Data()
    mutating func line(_ value:String) throws {
        var count=0
        for c in value.unicodeScalars {
            let bytes=String(c).utf8
            if count+bytes.count>75 {buffer.append(contentsOf:[13,10,32]);count=1}
            buffer.append(contentsOf:bytes);count+=bytes.count
        }
        buffer.append(contentsOf:[13,10])
        if buffer.count>=8192 {try file.write(contentsOf:buffer);buffer.removeAll(keepingCapacity:true)}
    }
    mutating func finish() throws {try file.write(contentsOf:buffer)}
}
extension BibleStore {
    @discardableResult public func exportReadingCalendar(_ id:Int,to file:URL,language:String) throws -> Int {
        let p=try readingPlan(id),chapters=try readingChapters(p),start=try ReadingDates.day(p.start)
        let done=Set(try database.rows("SELECT chapter_index FROM reading_progress WHERE plan_id=?",[.integer(id)]).compactMap{$0[0].integer})
        let identity="\(id)\n\(p.edition)\n\(p.start)\n\(p.daily)\n\(p.book ?? -1)\n\(p.title)"
        let hash=identity.utf8.reduce(UInt64(0xcbf29ce484222325)){($0^UInt64($1)) &* 0x100000001b3}
        let seconds=Int(Date().timeIntervalSince1970),sec=seconds%86400
        let stamp=String(format:"%@T%02d%02d%02dZ",calendarDate(try ReadingDates.day("1970-01-01")+seconds/86400),sec/3600,sec/60%60,sec%60)
        try writeDocument(to:file){handle in
            var out=CalendarWriter(file:handle)
            for header in ["BEGIN:VCALENDAR","VERSION:2.0","PRODID:-//Bibbia//Reading Plans//IT","CALSCALE:GREGORIAN"]{try out.line(header)}
            try out.line("X-WR-CALNAME:"+calendarText(p.title))
            for first in stride(from:0,to:chapters.count,by:p.daily) {
                let end=min(first+p.daily,chapters.count),day=first/p.daily
                let references=chapters[first..<end].map{"\($0.2) \($0.1+1)"}.joined(separator:" · ")
                let count=(first..<end).filter{done.contains($0)}.count
                let description=language=="en" ? "\(p.edition)\nReading: \(references)\nCompleted: \(count)/\(end-first)\nOpen Bibbia → Reading plans → Resume." : "\(p.edition)\nLetture: \(references)\nCompletati: \(count)/\(end-first)\nApri Bibbia → Piani di lettura → Riprendi."
                for property in ["BEGIN:VEVENT",String(format:"UID:bibbia-%016llx-%d@bibbia.local",hash,day),"DTSTAMP:"+stamp,"DTSTART;VALUE=DATE:"+calendarDate(start+day),"DTEND;VALUE=DATE:"+calendarDate(start+day+1),"SUMMARY:"+calendarText(p.title+" · "+references),"DESCRIPTION:"+calendarText(description),"TRANSP:TRANSPARENT","STATUS:CONFIRMED","END:VEVENT"]{try out.line(property)}
            }
            try out.line("END:VCALENDAR");try out.finish()
        }
        return (chapters.count+p.daily-1)/p.daily
    }
}
