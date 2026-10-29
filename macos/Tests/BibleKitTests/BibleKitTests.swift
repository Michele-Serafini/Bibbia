import Foundation
import Testing
@testable import BibleKit

private struct Fixture {
    let directory: URL
    let store: BibleStore
    init() throws {
        directory = FileManager.default.temporaryDirectory.appendingPathComponent("bibbia-swift-\(UUID().uuidString)")
        store = try BibleStore(directory: directory)
    }
    func cleanup() { try? FileManager.default.removeItem(at: directory) }
    func write(_ text: String) throws -> URL {
        let file = directory.appendingPathComponent("import-\(UUID().uuidString).json")
        try Data(text.utf8).write(to: file)
        return file
    }
}

@Test func allEditionCatalogsAndFirstChapters() async throws {
    let f = try Fixture(); defer { f.cleanup() }
    let editions = await f.store.editions()
    #expect(editions.count == 5)
    for edition in editions {
        let books = try await f.store.books(edition: edition.id)
        #expect(books.count == edition.books)
        let chapter = try await f.store.chapter(Position(edition: edition.id))
        #expect(!chapter.isEmpty)
        #expect(chapter.allSatisfy { !$0.text.isEmpty })
    }
}

@Test func initialPositionAndPreferences() async throws {
    let f = try Fixture(); defer { f.cleanup() }
    let snapshot = try await f.store.bootstrap()
    #expect(snapshot.position == Position())
    #expect(snapshot.preferences == Preferences())
    #expect(snapshot.books.count == 73)
    #expect(snapshot.verses[0].text.contains("principio"))
}

@Test func ceiCatholicCatalogAndSourceLabels() async throws {
    let f = try Fixture(); defer { f.cleanup() }
    let books = try await f.store.books(edition: "CEI 2008")
    #expect(books.count == 73)
    #expect(books.reduce(0) { $0 + $1.chapters.count } == 1328)
    #expect(books.contains { $0.name.lowercased().contains("tobia") })
    let esther = try #require(books.first { $0.name.lowercased().contains("ester") })
    #expect(esther.chapters.flatMap(\.labels).contains { $0.hasPrefix("G") })
    #expect(esther.chapters.flatMap(\.labels).contains { $0.hasPrefix("E") })
}

@Test func extendedCanonContainsEnochAndJubilees() async throws {
    let f = try Fixture(); defer { f.cleanup() }
    let books = try await f.store.books(edition: "Raccolta estesa (83 sezioni)")
    #expect(books.count == 83)
    #expect(books.contains { $0.name.contains("Enoch") && $0.chapters.count == 108 })
    #expect(books.contains { $0.name.contains("Jubilees") && $0.chapters.count == 50 })
}

@Test func caseInsensitiveSearchAndLimit() async throws {
    let f = try Fixture(); defer { f.cleanup() }
    let lower = try await f.store.search(edition: "CEI 2008", query: " dio ")
    let upper = try await f.store.search(edition: "CEI 2008", query: "DIO")
    #expect(lower.hits.count == 200 && lower.truncated)
    #expect(lower.hits.map(\.position) == upper.hits.map(\.position))
    #expect(lower.hits.first?.position == Position(edition: "CEI 2008"))
}

@Test func ceiJohn316SearchPreservesOfficialText() async throws {
    let f = try Fixture(); defer { f.cleanup() }
    let result = try await f.store.search(edition: "CEI 2008", query: "HA TANTO AMATO IL MONDO")
    let verse = try #require(result.hits.first { $0.name == "Giovanni" && $0.position.chapter == 2 && $0.label == "16" })
    #expect(verse.text.hasPrefix("Dio infatti ha tanto amato il mondo"))
}

