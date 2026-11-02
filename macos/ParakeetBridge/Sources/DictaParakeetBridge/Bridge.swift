import Foundation
import FluidAudio

public typealias ProgressCallback = @convention(c) (Double, UnsafeMutableRawPointer?) -> Void
private struct CallbackContext: @unchecked Sendable {
    let callback: ProgressCallback?
    let pointer: UnsafeMutableRawPointer?
}
private final class ResultBox<T>: @unchecked Sendable {
    var value: Result<T, Error>?
    let lock = NSLock()
    func set(_ value: Result<T, Error>) { lock.lock(); self.value = value; lock.unlock() }
    func get() -> Result<T, Error>? { lock.lock(); defer { lock.unlock() }; return value }
}
private final class DownloadTaskState: @unchecked Sendable {
    private let lock = NSLock()
    private var task: Task<Void, Never>?
    func set(_ task: Task<Void, Never>) { lock.lock(); self.task = task; lock.unlock() }
    func cancel() { lock.lock(); let task = self.task; lock.unlock(); task?.cancel() }
}
private let downloadTaskState = DownloadTaskState()
private final class ParakeetHandle: @unchecked Sendable {
    let manager: AsrManager
    init(models: AsrModels) { manager = AsrManager(models: models) }
    func transcribe(_ samples: [Float]) async throws -> String {
        var state = try TdtDecoderState()
        return try await manager.transcribe(samples, decoderState: &state).text
    }
}

private func version(_ id: Int32) -> AsrModelVersion? {
    switch id { case 0: .ultra; case 1: .v3; case 2: .redux; default: nil }
}

@_cdecl("dicta_parakeet_download")
public func dictaParakeetDownload(_ id: Int32, _ path: UnsafePointer<CChar>, _ callback: ProgressCallback?, _ context: UnsafeMutableRawPointer?) -> Int32 {
    guard let version = version(id) else { return -1 }
    let directory = URL(fileURLWithPath: String(cString: path), isDirectory: true)
    let result = ResultBox<URL>()
    let callbackContext = CallbackContext(callback: callback, pointer: context)
    let semaphore = DispatchSemaphore(value: 0)
    let task = Task.detached {
        do {
            let location = try await AsrModels.download(to: directory, version: version, progressHandler: { progress in
                callbackContext.callback?(progress.fractionCompleted, callbackContext.pointer)
            })
            result.set(.success(location))
        } catch { result.set(.failure(error)) }
        semaphore.signal()
    }
    downloadTaskState.set(task)
    semaphore.wait()
    downloadTaskState.set(Task {})
    return (try? result.get()?.get()) == nil ? -2 : 0
}

@_cdecl("dicta_parakeet_cancel_download")
public func dictaParakeetCancelDownload() { downloadTaskState.cancel() }

@_cdecl("dicta_parakeet_is_installed")
public func dictaParakeetIsInstalled(_ id: Int32, _ path: UnsafePointer<CChar>) -> Int32 {
    guard let version = version(id) else { return 0 }
    return AsrModels.modelsExist(at: URL(fileURLWithPath: String(cString: path), isDirectory: true), version: version) ? 1 : 0
}

@_cdecl("dicta_parakeet_create")
public func dictaParakeetCreate(_ id: Int32, _ path: UnsafePointer<CChar>) -> UnsafeMutableRawPointer? {
    guard let version = version(id) else { return nil }
    let directory = URL(fileURLWithPath: String(cString: path), isDirectory: true)
    guard AsrModels.modelsExist(at: directory, version: version) else { return nil }
    let result = ResultBox<ParakeetHandle>()
    let semaphore = DispatchSemaphore(value: 0)
    Task.detached {
        do { result.set(.success(ParakeetHandle(models: try await AsrModels.load(from: directory, version: version)))) }
        catch { result.set(.failure(error)) }
        semaphore.signal()
    }
    semaphore.wait()
    guard case .success(let handle)? = result.get() else { return nil }
    return Unmanaged.passRetained(handle).toOpaque()
}

@_cdecl("dicta_parakeet_transcribe")
public func dictaParakeetTranscribe(_ opaque: UnsafeMutableRawPointer, _ samples: UnsafePointer<Float>, _ count: Int) -> UnsafeMutablePointer<CChar>? {
    guard count > 0 else { return nil }
    let handle = Unmanaged<ParakeetHandle>.fromOpaque(opaque).takeUnretainedValue()
    let buffer = Array(UnsafeBufferPointer(start: samples, count: count))
    let result = ResultBox<String>()
    let semaphore = DispatchSemaphore(value: 0)
    Task.detached {
        do { result.set(.success(try await handle.transcribe(buffer))) }
        catch { result.set(.failure(error)) }
        semaphore.signal()
    }
    semaphore.wait()
    guard case .success(let text)? = result.get() else { return nil }
    return strdup(text)
}

@_cdecl("dicta_parakeet_destroy")
public func dictaParakeetDestroy(_ opaque: UnsafeMutableRawPointer) {
    Unmanaged<ParakeetHandle>.fromOpaque(opaque).release()
}

@_cdecl("dicta_parakeet_free_text")
public func dictaParakeetFreeText(_ text: UnsafeMutablePointer<CChar>) { free(text) }
