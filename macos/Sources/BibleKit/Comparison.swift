import Foundation

public struct ComparedVerse: Codable, Sendable {
    public let edition: String
    public let book, chapter, verse: Int
    public let label, reference, text: String
    public var position: Position { Position(edition: edition, book: book, chapter: chapter, verse: verse) }
}
public struct ComparisonRow: Codable, Sendable, Identifiable {
    public let left, right: [ComparedVerse]
    public var id: Int { left.first?.verse ?? -1 }
}
public struct Comparison: Codable, Sendable {
    public let edition, other, status: String
    public let rows: [ComparisonRow]
}

struct ComparisonIndex: Sendable {
    private struct Names:Decodable {let aliases:[String:String]}
    let aliases: [String: String]
    private let psalms:Data
    init(resources:URL) throws {
        aliases=try JSONDecoder().decode(Names.self,from:Data(contentsOf:resources.appendingPathComponent("comparison-books.json"))).aliases
        psalms=try Data(contentsOf:resources.appendingPathComponent("comparison-psalms.bin"),options:.mappedIfSafe)
        guard psalms.count>=1816,psalms.prefix(4)==Data("BPC1".utf8) else{throw BibleError.invalid("Damaged comparison index")}
    }
    func group(_ edition:String,_ chapter:Int,_ verse:Int)->UInt16? {
        let profile:Int
        switch edition {case "CEI 2008":profile=0;case "Martini 1781 (cattolica)":profile=1;case "Diodati 1649","KJV","Raccolta estesa (83 sezioni)":profile=2;default:return nil}
        guard (0..<150).contains(chapter),verse>0 else{return nil}
        return psalms.withUnsafeBytes { bytes in
            let start=Int(UInt32(littleEndian:bytes.loadUnaligned(fromByteOffset:4+(profile*151+chapter)*4,as:UInt32.self)))
            let end=Int(UInt32(littleEndian:bytes.loadUnaligned(fromByteOffset:4+(profile*151+chapter+1)*4,as:UInt32.self)))
            guard verse<=end-start else{return nil}
            let at=1816+(start+verse-1)*2
            guard at+2<=bytes.count else{return nil}
            let value=UInt16(littleEndian:bytes.loadUnaligned(fromByteOffset:at,as:UInt16.self))
            return value==0 ? nil:value
        }
    }
    func identity(_ name: String) -> String {
        let key = name.lowercased().filter { $0.isLetter || $0.isNumber }
        return aliases[key] ?? "custom:\(key)"
    }
    func key(_ edition: String, _ code: String, _ chapters: Int, _ chapter: Int, _ label: String) -> String {
        if code == "Ps" {
            if let verse=Int(label),let id=group(edition,chapter,verse){return "Ps:\(id)"}
            return "unmapped:\(edition):\(chapter):\(label)"
        }
        var c = chapter+1, v = label
        if code == "Esth", edition == "CEI 2008" {
            guard label.hasPrefix("E") else { return "unmapped:CEI:\(chapter):\(label)" }
            v = String(label.dropFirst())
        }
        if code == "Joel", chapters == 4 {
            if c == 3, let n = Int(label) { c = 2; v = String(n+27) }
            else if c == 4 { c = 3 }
        }
        if code == "Mal", chapters == 3, c == 3, let n = Int(v), n > 18 { c = 4; v = String(n-18) }
        return "\(code):\(c):\(v)"
    }
}

extension BibleStore {
    public func compare(_ position: Position, other: String) throws -> Comparison {
        if comparisonIndex == nil {
            comparisonIndex = try ComparisonIndex(resources:resources)
        }
        let index = comparisonIndex!
        let primary = try corpus(position.edition)
        let left = try primary.chapter(position.book, position.chapter)
        let source = primary.books[position.book]
        let code = index.identity(source.name)
        let target = try corpus(other)
        let targetBook = target.books.firstIndex { index.identity($0.name) == code }
        func key(_ edition: String, _ book: BookIndex, _ chapter: Int, _ label: String) -> String {
            index.key(edition,code,book.chapters.count,chapter,label)
        }
        let wanted = Set((0..<left.count).map { key(position.edition,source,position.chapter,left.label($0)) })
        var right = [String: [ComparedVerse]](), differing = false
        if let b = targetBook {
            let book = target.books[b]
            let chapters:[Int]
            if code=="Ps",position.edition != other {
                let ids=Set((0..<left.count).compactMap {Int(left.label($0)).flatMap {index.group(position.edition,position.chapter,$0)}})
                chapters=book.chapters.indices.filter {c in (1...book.chapters[c].count).contains {v in index.group(other,c,v).map {ids.contains($0)} ?? false}}
            }else if ["Ps","Joel","Mal","Esth"].contains(code){chapters=Array(book.chapters.indices)}
            else{chapters=book.chapters.indices.contains(position.chapter) ? [position.chapter]:[]}
            for c in chapters {
                let candidate=book.chapters[c]
                guard (0..<candidate.count).contains(where: { wanted.contains(key(other,book,c,candidate.label($0))) }) else { continue }
                if !["Ps","Joel","Mal","Esth"].contains(code), c == position.chapter,
                   candidate.label(candidate.count-1) != left.label(left.count-1) { differing = true; continue }
                for v in 0..<candidate.count {
                    let k = key(other,book,c,candidate.label(v))
                    if wanted.contains(k) {
                        right[k,default:[]].append(ComparedVerse(edition: other, book: b, chapter: c, verse: v,
                            label: candidate.label(v), reference: "\(book.name) \(c+1):\(candidate.label(v))", text: try target.verse(b,c,v)))
                    }
                }
            }
        }
        var groups = [String:Int](), lefts = [[ComparedVerse]](), rights = [[ComparedVerse]]()
        for v in 0..<left.count {
            let k = key(position.edition,source,position.chapter,left.label(v))
            let cell = ComparedVerse(edition: position.edition, book: position.book, chapter: position.chapter, verse: v,
                label: left.label(v), reference: "\(source.name) \(position.chapter+1):\(left.label(v))", text: try primary.verse(position.book,position.chapter,v))
            if let row = groups[k] { lefts[row].append(cell) }
            else { groups[k] = lefts.count; lefts.append([cell]); rights.append(right[k] ?? []) }
        }
        let missing = rights.filter(\.isEmpty).count
        let status = targetBook == nil ? "missingBook" : differing || missing == lefts.count ? "differentNumbering" : missing > 0 ? "partial" : "aligned"
        return Comparison(edition: position.edition, other: other, status: status,
                          rows: zip(lefts,rights).map { ComparisonRow(left: $0.0, right: $0.1) })
    }
}