@Test func searchDoesNotCrossVerseBoundaries() async throws {
    let f = try Fixture(); defer { f.cleanup() }
    let file = try f.write("""
        [{"name":"Test","abbrev":"T","chapters":[["abc","def","abCDef"]]}]
        """)
    _ = try await f.store.importEdition(file: file, name: "Boundary")
    let result = try await f.store.search(edition: "Boundary", query: "cde")
    #expect(result.hits.map(\.position.verse) == [2])
    #expect(!result.truncated)
    await #expect(throws: BibleError.self) { try await f.store.search(edition: "KJV", query: "x") }
}

@Test func utf16SelectionExcludesNumbersAndSplitsVerses() throws {
    let verses = [Verse(id: 0, label: "1a", text: "A😀B", annotation: Annotation()),
                  Verse(id: 1, label: "2", text: "second", annotation: Annotation())]
    let layout = ReadingLayout(verses: verses)
    #expect(layout.selection(NSRange(location: 0, length: layout.text.utf16.count)) ==
            [SelectedSpan(verse: 0, start: 0, length: 4), SelectedSpan(verse: 1, start: 0, length: 6)])
    let prefixOnly = layout.selection(NSRange(location: 0, length: layout.spans[0].location))
    #expect(prefixOnly.isEmpty)
    try Marking.validate(text: "A😀B", start: 1, length: 2)
    #expect(throws: BibleError.self) { try Marking.validate(text: "A😀B", start: 1, length: 1) }
    #expect(throws: BibleError.self) { try Marking.validate(text: "A😀B", start: 2, length: 1) }
    try Marking.validate(text: "e\u{301}", start: 1, length: 1)
}

@Test func overlappingMarksMergeAndClear() throws {
    let text = "abcdefghij"
    var annotation = try Marking.apply(text: text, annotation: Annotation(), start: 1, length: 4, action: "yellow")
    annotation = try Marking.apply(text: text, annotation: annotation, start: 3, length: 4, action: "underline")
    #expect(annotation.ranges == [TextMark(start: 1, length: 2, highlight: "yellow"),
        TextMark(start: 3, length: 2, highlight: "yellow", underline: true), TextMark(start: 5, length: 2, underline: true)])
    annotation = try Marking.apply(text: text, annotation: annotation, start: 0, length: 10, action: "clear")
    #expect(annotation.isEmpty)
}

@Test func legacyWholeVerseMarksPreservedOutsideClearedPhrase() throws {
    var annotation = Annotation(); annotation.underline = true; annotation.highlight = "green"
    let result = try Marking.apply(text: "abcdef", annotation: annotation, start: 2, length: 2, action: "clear")
    #expect(!result.underline && result.highlight == nil)
    #expect(result.ranges == [TextMark(start: 0, length: 2, highlight: "green", underline: true),
                             TextMark(start: 4, length: 2, highlight: "green", underline: true)])
}

@Test func multiVerseTransactionRollsBackOnInvalidRange() async throws {
    let f = try Fixture(); defer { f.cleanup() }
    await #expect(throws: BibleError.self) {
        try await f.store.mark(Position(), selection: [SelectedSpan(verse: 0, start: 0, length: 3),
                                                      SelectedSpan(verse: 1, start: 99999, length: 1)], action: "yellow")
    }
    let verses = try await f.store.chapter(Position())
    #expect(verses[0].annotation.isEmpty)
}

@Test func noteBookmarkSelectionAndSettingsSurviveReopen() async throws {
    let f = try Fixture(); defer { f.cleanup() }
    let position = Position(edition: "CEI 2008", book: 1, chapter: 2, verse: 3)
    _ = try await f.store.mark(position, selection: [SelectedSpan(verse: 3, start: 0, length: 4)], action: "underline")
    _ = try await f.store.setNote("Appunto con accenti e 😀", at: position)
    _ = try await f.store.toggleBookmark(at: position)
    try await f.store.savePosition(position)
    var preferences = Preferences(); preferences.fontSize = 28; preferences.language = "en"; preferences.theme = "Dark"
    try await f.store.savePreferences(preferences)
    let reopened = try BibleStore(directory: f.directory)
    let snapshot = try await reopened.bootstrap()
    #expect(snapshot.position == position)
    #expect(snapshot.preferences == preferences)
    #expect(snapshot.verses[3].annotation.note == "Appunto con accenti e 😀")
    #expect(snapshot.verses[3].annotation.bookmark)
    #expect(snapshot.verses[3].annotation.ranges == [TextMark(start: 0, length: 4, underline: true)])
}

