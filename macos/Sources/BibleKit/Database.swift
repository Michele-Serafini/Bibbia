import Foundation
import CSQLite

enum SQLValue {
    case text(String), integer(Int), null
    var string: String? { if case .text(let s) = self { return s }; return nil }
    var integer: Int? { if case .integer(let n) = self { return n }; return nil }
}

/// Confined to BibleStore's actor. Prepared statements are reused and finalized on close.
final class Database {
    var handle: OpaquePointer?
    private var statements: [String: OpaquePointer] = [:]
    private let transient = unsafeBitCast(-1, to: sqlite3_destructor_type.self)

    let directory:URL
    init(directory: URL) throws {
        self.directory=directory
        let fm = FileManager.default
        if !fm.fileExists(atPath: directory.path) {
            try fm.createDirectory(at: directory, withIntermediateDirectories: true,
                                   attributes: [.posixPermissions: 0o700])
        }
        let file = directory.appendingPathComponent("bible.sqlite3")
        let existed = fm.fileExists(atPath: file.path)
        guard sqlite3_open_v2(file.path, &handle, SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE | SQLITE_OPEN_NOMUTEX, nil) == SQLITE_OK else {
            let message = handle.map { String(cString: sqlite3_errmsg($0)) } ?? "SQLite unavailable"
            sqlite3_close(handle); handle = nil
            throw BibleError.invalid(message)
        }
        if !existed { try fm.setAttributes([.posixPermissions: 0o600], ofItemAtPath: file.path) }
        sqlite3_busy_timeout(handle, 3000)
        try applyPendingRestore()
        try executeBatch("""
            PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA cache_size=-1024;
            CREATE TABLE IF NOT EXISTS annotations (
              edition TEXT NOT NULL, book INTEGER NOT NULL, chapter INTEGER NOT NULL,
              verse INTEGER NOT NULL, payload TEXT NOT NULL, PRIMARY KEY(edition,book,chapter,verse));
            CREATE TABLE IF NOT EXISTS note_groups(kind TEXT NOT NULL CHECK(kind IN ('folder','tag')),name TEXT NOT NULL,PRIMARY KEY(kind,name));
            CREATE TABLE IF NOT EXISTS note_organization(edition TEXT NOT NULL,book INTEGER NOT NULL,chapter INTEGER NOT NULL,verse INTEGER NOT NULL,folder TEXT NOT NULL DEFAULT '',tags TEXT NOT NULL DEFAULT '[]',PRIMARY KEY(edition,book,chapter,verse));
            CREATE INDEX IF NOT EXISTS note_organization_folder ON note_organization(edition,folder,book,chapter,verse);
            CREATE TABLE IF NOT EXISTS reading_plans(id INTEGER PRIMARY KEY,payload TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS reading_progress(plan_id INTEGER NOT NULL REFERENCES reading_plans(id) ON DELETE CASCADE,chapter_index INTEGER NOT NULL,PRIMARY KEY(plan_id,chapter_index));
            CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, payload TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS editions (id TEXT PRIMARY KEY, payload TEXT NOT NULL, book_count INTEGER);
            """)
        if !(try rows("PRAGMA table_info(editions)")).contains(where: { $0[1].string == "book_count" }) {
            try executeBatch("ALTER TABLE editions ADD COLUMN book_count INTEGER")
        }
        try executeBatch("UPDATE editions SET book_count=json_array_length(payload) WHERE book_count IS NULL")
    }

    deinit {
        for statement in statements.values { sqlite3_finalize(statement) }
        sqlite3_close(handle)
    }

    private func fail() -> BibleError { .invalid(String(cString: sqlite3_errmsg(handle))) }

    func executeBatch(_ sql: String) throws {
        guard sqlite3_exec(handle, sql, nil, nil, nil) == SQLITE_OK else { throw fail() }
    }

    func rows(_ sql: String, _ bindings: [SQLValue] = []) throws -> [[SQLValue]] {
        var result = [[SQLValue]]()
        try scan(sql, bindings) { row in result.append(row); return true }
        return result
    }

    func scan(_ sql: String, _ bindings: [SQLValue] = [], _ visit: ([SQLValue]) throws -> Bool) throws {
        let statement: OpaquePointer
        if let cached = statements[sql] { statement = cached }
        else {
            var prepared: OpaquePointer?
            guard sqlite3_prepare_v2(handle, sql, -1, &prepared, nil) == SQLITE_OK,
                  let prepared else { throw fail() }
            statement = prepared; statements[sql] = prepared
        }
        defer { sqlite3_reset(statement); sqlite3_clear_bindings(statement) }
        for (index, value) in bindings.enumerated() {
            let status: Int32
            switch value {
            case .text(let string): status = string.withCString { sqlite3_bind_text(statement, Int32(index + 1), $0, -1, transient) }
            case .integer(let number): status = sqlite3_bind_int64(statement, Int32(index + 1), Int64(number))
            case .null: status = sqlite3_bind_null(statement, Int32(index + 1))
            }
            guard status == SQLITE_OK else { throw fail() }
        }
        while true {
            let status = sqlite3_step(statement)
            if status == SQLITE_DONE { return }
            guard status == SQLITE_ROW else { throw fail() }
            let row: [SQLValue] = (0..<sqlite3_column_count(statement)).map { column in
                switch sqlite3_column_type(statement, column) {
                case SQLITE_INTEGER: return .integer(Int(sqlite3_column_int64(statement, column)))
                case SQLITE_TEXT:
                    return .text(String(cString: sqlite3_column_text(statement, column)))
                default: return .null
                }
            }
            if try !visit(row) { return }
        }
    }

    func transaction<T>(_ operation: () throws -> T) throws -> T {
        try executeBatch("BEGIN IMMEDIATE")
        do {
            let value = try operation()
            try executeBatch("COMMIT")
            return value
        } catch {
            try? executeBatch("ROLLBACK")
            throw error
        }
    }

    func setting<T: Decodable>(_ key: String, as type: T.Type) throws -> T? {
        guard let value = try rows("SELECT payload FROM settings WHERE key=?", [.text(key)]).first?.first?.string else { return nil }
        return try JSONDecoder().decode(type, from: Data(value.utf8))
    }

    func set<T: Encodable>(_ key: String, _ value: T) throws {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        let payload = String(decoding: try encoder.encode(value), as: UTF8.self)
        _ = try rows("INSERT INTO settings VALUES (?,?) ON CONFLICT(key) DO UPDATE SET payload=excluded.payload WHERE payload<>excluded.payload",
                     [.text(key), .text(payload)])
    }
}
