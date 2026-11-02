import Foundation
import WhisperBridge

/// In-memory transcription backend. It never writes audio or transcript contents to disk or logs.
final class WhisperEngine {
    enum EngineError: Error, LocalizedError {
        case modelUnavailable
        case modelLoadFailed
        case invalidAudio
        case inferenceFailed

        var errorDescription: String? {
            switch self {
            case .modelUnavailable: "Il modello selezionato non è installato o non supera la verifica. Apri Modelli e installalo di nuovo."
            case .modelLoadFailed: "Whisper non riesce ad aprire il modello selezionato. Verifica lo spazio libero e reinstalla il modello."
            case .invalidAudio: "L'audio acquisito non è valido. Riprova la dettatura."
            case .inferenceFailed: "La trascrizione non ha prodotto testo. Riprova parlando più vicino al microfono."
            }
        }
    }

    private let handle: OpaquePointer

    init(modelURL: URL) throws {
        guard let engine = modelURL.path.withCString({ dicta_whisper_create($0) }) else {
            throw EngineError.modelLoadFailed
        }
        handle = engine
    }

    deinit { dicta_whisper_destroy(handle) }

    func transcribe(pcm16kMono samples: [Float], language: String = "it") throws -> String {
        guard !samples.isEmpty, samples.allSatisfy(\.isFinite) else { throw EngineError.invalidAudio }
        let result = samples.withUnsafeBufferPointer { buffer in
            language.withCString { dicta_whisper_transcribe(handle, buffer.baseAddress, buffer.count, $0) }
        }
        guard let result else { throw EngineError.inferenceFailed }
        defer { dicta_whisper_free_text(result) }
        return String(cString: result)
    }

    static func installedModelURL(modelID: String) throws -> URL {
        let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("app.dicta.desktop", isDirectory: true)
            .appendingPathComponent("models", isDirectory: true)
        guard let model = ModelCatalogService.models.first(where: { $0.id == modelID }) else {
            throw EngineError.modelUnavailable
        }
        let candidate = support.appendingPathComponent(model.filename)
        if (try? ModelCatalogService.verify(path: candidate, model: model)) == true { return candidate }
        throw EngineError.modelUnavailable
    }
}
