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

struct ComparisonIndex: Sendable, Decodable {
    let aliases: [String: String]
    let psalms: [String: [[String]]]
    func identity(_ name: String) -> String {
        let key = name.lowercased().filter { $0.isLetter || $0.isNumber }
        return aliases[key] ?? "custom:\(key)"
    }
    func key(_ edition: String, _ code: String, _ chapters: Int, _ chapter: Int, _ label: String) -> String {
        if code == "Ps" {
            let files = ["CEI 2008":"cei2008", "Martini 1781 (cattolica)":"martini", "Diodati 1649":"diodati", "KJV":"kjv", "Raccolta estesa (83 sezioni)":"extended"]
            if let file = files[edition], let data = psalms[file], data.indices.contains(chapter),
               let verse = Int(label), data[chapter].indices.contains(verse-1) { return data[chapter][verse-1] }
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
            comparisonIndex = try JSONDecoder().decode(ComparisonIndex.self,
                from: Data(contentsOf: resources.appendingPathComponent("comparison.json")))
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
            for (c, candidate) in book.chapters.enumerated() {
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