@Test func savedPaginationHasStableOrdering() async throws {
    let f = try Fixture(); defer { f.cleanup() }
    for verse in [5, 1, 3] {
        _ = try await f.store.toggleBookmark(at: Position(verse: verse))
    }
    let first = try await f.store.saved(edition: Position().edition, limit: 2)
    let second = try await f.store.saved(edition: Position().edition, limit: 2, offset: 2)
    #expect(first.map(\.position.verse) == [1, 3])
    #expect(second.map(\.position.verse) == [5])
    await #expect(throws: BibleError.self) { try await f.store.saved(edition: "KJV", limit: 201) }
    await #expect(throws: BibleError.self) { try await f.store.saved(edition: "KJV", offset: -1) }
}

@Test func clearingLastAnnotationRemovesSavedRow() async throws {
    let f = try Fixture(); defer { f.cleanup() }
    _ = try await f.store.toggleBookmark(at: Position())
    _ = try await f.store.toggleBookmark(at: Position())
    #expect(try await f.store.saved(edition: Position().edition).isEmpty)
}

@Test func importedLabelsAndUnicodeAreKept() async throws {
    let f = try Fixture(); defer { f.cleanup() }
    let file = try f.write("""
        [{"name":"Prova","abbrev":"Pr","chapters":[["α😀β","Ecco una parola"]],"labels":[["1a","2–3"]]}]
        """)
    _ = try await f.store.importEdition(file: file, name: "La mia versione")
    let verses = try await f.store.chapter(Position(edition: "La mia versione"))
    #expect(verses.map(\.label) == ["1a", "2–3"])
    #expect(verses[0].text == "α😀β")
    await #expect(throws: BibleError.self) { try await f.store.importEdition(file: file, name: "La mia versione") }
    await #expect(throws: BibleError.self) { try await f.store.importEdition(file: file, name: "CEI 2008") }
}

@Test func invalidImportIsNotInserted() async throws {
    let f = try Fixture(); defer { f.cleanup() }
    for json in ["[]", """
        [{"name":"Test","abbrev":"T","chapters":[["a","b"]],"labels":[["1","1"]]}]
        """, """
        [{"name":"Test","abbrev":"T","chapters":[[" "]]}]
        """] {
        let file = try f.write(json)
        await #expect(throws: (any Error).self) { try await f.store.importEdition(file: file, name: "Invalid") }
    }
    #expect(await f.store.editions().count == 5)
}

@Test func oversizedNoteAndInvalidPositionAreRejected() async throws {
    let f = try Fixture(); defer { f.cleanup() }
    await #expect(throws: BibleError.self) { try await f.store.setNote(String(repeating: "é", count: 17000), at: Position()) }
    await #expect(throws: BibleError.self) { try await f.store.savePosition(Position(book: -1)) }
    await #expect(throws: BibleError.self) { try await f.store.savePosition(Position(verse: 9999)) }
}

@Test func readsWindowsLinuxDatabaseAndMigratesOldImportTable() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: directory) }
    do {
        let db = try Database(directory: directory)
        try db.executeBatch("DROP TABLE editions; CREATE TABLE editions(id TEXT PRIMARY KEY,payload TEXT NOT NULL)")
        _ = try db.rows("INSERT INTO editions VALUES (?,?)", [.text("Legacy"), .text("""
            [{"name":"Legacy","abbrev":"L","chapters":[["old text"]]}]
            """)])
        _ = try db.rows("INSERT INTO annotations VALUES (?,?,?,?,?)", [.text("Legacy"), .integer(0), .integer(0), .integer(0), .text("""
            {"highlight":"yellow","underline":true,"bookmark":true,"note":"from Windows"}
            """)])
    }
    let store = try BibleStore(directory: directory)
    let verses = try await store.chapter(Position(edition: "Legacy"))
    #expect(verses[0].annotation.note == "from Windows")
    #expect(verses[0].annotation.ranges.isEmpty)
    #expect(verses[0].annotation.underline)
}

