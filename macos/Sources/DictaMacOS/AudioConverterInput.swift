import AVFoundation
import Foundation

// AVAudioConverter consumes this buffer synchronously during convert; the lock protects repeated callback requests.
final class AudioConverterInput: @unchecked Sendable {
    private let lock = NSLock()
    private var buffer: AVAudioPCMBuffer?

    init(_ buffer: AVAudioPCMBuffer) { self.buffer = buffer }

    func take() -> AVAudioPCMBuffer? {
        lock.lock()
        defer { lock.unlock() }
        let result = buffer
        buffer = nil
        return result
    }
}
