import Foundation
import AVFoundation
import AppKit
import SQLite3

@main
enum CoreScriptTests {
    static func main() async throws {
        try testStateTransitions()
        try testAudioConverterInput()
        try await MainActor.run {
            let before = NSPasteboard.general.changeCount
            try require(!RexPaste.send("non inviare\n", targetStillValid: { true }), "newline must not be sent")
            try require(!RexPaste.send("test", targetStillValid: { false }), "changed target must be rejected")
            try require(NSPasteboard.general.changeCount == before, "rejected input must not modify clipboard")
        }
        try testSessionStore()
        try testTranscriptSafety()
        try require(TerminalRecovery.supports(bundleID: "com.superlogical.rex"), "Rex should use explicit recovery")
        try require(!TerminalRecovery.supports(bundleID: "com.example.other"), "other apps must not bypass AX checks")
        try testReadOnlyLegacyInspection()
        try await testModelCatalogVerification()
        try testParakeetCacheIsolation()
        print("Core script tests passed.")
    }

    private static func testParakeetCacheIsolation() throws {
        let model = ParakeetModel.all.first { $0.id == "parakeet-ultra" }!
        let cache = ParakeetBridge.cacheDirectory(for: model)
        try require(cache.lastPathComponent == "parakeet-ultra-coreml", "FluidAudio cache must use the repository folder")
        let parent = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        let modelCache = parent.appendingPathComponent(cache.lastPathComponent, isDirectory: true)
        let siblingCache = parent.appendingPathComponent("parakeet-tdt-0.6b-v3-coreml", isDirectory: true)
        try FileManager.default.createDirectory(at: modelCache, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: siblingCache, withIntermediateDirectories: true)
        try Data("model".utf8).write(to: modelCache.appendingPathComponent("weights"))
        try Data("other".utf8).write(to: siblingCache.appendingPathComponent("weights"))
        try FileManager.default.removeItem(at: modelCache)
        try require(!FileManager.default.fileExists(atPath: modelCache.path), "deleting a Parakeet model must remove its actual repository cache")
        try require(FileManager.default.fileExists(atPath: siblingCache.appendingPathComponent("weights").path), "deleting a Parakeet model must preserve other model caches")
        try FileManager.default.removeItem(at: parent)
    }

