import Foundation
import Testing
@testable import BibleKit

@Test func organizedNotesKeepDataAcrossSearchRenameDeleteAndReopen() async throws {
    let directory=FileManager.default.temporaryDirectory.appendingPathComponent("bibbia-organize-\(UUID().uuidString)")
    defer { try? FileManager.default.removeItem(at:directory) }
    let store=try BibleStore(directory:directory)
    for verse in 0..<4 { _ = try await store.setNote(verse%2 == 0 ? "PERCHÉ 🌅 gioia":"Pace",at:Position(edition:"KJV",verse:verse)) }
    _ = try await store.mark(Position(edition:"KJV"),selection:[SelectedSpan(verse:1,start:0,length:2)],action:"underline")
    try await store.collection(kind:"folder",action:"create",name:"Vuota")
    for verse in [0,2] { try await store.organize(Position(edition:"KJV",verse:verse),folder:"Studio",tags:["speranza","amore","amore"]) }
    var filter=NoteFilter();filter.folder="Studio";filter.tag="amore";filter.query="perché 🌅";filter.kind="note"
    let page=try await store.saved(edition:"KJV",limit:1,offset:1,filter:filter)
    #expect(page.count==1 && page[0].position.verse==2)
    #expect(page[0].organization.tags==["amore","speranza"])
    filter=NoteFilter();filter.query="created"
    #expect(try await store.saved(edition:"KJV",filter:filter).count==1)
    filter=NoteFilter();filter.folder=""
    #expect(try await store.saved(edition:"KJV",filter:filter).count==2)
    filter=NoteFilter();filter.kind="marked"
    #expect(try await store.saved(edition:"KJV",filter:filter).first?.position.verse==1)
    filter=NoteFilter();filter.query="%' OR 1=1 --"
    #expect(try await store.saved(edition:"KJV",filter:filter).isEmpty)
    do { try await store.collection(kind:"folder",action:"rename",name:"Studio",newName:"Vuota");Issue.record("Collision must roll back") } catch {}
    try await store.collection(kind:"folder",action:"rename",name:"Studio",newName:"Vangelo")
    try await store.collection(kind:"tag",action:"rename",name:"amore",newName:"carità")
    _ = try await store.mark(Position(edition:"KJV"),selection:[SelectedSpan(verse:0,start:0,length:2)],action:"yellow")
    let reopened=try BibleStore(directory:directory)
    filter=NoteFilter();filter.folder="Vangelo";filter.tag="carità"
    let saved=try await reopened.saved(edition:"KJV",filter:filter)
    #expect(saved.count==2 && saved[0].annotation.note=="PERCHÉ 🌅 gioia")
    try await reopened.collection(kind:"folder",action:"delete",name:"Vangelo")
    try await reopened.collection(kind:"tag",action:"delete",name:"carità")
    let remaining=try await reopened.saved(edition:"KJV")
    #expect(remaining.count==4 && remaining[0].organization.folder.isEmpty && remaining[0].organization.tags==["speranza"])
    _ = try await reopened.setNote("",at:Position(edition:"KJV",verse:2))
    let db=try Database(directory:directory)
    #expect(try db.rows("SELECT 1 FROM note_organization WHERE verse=2").isEmpty)
}
