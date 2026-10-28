import Foundation

public enum BibleError: Error, LocalizedError {
    case invalid(String)
    public var errorDescription: String? {
        switch self { case .invalid(let message): return message }
    }
}

public struct Edition: Codable, Identifiable, Sendable, Hashable {
    public let id: String
    public let file: String
    public let language: String
    public let canon: String
    public let books: Int
}

public struct ChapterIndex: Codable, Sendable {
    public let first: Int
    public let count: Int
    public let labels: [String]
    public func label(_ verse: Int) -> String {
        labels.indices.contains(verse) ? labels[verse] : String(verse + 1)
    }
}

public struct BookIndex: Codable, Sendable, Identifiable {
    public let name: String
    public let abbrev: String
    public let chapters: [ChapterIndex]
    public var id: String { abbrev }
}

public struct Position: Codable, Sendable, Equatable, Hashable {
    public var edition: String
    public var book: Int
    public var chapter: Int
    public var verse: Int
    public init(edition: String = "Martini 1781 (cattolica)", book: Int = 0,
                chapter: Int = 0, verse: Int = 0) {
        self.edition = edition; self.book = book; self.chapter = chapter; self.verse = verse
    }
    enum CodingKeys: String, CodingKey { case edition, book, chapter, verse }
    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        edition = try c.decode(String.self, forKey: .edition)
        book = try c.decode(Int.self, forKey: .book)
        chapter = try c.decode(Int.self, forKey: .chapter)
        verse = try c.decodeIfPresent(Int.self, forKey: .verse) ?? 0
    }
}

public struct TextMark: Codable, Sendable, Equatable {
    public var start: Int
    public var length: Int
    public var highlight: String?
    public var underline: Bool
    public init(start: Int, length: Int, highlight: String? = nil, underline: Bool = false) {
        self.start = start; self.length = length; self.highlight = highlight; self.underline = underline
    }
}

public struct Annotation: Codable, Sendable, Equatable {
    public var highlight: String?
    public var underline = false
    public var bookmark = false
    public var note = ""
    public var ranges: [TextMark] = []
    public init() {}
    public var isEmpty: Bool {
        highlight == nil && !underline && !bookmark && note.isEmpty && ranges.isEmpty
    }
    // Old backups written before phrase annotations have no `ranges` field.
    enum CodingKeys: String, CodingKey { case highlight, underline, bookmark, note, ranges }
    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        highlight = try c.decodeIfPresent(String.self, forKey: .highlight)
        underline = try c.decodeIfPresent(Bool.self, forKey: .underline) ?? false
        bookmark = try c.decodeIfPresent(Bool.self, forKey: .bookmark) ?? false
        note = try c.decodeIfPresent(String.self, forKey: .note) ?? ""
        ranges = try c.decodeIfPresent([TextMark].self, forKey: .ranges) ?? []
    }
}

public struct Verse: Sendable, Identifiable {
    public let id: Int
    public let label: String
    public let text: String
    public var annotation: Annotation
}

public struct Hit: Sendable, Identifiable {
    public let position: Position
    public let name: String
    public let label: String
    public let text: String
    public let annotation: Annotation
    public var id: Position { position }
    public var reference: String { "\(name) \(position.chapter + 1):\(label)" }
}

public struct SearchResult: Sendable {
    public let hits: [Hit]
    public let truncated: Bool
}

public struct Preferences: Codable, Sendable, Equatable {
    public var language = "it"
    public var theme = "Default"
    public var fontSize = 22
    public var bibleLanguage = "all"
    public var comparisonEdition = "CEI 2008"
    public init() {}
}

public struct Snapshot: Sendable {
    public let editions: [Edition]
    public let position: Position
    public let preferences: Preferences
    public let books: [BookIndex]
    public let verses: [Verse]
}

public struct SelectedSpan: Sendable, Equatable {
    public let verse: Int
    public let start: Int
    public let length: Int
    public init(verse: Int, start: Int, length: Int) {
        self.verse = verse; self.start = start; self.length = length
    }
}

/// UTF-16 offsets are shared with AppKit, WinUI and the Linux database schema.
public struct ReadingLayout: Sendable {
    public let text: String
    public let spans: [NSRange]
    public init(verses: [Verse]) {
        var text = ""
        var spans: [NSRange] = []
        var offset = 0
        for verse in verses {
            let prefix = "\(verse.annotation.bookmark ? "★" : " ") \(verse.label)\t"
            text += prefix
            offset += prefix.utf16.count
            let length = verse.text.utf16.count
            spans.append(NSRange(location: offset, length: length))
            text += verse.text + "\n"
            offset += length + 1
        }
        self.text = text; self.spans = spans
    }
    public func selection(_ range: NSRange) -> [SelectedSpan] {
        guard range.location != NSNotFound, range.length > 0,
              range.location <= text.utf16.count,
              range.length <= text.utf16.count - range.location else { return [] }
        return spans.enumerated().compactMap { verse, span in
            let part = NSIntersectionRange(range, span)
            guard part.length > 0 else { return nil }
            return SelectedSpan(verse: verse, start: part.location - span.location, length: part.length)
        }
    }
}
