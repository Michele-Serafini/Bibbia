import Foundation

public struct NoteFilter: Sendable, Equatable {
    public var query = ""
    public var folder: String?
    public var tag: String?
    public var kind = "all"
    public init() {}
}

extension BibleStore {
    private func groupName(_ raw: String) throws -> String {
        let name = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty, name.utf8.count <= 80, !name.contains(","), !name.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) }) else { throw BibleError.invalid("Nome non valido (1–80 byte, senza virgole) / Invalid name (1–80 bytes, no commas)") }
        return name
    }
    public func collections() throws -> NoteCollections {
        var result = NoteCollections()
        for row in try database.rows("SELECT kind,name FROM note_groups ORDER BY kind,name") {
            if row[0].string == "folder" { result.folders.append(row[1].string!) }
            else { result.tags.append(row[1].string!) }
        }
        return result
    }
    @discardableResult
    public func organize(_ position: Position, folder: String, tags: [String]) throws -> NoteOrganization {
        let p: [SQLValue] = [.text(position.edition), .integer(position.book), .integer(position.chapter), .integer(position.verse)]
        guard !(try database.rows("SELECT 1 FROM annotations WHERE edition=? AND book=? AND chapter=? AND verse=?",p)).isEmpty else { throw BibleError.invalid("Salva prima un appunto / Save an annotation first") }
        let folder = folder.trimmingCharacters(in: .whitespacesAndNewlines)
        if !folder.isEmpty { _ = try groupName(folder) }
        guard tags.count <= 16 else { throw BibleError.invalid("Al massimo 16 etichette / At most 16 tags") }
        let tags = try Array(Set(tags.map { try groupName($0) })).sorted()
        let result = NoteOrganization(folder: folder, tags: tags)
        try database.transaction {
            if !folder.isEmpty { _ = try database.rows("INSERT OR IGNORE INTO note_groups VALUES('folder',?)",[.text(folder)]) }
            for tag in tags { _ = try database.rows("INSERT OR IGNORE INTO note_groups VALUES('tag',?)",[.text(tag)]) }
            let payload = String(decoding: try JSONEncoder().encode(tags), as: UTF8.self)
            _ = try database.rows("INSERT INTO note_organization VALUES(?,?,?,?,?,?) ON CONFLICT(edition,book,chapter,verse) DO UPDATE SET folder=excluded.folder,tags=excluded.tags",p+[.text(folder),.text(payload)])
        }
        return result
    }
    @discardableResult
    public func collection(kind: String, action: String, name: String, newName: String = "") throws -> NoteCollections {
        guard ["folder","tag"].contains(kind) else { throw BibleError.invalid("Tipo raccolta non valido / Invalid collection kind") }
        let old = try groupName(name)
        try database.transaction {
            if action == "create" { _ = try database.rows("INSERT OR IGNORE INTO note_groups VALUES(?,?)",[.text(kind),.text(old)]); return }
            guard ["rename","delete"].contains(action), !(try database.rows("SELECT 1 FROM note_groups WHERE kind=? AND name=?",[.text(kind),.text(old)])).isEmpty else { throw BibleError.invalid("Raccolta non disponibile / Collection unavailable") }
            let new = action == "rename" ? try groupName(newName) : ""
            if old == new { return }
            if action == "rename" { _ = try database.rows("INSERT INTO note_groups VALUES(?,?)",[.text(kind),.text(new)]) }
            if kind == "folder" { _ = try database.rows("UPDATE note_organization SET folder=? WHERE folder=?",[.text(new),.text(old)]) }
            else if action == "rename" { _ = try database.rows("UPDATE note_organization SET tags=(SELECT json_group_array(CASE WHEN value=?1 THEN ?2 ELSE value END) FROM json_each(note_organization.tags)) WHERE EXISTS(SELECT 1 FROM json_each(note_organization.tags) WHERE value=?1)",[.text(old),.text(new)]) }
            else { _ = try database.rows("UPDATE note_organization SET tags=(SELECT json_group_array(value) FROM json_each(note_organization.tags) WHERE value<>?1) WHERE EXISTS(SELECT 1 FROM json_each(note_organization.tags) WHERE value=?1)",[.text(old)]) }
            _ = try database.rows("DELETE FROM note_groups WHERE kind=? AND name=?",[.text(kind),.text(old)])
        }
        return try collections()
    }
    public func saved(edition: String, limit: Int = 200, offset: Int = 0, filter: NoteFilter = NoteFilter()) throws -> [Hit] {
        guard (1...200).contains(limit), offset >= 0, filter.query.utf8.count <= 1024, ["all","note","marked","bookmark"].contains(filter.kind) else { throw BibleError.invalid("Filtro o pagina non valida / Invalid filter or page") }
        let source = try corpus(edition), query = filter.query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        var result = [Hit](), skipped = 0
        try database.scan("SELECT a.book,a.chapter,a.verse,a.payload,COALESCE(o.folder,''),COALESCE(o.tags,'[]') FROM annotations a LEFT JOIN note_organization o USING(edition,book,chapter,verse) WHERE a.edition=?1 AND (?2 IS NULL OR COALESCE(o.folder,'')=?2) AND (?3 IS NULL OR EXISTS(SELECT 1 FROM json_each(COALESCE(o.tags,'[]')) WHERE value=?3)) ORDER BY a.book,a.chapter,a.verse",[.text(edition),filter.folder.map(SQLValue.text) ?? .null,filter.tag.map(SQLValue.text) ?? .null]) { row in
            try Task.checkCancellation()
            guard let b=row[0].integer,let c=row[1].integer,let v=row[2].integer,let json=row[3].string else { throw BibleError.invalid("Appunto non valido / Invalid annotation") }
            let a = try JSONDecoder().decode(Annotation.self,from:Data(json.utf8))
            let marked = a.highlight != nil || a.underline || !a.ranges.isEmpty
            if (filter.kind == "note" && a.note.isEmpty) || (filter.kind == "marked" && !marked) || (filter.kind == "bookmark" && !a.bookmark) { return true }
            let span = try source.chapter(b,c), text = try source.verse(b,c,v)
            let org = NoteOrganization(folder:row[4].string ?? "",tags:try JSONDecoder().decode([String].self,from:Data((row[5].string ?? "[]").utf8)))
            var hit = Hit(position:Position(edition:edition,book:b,chapter:c,verse:v),name:source.books[b].name,label:span.label(v),text:text,annotation:a)
            hit.organization = org
            if !query.isEmpty && !([text,a.note,hit.reference,org.folder]+org.tags).contains(where: { $0.lowercased().contains(query) }) { return true }
            if skipped < offset { skipped += 1; return true }
            result.append(hit)
            return result.count < limit
        }
        return result
    }
}
