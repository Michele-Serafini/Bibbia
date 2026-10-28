import Foundation
import Testing
@testable import BibleKit

@Test func comparisonsAlignCanonicalBooksAndOriginalReferences() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent("bibbia-compare-\(UUID().uuidString)")
    defer { try? FileManager.default.removeItem(at: directory) }
    let store = try BibleStore(directory: directory)
    let john = try await store.compare(Position(edition: "CEI 2008", book: 49, chapter: 2), other: "KJV")
    #expect(john.status == "aligned")
    #expect(john.rows[15].right[0].book == 42)
    #expect(john.rows[15].right[0].label == "16")
    #expect(john.rows[15].right[0].text.contains("God so loved"))
    let psalm = try await store.compare(Position(edition: "CEI 2008", book: 22, chapter: 22), other: "Martini 1781 (cattolica)")
    #expect(psalm.rows[0].right[0].chapter == 21)
    let joel = try await store.compare(Position(edition: "CEI 2008", book: 35, chapter: 2), other: "KJV")
    #expect(joel.rows[0].right[0].reference == "Joel 2:28")
    let absent = try await store.compare(Position(edition: "CEI 2008", book: 16), other: "KJV")
    #expect(absent.status == "missingBook" && absent.rows.allSatisfy(\.right.isEmpty))
}