@Test func importedBuiltinNameCollisionKeepsNotesAndPosition() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: directory) }
    do {
        let db = try Database(directory: directory)
        _ = try db.rows("INSERT INTO editions VALUES (?,?,?)", [.text("CEI 2008"), .text("""
            [{"name":"Personal","abbrev":"P","chapters":[["My text"]]}]
            """), .integer(1)])
        _ = try db.rows("INSERT INTO annotations VALUES (?,?,?,?,?)", [.text("CEI 2008"), .integer(0), .integer(0), .integer(0), .text("{\"note\":\"keep me\"}")])
        try db.set("position", Position(edition: "CEI 2008"))
    }
    let store = try BibleStore(directory: directory)
    let snapshot = try await store.bootstrap()
    #expect(snapshot.position.edition == "CEI 2008 (importata)")
    #expect(snapshot.verses[0].text == "My text")
    #expect(snapshot.verses[0].annotation.note == "keep me")
}

@Test func oldReadingPositionWithoutVerseDefaultsToFirstVerse() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: directory) }
    do {
        let db = try Database(directory: directory)
        _ = try db.rows("INSERT INTO settings VALUES (?,?)", [.text("position"), .text("""
            {"edition":"KJV","book":1,"chapter":2}
            """)])
    }
    let store = try BibleStore(directory: directory)
    #expect(try await store.bootstrap().position == Position(edition: "KJV", book: 1, chapter: 2))
}

@Test func readingPlansAndColoredNotesPersistWithoutLosingMarks() async throws {
    let f=try Fixture();defer{f.cleanup()}
    let id=try await f.store.createReadingPlan(title:"Quaresima 🌅",edition:"CEI 2008",start:"2024-02-28",daily:2,book:0)
    var detail=try await f.store.readingDetail(id,from:"2024-02-28",selected:"2024-02-29")
    #expect(detail.total==50 && detail.duration==25 && detail.days.count==42)
    #expect(detail.chapters.first?.chapter==2)
    try await f.store.completeReadingChapter(id,index:0,complete:true)
    try await f.store.completeReadingChapter(id,index:0,complete:true)
    try await f.store.completeReadingChapter(id,index:1,complete:true)
    detail=try await f.store.readingDetail(id,from:"2024-02-28",selected:"2024-02-27")
    #expect(detail.completed==2 && detail.days[0].done==2 && detail.chapters.isEmpty)
    #expect(try await f.store.resumeReadingPlan(id).chapter==2)
    let position=Position(edition:"CEI 2008",book:0,chapter:2,verse:4)
    try await f.store.savePosition(position)
    _ = try await f.store.writeNote("Speranza 🌅",color:"blue",style:"sticky",at:position)
    _ = try await f.store.organize(position,folder:"Studio",tags:["speranza"])
    _ = try await f.store.mark(position,selection:[SelectedSpan(verse:4,start:0,length:2)],action:"underline")
    let reopened=try BibleStore(directory:f.directory)
    #expect(try await reopened.resumeReadingPlan(id)==position)
    var filter=NoteFilter();filter.query="SPERANZA";filter.tag="speranza"
    let rows=try await reopened.saved(edition:position.edition,filter:filter)
    #expect(rows.count==1 && rows[0].annotation.noteColor=="blue" && rows[0].annotation.noteStyle=="sticky")
    #expect(rows[0].organization.folder=="Studio" && !rows[0].annotation.ranges.isEmpty)
    try await reopened.deleteReadingPlan(id)
    #expect(try await reopened.readingPlans().isEmpty)
    #expect(try ReadingDates.day("2024-03-01")-ReadingDates.day("2024-02-28")==2)
    #expect(throws: BibleError.self){try ReadingDates.day("2023-02-29")}
}
