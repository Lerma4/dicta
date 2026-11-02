import XCTest
@testable import DictaMacOS

final class AvailabilityTests: XCTestCase {
    func testAllExistingSectionsArePresentAndExplicitlyUnavailable() {
        XCTAssertEqual(DictaSection.allCases.map(\.title), [
            "Panoramica", "Statistiche", "Cronologia", "Modelli", "Impostazioni"
        ])
        for section in DictaSection.allCases {
            XCTAssertFalse(section.availability.isEmpty, "\(section.title) must explain unavailable features")
        }
    }

    func testModelSectionDoesNotImplyAutomaticDownloads() {
        XCTAssertTrue(DictaSection.models.availability.contains("Nessun download viene avviato"))
    }

    func testParakeetCatalogPreservesPlatformAndModelMetadataWithoutDownloading() {
        XCTAssertEqual(ParakeetModel.all.map(\.id), ["parakeet-ultra", "parakeet-v3", "parakeet-redux"])
        XCTAssertEqual(ParakeetModel.all.map(\.requiredOS), [14, 14, 15])
        XCTAssertEqual(ParakeetModel.all.map(\.approximateBytes), [630_000_000, 480_000_000, 220_000_000])
        XCTAssertTrue(ParakeetModel.all.allSatisfy { URL(string: $0.modelURL)?.host == "huggingface.co" })
    }

    func testWhisperRunsOfflineOnInMemorySilenceWhenModelIsInstalled() throws {
        let modelURL: URL
        do {
            modelURL = try WhisperEngine.installedModelURL()
        } catch WhisperEngine.EngineError.modelUnavailable {
            throw XCTSkip("No verified local Whisper model is installed")
        }
        let engine = try WhisperEngine(modelURL: modelURL)
        let transcript = try engine.transcribe(pcm16kMono: Array(repeating: 0, count: 16_000))
        XCTAssertLessThanOrEqual(transcript.utf8.count, 16_000)
    }

    @MainActor
    func testNativeDictationStartsIdleAndNeverCreatesRecoveryTextWithoutTranscription() {
        let service = NativeDictationService()
        XCTAssertEqual(service.phase, .idle)
        XCTAssertFalse(service.recoveryPending)
        XCTAssertNil(service.recoveryText)
    }
}
