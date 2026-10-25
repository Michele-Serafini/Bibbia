import Foundation
#if canImport(Darwin)
import Darwin
#else
import Glibc
#endif

struct ImportedBook: Codable {
    var name: String
    var abbrev: String
    var chapters: [[String]]
    var labels: [[String]]?
}

final class Corpus {
    let books: [BookIndex]
    private let text: Data?
    private let offsets: Data?
    private let imported: [ImportedBook]?
    private let root: URL?
    private let file: String?
    private var lower: Data?
    private var lowerOffsets: Data?

    init(root: URL, file: String) throws {
        books = try JSONDecoder().decode([BookIndex].self, from: Data(contentsOf: root.appendingPathComponent("\(file)-catalog.json")))
        text = try Data(contentsOf: root.appendingPathComponent("\(file)-text.bin"), options: .mappedIfSafe)
        offsets = try Data(contentsOf: root.appendingPathComponent("\(file)-offsets.bin"), options: .mappedIfSafe)
        imported = nil; self.root = root; self.file = file
        guard let text, let offsets, offsets.count >= 4, offsets.count % 4 == 0,
              Self.at(offsets, offsets.count / 4 - 1) == text.count else {
            throw BibleError.invalid("Indice del testo danneggiato / Damaged corpus index")
        }
    }

    init(imported books: [ImportedBook]) {
        self.imported = books; text = nil; offsets = nil; root = nil; file = nil
        var first = 0
        self.books = books.map { book in
            let chapters = book.chapters.enumerated().map { ci, verses in
                defer { first += verses.count }
                return ChapterIndex(first: first, count: verses.count, labels: book.labels?[ci] ?? [])
            }
            return BookIndex(name: book.name, abbrev: book.abbrev, chapters: chapters)
        }
    }

    private static func at(_ data: Data, _ index: Int) -> Int {
        data.withUnsafeBytes { Int(UInt32(littleEndian: $0.loadUnaligned(fromByteOffset: index * 4, as: UInt32.self))) }
    }

    func chapter(_ book: Int, _ chapter: Int) throws -> ChapterIndex {
        guard books.indices.contains(book), books[book].chapters.indices.contains(chapter) else {
            throw BibleError.invalid("Capitolo non disponibile / Chapter unavailable")
        }
        return books[book].chapters[chapter]
    }

    func verse(_ book: Int, _ chapter: Int, _ verse: Int) throws -> String {
        let span = try self.chapter(book, chapter)
        guard verse >= 0, verse < span.count else { throw BibleError.invalid("Versetto non disponibile / Verse unavailable") }
        if let imported { return imported[book].chapters[chapter][verse] }
        guard let text, let offsets else { throw BibleError.invalid("Testo non disponibile") }
        let index = span.first + verse
        let start = Self.at(offsets, index), end = Self.at(offsets, index + 1)
        guard start <= end, end <= text.count else { throw BibleError.invalid("Indice danneggiato") }
        return text.withUnsafeBytes { String(decoding: $0[start..<end], as: UTF8.self) }
    }

    func search(edition: String, query: String) throws -> SearchResult {
        if lower == nil, let root, let file {
            lower = try Data(contentsOf: root.appendingPathComponent("\(file)-lower.bin"), options: .mappedIfSafe)
            lowerOffsets = try Data(contentsOf: root.appendingPathComponent("\(file)-lower-offsets.bin"), options: .mappedIfSafe)
            guard let lower, let lowerOffsets, lowerOffsets.count == offsets?.count,
                  Self.at(lowerOffsets, lowerOffsets.count / 4 - 1) == lower.count else {
                self.lower = nil; self.lowerOffsets = nil
                throw BibleError.invalid("Indice di ricerca danneggiato / Damaged search index")
            }
        }
        var hits: [Hit] = []
        let needle = Array(query.utf8)
        // Only lowercase bytes and four-byte offsets are touched during search;
        // strings are materialized only for the first 200 matching verses.
        func scan(_ matches: (Int, Int, Int, Int) -> Bool) throws -> SearchResult {
            for (bi, book) in books.enumerated() {
                for (ci, span) in book.chapters.enumerated() {
                    for vi in 0..<span.count {
                        if (span.first + vi) % 128 == 0 { try Task.checkCancellation() }
                        guard matches(bi, ci, vi, span.first + vi) else { continue }
                        if hits.count == 200 { return SearchResult(hits: hits, truncated: true) }
                        hits.append(Hit(position: Position(edition: edition, book: bi, chapter: ci, verse: vi),
                                        name: book.name, label: span.label(vi),
                                        text: try verse(bi, ci, vi), annotation: Annotation()))
                    }
                }
            }
            return SearchResult(hits: hits, truncated: false)
        }
        if let lower, let lowerOffsets {
            return try lower.withUnsafeBytes { bytes in
                try lowerOffsets.withUnsafeBytes { index in
                    try needle.withUnsafeBytes { pattern in
                        try scan { _, _, _, verse in
                            let start = Int(UInt32(littleEndian: index.loadUnaligned(fromByteOffset: verse * 4, as: UInt32.self)))
                            let end = Int(UInt32(littleEndian: index.loadUnaligned(fromByteOffset: (verse + 1) * 4, as: UInt32.self)))
                            guard end >= start, end <= bytes.count, end - start >= pattern.count else { return false }
                            return memmem(bytes.baseAddress!.advanced(by: start), end - start,
                                          pattern.baseAddress!, pattern.count) != nil
                        }
                    }
                }
            }
        }
        return try scan { bi, ci, vi, _ in imported?[bi].chapters[ci][vi].lowercased().contains(query) == true }
    }
}
