import Darwin
import Foundation

struct ParakeetModel: Identifiable, Sendable {
    let id: String
    let name: String
    let requiredOS: Int
    let approximateBytes: Int64
    let description: String
    let modelURL: String
    var bridgeID: Int32 {
        switch id {
        case "parakeet-ultra": 0
        case "parakeet-v3": 1
        case "parakeet-redux": 2
        default: -1
        }
    }

    static let all: [ParakeetModel] = [
        .init(id: "parakeet-ultra", name: "Parakeet Ultra", requiredOS: 14, approximateBytes: 630_000_000, description: "Int8, consigliato per nuove installazioni.", modelURL: "https://huggingface.co/FluidInference/parakeet-ultra-coreml"),
        .init(id: "parakeet-v3", name: "Parakeet V3", requiredOS: 14, approximateBytes: 480_000_000, description: "Modello multilingue compatibile con macOS 14+.", modelURL: "https://huggingface.co/FluidInference/parakeet-tdt-0.6b-v3-coreml"),
        .init(id: "parakeet-redux", name: "Parakeet Redux", requiredOS: 15, approximateBytes: 220_000_000, description: "Modello compatto; la prima compilazione Core ML può richiedere diversi minuti.", modelURL: "https://huggingface.co/FluidInference/parakeet-redux-coreml")
    ]
}

private typealias DownloadFn = @convention(c) (Int32, UnsafePointer<CChar>, (@convention(c) (Double, UnsafeMutableRawPointer?) -> Void)?, UnsafeMutableRawPointer?) -> Int32
private typealias CancelFn = @convention(c) () -> Void
private typealias InstalledFn = @convention(c) (Int32, UnsafePointer<CChar>) -> Int32
private typealias CreateFn = @convention(c) (Int32, UnsafePointer<CChar>) -> UnsafeMutableRawPointer?
private typealias TranscribeFn = @convention(c) (UnsafeMutableRawPointer, UnsafePointer<Float>, Int) -> UnsafeMutablePointer<CChar>?
private typealias DestroyFn = @convention(c) (UnsafeMutableRawPointer) -> Void
private typealias FreeTextFn = @convention(c) (UnsafeMutablePointer<CChar>) -> Void

final class ParakeetBridge: @unchecked Sendable {
    static let shared = ParakeetBridge()
    private let handle: UnsafeMutableRawPointer?
    private let downloadFn: DownloadFn?
    private let installedFn: InstalledFn?
    private let cancelFn: CancelFn?
    private let createFn: CreateFn?
    private let transcribeFn: TranscribeFn?
    private let destroyFn: DestroyFn?
    private let freeTextFn: FreeTextFn?
    private let engineLock = NSLock()
    private var engines: [String: UnsafeMutableRawPointer] = [:]

    private init() {
        guard #available(macOS 14, *), let url = Bundle.main.privateFrameworksURL?.appendingPathComponent("libDictaParakeetBridge.dylib"), let library = dlopen(url.path, RTLD_NOW | RTLD_LOCAL) else {
            handle = nil; downloadFn = nil; installedFn = nil; cancelFn = nil; createFn = nil; transcribeFn = nil; destroyFn = nil; freeTextFn = nil
            return
        }
        handle = library
        downloadFn = Self.symbol("dicta_parakeet_download", in: library)
        installedFn = Self.symbol("dicta_parakeet_is_installed", in: library)
        cancelFn = Self.symbol("dicta_parakeet_cancel_download", in: library)
        createFn = Self.symbol("dicta_parakeet_create", in: library)
        transcribeFn = Self.symbol("dicta_parakeet_transcribe", in: library)
        destroyFn = Self.symbol("dicta_parakeet_destroy", in: library)
        freeTextFn = Self.symbol("dicta_parakeet_free_text", in: library)
    }

    private static func symbol<T>(_ name: String, in library: UnsafeMutableRawPointer) -> T? {
        guard let raw = dlsym(library, name) else { return nil }
        return unsafeBitCast(raw, to: T.self)
    }

    static func cacheDirectory(for model: ParakeetModel) -> URL {
        let repository = URL(string: model.modelURL)!.lastPathComponent
        return FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("app.dicta.desktop/models/parakeet", isDirectory: true)
            .appendingPathComponent(repository, isDirectory: true)
    }

    func cancelDownload() { cancelFn?() }

    func isInstalled(_ model: ParakeetModel) -> Bool {
        guard let installedFn else { return false }
        return Self.cacheDirectory(for: model).path.withCString { installedFn(model.bridgeID, $0) == 1 }
    }

    func download(_ model: ParakeetModel, progress: DownloadProgressValue) throws {
        guard let downloadFn else { throw ParakeetError.unavailable }
        let result = Self.cacheDirectory(for: model).path.withCString { path in
            downloadFn(model.bridgeID, path, { value, context in
                guard let context else { return }
                Unmanaged<DownloadProgressValue>.fromOpaque(context).takeUnretainedValue().update(value)
            }, Unmanaged.passUnretained(progress).toOpaque())
        }
        guard result == 0 else { throw ParakeetError.downloadFailed }
    }

    func transcribe(_ samples: [Float], model: ParakeetModel) throws -> String {
        guard let createFn, let transcribeFn, let freeTextFn else { throw ParakeetError.unavailable }
        engineLock.lock()
        defer { engineLock.unlock() }
        for id in Array(engines.keys) where id != model.id {
            if let engine = engines.removeValue(forKey: id) { destroyFn?(engine) }
        }
        let opaque: UnsafeMutableRawPointer
        if let cached = engines[model.id] {
            opaque = cached
        } else {
            guard let created = Self.cacheDirectory(for: model).path.withCString({ createFn(model.bridgeID, $0) }) else {
                throw ParakeetError.notInstalled
            }
            engines[model.id] = created
            opaque = created
        }
        let output = samples.withUnsafeBufferPointer { buffer in
            transcribeFn(opaque, buffer.baseAddress!, buffer.count)
        }
        guard let output else { throw ParakeetError.inferenceFailed }
        defer { freeTextFn(output) }
        return String(cString: output)
    }

    func unload(_ model: ParakeetModel) {
        engineLock.lock()
        defer { engineLock.unlock() }
        if let engine = engines.removeValue(forKey: model.id) { destroyFn?(engine) }
    }

    deinit {
        guard let destroyFn else { return }
        for engine in engines.values { destroyFn(engine) }
        if let handle { dlclose(handle) }
    }
}

final class DownloadProgressValue: @unchecked Sendable {
    private let lock = NSLock()
    private var fraction: Double = 0
    func update(_ value: Double) { lock.lock(); fraction = value; lock.unlock() }
    var value: Double { lock.lock(); defer { lock.unlock() }; return fraction }
}

enum ParakeetError: Error, LocalizedError {
    case unavailable, downloadFailed, notInstalled, inferenceFailed
    var errorDescription: String? {
        switch self {
        case .unavailable: "Parakeet è disponibile solo su macOS 14 o successivo e richiede il componente incluso nell'app."
        case .downloadFailed: "Download Parakeet non riuscito o annullato. Verifica la connessione e riprova."
        case .notInstalled: "Il modello Parakeet selezionato non è installato. Installalo esplicitamente dalla sezione Modelli."
        case .inferenceFailed: "Trascrizione Parakeet non riuscita."
        }
    }
}