    private static func testAudioConverterInput() throws {
        let format = AVAudioFormat(standardFormatWithSampleRate: 16_000, channels: 1)!
        let buffer = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: 64)!
        let source = AudioConverterInput(buffer)
        try require(source.take() === buffer, "converter must receive the original buffer once")
        try require(source.take() == nil, "converter must not receive the same buffer twice")
    }

    private static func testSessionStore() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let store = SessionStore(url: directory.appendingPathComponent("test.sqlite3"))
        try store.save(text: "testo privato", duration: 3, keepText: false)
        try require(try store.history().isEmpty, "history must be opt-in")
        try store.save(text: "ciao mondo", duration: 5, keepText: true)
        let totals = try store.totals()
        try require(totals.count == 2 && totals.words == 4 && totals.duration == 8, "statistics must reflect completed sessions")
        try require(try store.history().first?.text == "ciao mondo", "opt-in transcript must be stored")
        try store.clearTranscripts()
        try require(try store.history().isEmpty, "clearing must remove transcripts")
        try require(try store.totals().count == 2, "clearing transcripts must preserve statistics")
    }

    private static func testTranscriptSafety() throws {
        try require(TranscriptSafety.needsReview(" [Musica] "), "standalone music marker requires review")
        try require(TranscriptSafety.needsReview("(music)"), "English music marker requires review")
        try require(!TranscriptSafety.needsReview("Ascolto musica"), "ordinary speech must not be blocked")
        try require(!TranscriptSafety.needsReview("[Musica] Ciao"), "other transcripts must not be blocked")
    }

    private static func testStateTransitions() throws {
        var phase = DictationPhase.idle
        for next in [DictationPhase.preparing, .recording, .processing, .inserting, .completed, .idle] {
            phase = try phase.transition(to: next)
        }
        for active in [DictationPhase.preparing, .recording, .processing, .inserting] {
            try require(active.isActive, "\(active) should be active")
            try require(try active.transition(to: .cancelled) == .cancelled, "active phase should cancel")
            try require(try active.transition(to: .error) == .error, "active phase should fail")
        }
        let invalidTransitions: [(DictationPhase, DictationPhase)] = [
            (.idle, .processing), (.recording, .recording), (.completed, .inserting)
        ]
        for (from, to) in invalidTransitions {
            do {
                _ = try from.transition(to: to)
                throw TestFailure(description: "Expected invalid transition \(from) -> \(to)")
            } catch is DictationTransitionError {}
        }
        let snapshot = DictationSnapshot()
        try require(snapshot.state == .idle, "snapshot defaults to idle")
        let encoded = try JSONSerialization.jsonObject(with: JSONEncoder().encode(snapshot)) as? [String: Any]
        try require(encoded?["elapsed_ms"] as? Int == 0 && encoded?["session_id"] != nil, "snapshot JSON keys should match legacy shape")
    }

    private static func testReadOnlyLegacyInspection() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let database = directory.appendingPathComponent(LegacyReadOnlyRepository.databaseName)
        try createFixture(at: database)
        let before = try Data(contentsOf: database)
        let repository = LegacyReadOnlyRepository()
        let report = try repository.inspect(at: database)
        let after = try Data(contentsOf: database)
        let legacyHome = directory.appendingPathComponent("home", isDirectory: true)
        let legacyDatabase = legacyHome
            .appendingPathComponent("Library/Application Support/app.dicta.desktop", isDirectory: true)
            .appendingPathComponent(LegacyReadOnlyRepository.databaseName)
        try FileManager.default.createDirectory(at: legacyDatabase.deletingLastPathComponent(), withIntermediateDirectories: true)
        try createFixture(at: legacyDatabase)
        let discovered = try LegacyReadOnlyRepository(homeDirectory: legacyHome).inspect()

        try require(before == after, "inspection must not alter database bytes")
        try require(report.schemaVersion == 1, "schema version should be detected")
        try require(report.path == database.standardizedFileURL, "resolved database path should be reported")
        try require(discovered.path == legacyDatabase.standardizedFileURL, "legacy application-data path should be detected")
        let sessions = report.tables.first { $0.name == "sessions" }
        try require(sessions?.rowCount == 1, "actual session count should be detected")
        try require(sessions?.columns.contains("started_at_utc") == true, "actual schema columns should be detected")
        try require(report.tables.first { $0.name == "transcripts" }?.rowCount == 1, "transcript count should be detected")
        try require(report.tables.first { $0.name == "settings" }?.rowCount == 1, "settings count should be detected")

        let missing = directory.appendingPathComponent("missing.sqlite3")
        do {
            _ = try repository.inspect(at: missing)
            throw TestFailure(description: "missing database should fail")
        } catch LegacyRepositoryError.databaseNotFound {}
        try require(!FileManager.default.fileExists(atPath: missing.path), "inspection must not create missing database")
    }

    private static func testModelCatalogVerification() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let payload = Data("abc".utf8)
        let validModel = CatalogModel(id: "fixture", name: "Fixture", sizeBytes: 3,
                                      sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
                                      description: "fixture", filename: "fixture.bin")
        let file = directory.appendingPathComponent(validModel.filename)
        try payload.write(to: file)
        try require(try ModelCatalogService.verify(path: file, model: validModel), "valid hash and size should pass")
        do {
            _ = try ModelCatalogService.verify(path: file, model: CatalogModel(id: "fixture", name: "Fixture", sizeBytes: 4,
                sha256: validModel.sha256, description: "fixture", filename: validModel.filename))
            throw TestFailure(description: "incorrect size should fail")
        } catch ModelCatalogError.invalidSize {}
        do {
            _ = try ModelCatalogService.verify(path: file, model: CatalogModel(id: "fixture", name: "Fixture", sizeBytes: 3,
                sha256: String(repeating: "0", count: 64), description: "fixture", filename: validModel.filename))
            throw TestFailure(description: "incorrect checksum should fail")
        } catch ModelCatalogError.invalidChecksum {}
        try require(ModelCatalogService.models.allSatisfy { !$0.filename.contains("/") && $0.filename.hasPrefix("ggml-") },
                    "catalog paths must be leaf filenames")
        try require(ModelCatalogService.models.allSatisfy { $0.downloadURL.absoluteString.contains(ModelCatalogService.revision) },
                    "catalog URLs must use the pinned revision")
        let existing = directory.appendingPathComponent("ggml-tiny.bin")
        try Data("preexisting".utf8).write(to: existing)
        let before = try Data(contentsOf: existing)
        do {
            _ = try await ModelCatalogService().download(modelID: "tiny", to: directory)
            throw TestFailure(description: "existing destination must be refused")
        } catch ModelCatalogError.destinationExists {}
        try require(try Data(contentsOf: existing) == before, "existing files must remain untouched")
        do {
            try ModelCatalogService().delete(modelID: "../other", from: directory)
            throw TestFailure(description: "unknown model deletion must be refused")
        } catch ModelCatalogError.unknownModel {}
        try require(try Data(contentsOf: existing) == before, "unknown deletion must leave model untouched")
        try ModelCatalogService().delete(modelID: "tiny", from: directory)
        try require(!FileManager.default.fileExists(atPath: existing.path), "catalog model must be deleted")
        try require(try Data(contentsOf: file) == payload, "other files must remain untouched")
    }

    private static func createFixture(at url: URL) throws {
        var database: OpaquePointer?
        guard sqlite3_open(url.path, &database) == SQLITE_OK, let database else {
            throw TestFailure(description: "Could not create SQLite fixture")
        }
        defer { sqlite3_close(database) }
        let sql = """
        PRAGMA user_version=1;
        CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
        CREATE TABLE sessions (id TEXT PRIMARY KEY, started_at_utc TEXT NOT NULL, status TEXT NOT NULL);
        CREATE TABLE transcripts (session_id TEXT PRIMARY KEY, text TEXT NOT NULL);
        INSERT INTO settings VALUES ('settings', '{}');
        INSERT INTO sessions VALUES ('fixture', '2025-01-01T00:00:00Z', 'completed');
        INSERT INTO transcripts VALUES ('fixture', 'private fixture transcript');
        """
        guard sqlite3_exec(database, sql, nil, nil, nil) == SQLITE_OK else {
            throw TestFailure(description: String(cString: sqlite3_errmsg(database)))
        }
    }

    private static func require(_ condition: @autoclosure () throws -> Bool, _ message: String) throws {
        guard try condition() else { throw TestFailure(description: message) }
    }
}

struct TestFailure: Error, CustomStringConvertible {
    let description: String
    init(description: String) { self.description = description }
}
