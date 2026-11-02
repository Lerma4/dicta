import Foundation
import SQLite3

struct LegacyTable: Equatable {
    let name: String
    let columns: [String]
    let rowCount: Int64?
}

struct LegacyDatabaseInspection: Equatable {
    let path: URL
    let schemaVersion: Int
    let tables: [LegacyTable]
}

enum LegacyRepositoryError: Error, LocalizedError {
    case databaseNotFound([URL])
    case sqlite(String)

    var errorDescription: String? {
        switch self {
        case let .databaseNotFound(paths):
            "Legacy Dicta database not found. Checked: \(paths.map(\.path).joined(separator: ", "))"
        case let .sqlite(message): message
        }
    }
}

/// Inspects the legacy Tauri database without creating, migrating, or modifying it.
struct LegacyReadOnlyRepository {
    static let databaseName = "dicta.sqlite3"
    static let legacyApplicationSupportDirectory = "app.dicta.desktop"

    let fileManager: FileManager
    let homeDirectory: URL

    init(fileManager: FileManager = .default, homeDirectory: URL = URL(fileURLWithPath: NSHomeDirectory())) {
        self.fileManager = fileManager
        self.homeDirectory = homeDirectory
    }

    var candidateURLs: [URL] {
        [homeDirectory
            .appendingPathComponent("Library/Application Support", isDirectory: true)
            .appendingPathComponent(Self.legacyApplicationSupportDirectory, isDirectory: true)
            .appendingPathComponent(Self.databaseName)]
    }

    func inspect(at url: URL? = nil) throws -> LegacyDatabaseInspection {
        let candidates = url.map { [$0] } ?? candidateURLs
        guard let databaseURL = candidates.first(where: { fileManager.fileExists(atPath: $0.path) }) else {
            throw LegacyRepositoryError.databaseNotFound(candidates)
        }
        guard databaseURL.isFileURL else {
            throw LegacyRepositoryError.sqlite("Legacy database location must be a file URL.")
        }

        var handle: OpaquePointer?
        let openResult = sqlite3_open_v2(databaseURL.path, &handle, SQLITE_OPEN_READONLY | SQLITE_OPEN_FULLMUTEX, nil)
        guard openResult == SQLITE_OK, let connection = handle else {
            let message = handle.map { String(cString: sqlite3_errmsg($0)) } ?? "SQLite open failed (\(openResult))."
            if let handle { sqlite3_close(handle) }
            throw LegacyRepositoryError.sqlite(message)
        }
        defer { sqlite3_close(connection) }
        guard sqlite3_db_readonly(connection, "main") == 1 else {
            throw LegacyRepositoryError.sqlite("SQLite did not open the legacy database read-only.")
        }

        let version = try Self.integerQuery(connection, sql: "PRAGMA user_version")
        let tableNames = try Self.textRows(connection,
            sql: "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        let tables = try tableNames.map { name -> LegacyTable in
            let escapedName = name.replacingOccurrences(of: "\"", with: "\"\"")
            let columns = try Self.textRows(connection, sql: "PRAGMA table_info(\"\(escapedName)\")", column: 1)
            let count: Int64?
            if name == "sessions" || name == "transcripts" || name == "settings" {
                let escaped = name.replacingOccurrences(of: "\"", with: "\"\"")
                count = try Self.integerQuery(connection, sql: "SELECT COUNT(*) FROM \"\(escaped)\"")
            } else {
                count = nil
            }
            return LegacyTable(name: name, columns: columns, rowCount: count)
        }
        return LegacyDatabaseInspection(path: databaseURL.standardizedFileURL, schemaVersion: Int(version), tables: tables)
    }

    private static func integerQuery(_ connection: OpaquePointer, sql: String) throws -> Int64 {
        var statement: OpaquePointer?
        guard sqlite3_prepare_v2(connection, sql, -1, &statement, nil) == SQLITE_OK, let statement else {
            throw LegacyRepositoryError.sqlite(String(cString: sqlite3_errmsg(connection)))
        }
        defer { sqlite3_finalize(statement) }
        guard sqlite3_step(statement) == SQLITE_ROW else {
            throw LegacyRepositoryError.sqlite(String(cString: sqlite3_errmsg(connection)))
        }
        return sqlite3_column_int64(statement, 0)
    }

    private static func textRows(_ connection: OpaquePointer, sql: String, column: Int32 = 0) throws -> [String] {
        var statement: OpaquePointer?
        guard sqlite3_prepare_v2(connection, sql, -1, &statement, nil) == SQLITE_OK, let statement else {
            throw LegacyRepositoryError.sqlite(String(cString: sqlite3_errmsg(connection)))
        }
        defer { sqlite3_finalize(statement) }
        var values: [String] = []
        while true {
            switch sqlite3_step(statement) {
            case SQLITE_ROW:
                if let value = sqlite3_column_text(statement, column) {
                    values.append(String(cString: value))
                }
            case SQLITE_DONE:
                return values
            default:
                throw LegacyRepositoryError.sqlite(String(cString: sqlite3_errmsg(connection)))
            }
        }
    }
}
