import Foundation
import SQLite3

struct DictationRecord: Identifiable {
    let id: String
    let date: Date
    let duration: Double
    let words: Int
    let text: String?
}

struct DictationTotals {
    let count: Int
    let words: Int
    let duration: Double
}

struct SessionStore {
    let url: URL
    init(url: URL? = nil) {
        self.url = url ?? FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("app.dicta.native", isDirectory: true).appendingPathComponent("sessions.sqlite3")
    }

    private func withDatabase<T>(_ operation: (OpaquePointer) throws -> T) throws -> T {
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        var handle: OpaquePointer?
        guard sqlite3_open_v2(url.path, &handle, SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE | SQLITE_OPEN_FULLMUTEX, nil) == SQLITE_OK, let db = handle else {
            let error = handle.map { String(cString: sqlite3_errmsg($0)) } ?? "Impossibile aprire il database."
            if let handle { sqlite3_close(handle) }
            throw StoreError.database(error)
        }
        defer { sqlite3_close(db) }
        sqlite3_busy_timeout(db, 5000)
        guard sqlite3_exec(db, "CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, date REAL NOT NULL, duration REAL NOT NULL, words INTEGER NOT NULL, transcript TEXT)", nil, nil, nil) == SQLITE_OK else {
            throw StoreError.database(String(cString: sqlite3_errmsg(db)))
        }
        return try operation(db)
    }

    func save(text: String, duration: Double, keepText: Bool, date: Date = Date()) throws {
        try withDatabase { db in
            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, "INSERT INTO sessions (id,date,duration,words,transcript) VALUES (?,?,?,?,?)", -1, &stmt, nil) == SQLITE_OK, let stmt else { throw StoreError.database(String(cString: sqlite3_errmsg(db))) }
            defer { sqlite3_finalize(stmt) }
            let transient = unsafeBitCast(-1, to: sqlite3_destructor_type.self)
            _ = UUID().uuidString.withCString { sqlite3_bind_text(stmt, 1, $0, -1, transient) }
            sqlite3_bind_double(stmt, 2, date.timeIntervalSince1970)
            sqlite3_bind_double(stmt, 3, max(0, duration))
            sqlite3_bind_int64(stmt, 4, Int64(text.split(whereSeparator: { $0.isWhitespace }).count))
            if keepText { _ = text.withCString { sqlite3_bind_text(stmt, 5, $0, -1, transient) } } else { sqlite3_bind_null(stmt, 5) }
            guard sqlite3_step(stmt) == SQLITE_DONE else { throw StoreError.database(String(cString: sqlite3_errmsg(db))) }
        }
    }

    func totals() throws -> DictationTotals {
        try withDatabase { db in
            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, "SELECT COUNT(*), COALESCE(SUM(words),0), COALESCE(SUM(duration),0) FROM sessions", -1, &stmt, nil) == SQLITE_OK, let stmt else { throw StoreError.database(String(cString: sqlite3_errmsg(db))) }
            defer { sqlite3_finalize(stmt) }
            guard sqlite3_step(stmt) == SQLITE_ROW else { throw StoreError.database(String(cString: sqlite3_errmsg(db))) }
            return DictationTotals(count: Int(sqlite3_column_int64(stmt, 0)), words: Int(sqlite3_column_int64(stmt, 1)), duration: sqlite3_column_double(stmt, 2))
        }
    }

    func history() throws -> [DictationRecord] {
        try withDatabase { db in
            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, "SELECT id,date,duration,words,transcript FROM sessions WHERE transcript IS NOT NULL ORDER BY date DESC LIMIT 200", -1, &stmt, nil) == SQLITE_OK, let stmt else { throw StoreError.database(String(cString: sqlite3_errmsg(db))) }
            defer { sqlite3_finalize(stmt) }
            var result: [DictationRecord] = []
            while true {
                let status = sqlite3_step(stmt)
                if status == SQLITE_DONE { return result }
                guard status == SQLITE_ROW, let id = sqlite3_column_text(stmt, 0) else { throw StoreError.database(String(cString: sqlite3_errmsg(db))) }
                let text = sqlite3_column_text(stmt, 4).map { String(cString: $0) }
                result.append(DictationRecord(id: String(cString: id), date: Date(timeIntervalSince1970: sqlite3_column_double(stmt, 1)), duration: sqlite3_column_double(stmt, 2), words: Int(sqlite3_column_int64(stmt, 3)), text: text))
            }
        }
    }

    func clearTranscripts() throws {
        try withDatabase { db in
            guard sqlite3_exec(db, "UPDATE sessions SET transcript=NULL", nil, nil, nil) == SQLITE_OK else { throw StoreError.database(String(cString: sqlite3_errmsg(db))) }
        }
    }

    enum StoreError: Error, LocalizedError {
        case database(String)
        var errorDescription: String? { if case let .database(message) = self { return message }; return nil }
    }
}
