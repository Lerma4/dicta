import AVFoundation
import AppKit
import Combine
import Foundation

@MainActor
final class NativeDictationService: ObservableObject {
    @Published private(set) var phase: DictationPhase = .idle
    @Published private(set) var message = "Premi ⌘⇧Spazio per dettare in un'altra app."
    @Published private(set) var recoveryText: String?
    @Published private(set) var elapsed: TimeInterval = 0
    @Published private(set) var dataVersion = 0
    @Published private(set) var storageError: String?

    private let recorder = InMemoryAudioRecorder()
    private var target: AccessibilityTarget?
    private var startedAt: Date?
    private var timer: Timer?
    private var sessionLanguage = "it"

    var recoveryPending: Bool { recoveryText != nil }

    func toggle(modelID: String = "tiny", language: String = "it") {
        switch phase {
        case .recording: stopAndTranscribe(modelID: modelID)
        case .idle, .completed, .cancelled, .error:
            guard recoveryText == nil else { message = "Recupera il testo precedente prima di iniziare."; return }
            sessionLanguage = language
            start()
        default: break
        }
    }

    func start() {
        guard recoveryText == nil else { message = "Recupera il testo precedente prima di iniziare."; return }
        let isTerminal = AccessibilityTarget.isFrontmostTerminal()
        let captured = AccessibilityTarget.captureFocusedExternalTarget()
        guard captured != nil || (isTerminal && AccessibilityTarget.allowsTerminalRecovery()) else {
            message = "Seleziona un campo di testo scrivibile in un'altra app e concedi Accessibilità."; phase = .error; return
        }
        phase = .preparing
        AVCaptureDevice.requestAccess(for: .audio) { [weak self] granted in
            Task { @MainActor in
                guard let self else { return }
                guard granted else { self.phase = .error; self.message = "Accesso al microfono negato. Abilitalo in Impostazioni di Sistema."; return }
                do {
                    try self.recorder.start()
                    self.target = captured
                    self.startedAt = Date()
                    self.phase = .recording
                    self.message = captured == nil
                        ? "In ascolto… il testo sarà disponibile da copiare, non inserito automaticamente nel terminale."
                        : "In ascolto… premi ⌘⇧Spazio per terminare (massimo 5 minuti)."
                    self.timer = Timer.scheduledTimer(withTimeInterval: 0.25, repeats: true) { [weak self] _ in
                        Task { @MainActor in
                            guard let self, let began = self.startedAt else { return }
                            self.elapsed = Date().timeIntervalSince(began)
                            if self.elapsed >= 300 { self.stopAndTranscribe() }
                        }
                    }
                } catch { self.phase = .error; self.message = error.localizedDescription }
            }
        }
    }

    func cancel() {
        guard phase == .recording else { return }
        timer?.invalidate(); timer = nil
        _ = recorder.stop(); target = nil; startedAt = nil; elapsed = 0
        phase = .cancelled; message = "Dettatura annullata."
    }

    func stopAndTranscribe(modelID: String = "tiny") {
        guard phase == .recording else { return }
        timer?.invalidate(); timer = nil
        let samples = recorder.stop()
        let destination = target
        let duration = startedAt.map { Date().timeIntervalSince($0) } ?? 0
        let language = sessionLanguage
        let keepText = UserDefaults.standard.bool(forKey: "dicta.native.historyEnabled")
        target = nil; startedAt = nil
        guard !samples.isEmpty else { phase = .error; message = "Nessun audio acquisito."; return }
        phase = .processing; message = "Trascrizione locale in corso…"
        Task.detached(priority: .userInitiated) { [weak self] in
            do {
                let model = try WhisperEngine.installedModelURL(modelID: modelID)
                let engine = try WhisperEngine(modelURL: model)
                let text = try engine.transcribe(pcm16kMono: samples, language: language).trimmingCharacters(in: .whitespacesAndNewlines)
                guard !text.isEmpty else { throw WhisperEngine.EngineError.inferenceFailed }
                await MainActor.run { [weak self] in self?.finish(text: text, target: destination, duration: duration, keepText: keepText) }
            } catch {
                await MainActor.run { [weak self] in self?.fail(error) }
            }
        }
    }

    func copyRecovery() {
        guard let recoveryText else { return }
        NSPasteboard.general.clearContents()
        guard NSPasteboard.general.setString(recoveryText, forType: .string) else {
            message = "Impossibile copiare il testo negli appunti. Il testo è ancora disponibile per riprovare."
            return
        }
        self.recoveryText = nil
        phase = .completed; message = "Testo copiato negli appunti su richiesta."
    }

    func discardRecovery() {
        guard recoveryText != nil else { return }
        recoveryText = nil
        phase = .idle
        message = "Risultato scartato. Puoi avviare una nuova dettatura."
    }

    private func finish(text: String, target: AccessibilityTarget?, duration: TimeInterval, keepText: Bool) {
        if TranscriptSafety.needsReview(text) {
            recoveryText = text
            phase = .completed
            message = "Whisper ha rilevato musica anziché parole: controlla il microfono e riprova. Il risultato non è stato inserito automaticamente."
            return
        }
        do {
            try SessionStore().save(text: text, duration: duration, keepText: keepText)
            storageError = nil
            dataVersion += 1
        } catch {
            storageError = "Impossibile salvare statistiche o cronologia: \(error.localizedDescription)"
        }
        guard let target, target.insert(text) else {
            recoveryText = text
            phase = .completed
            message = target == nil
                ? "Testo pronto: copialo esplicitamente e incollalo nel terminale dopo averlo controllato."
                : "Testo non inserito: destinazione cambiata o non sicura. Copialo esplicitamente per recuperarlo."
            return
        }
        phase = .completed; message = "Dettatura inserita nel campo verificato."
    }

    private func fail(_ error: Error) { phase = .error; message = "Trascrizione non riuscita: \(error.localizedDescription)" }
}

private final class InMemoryAudioRecorder {
    private let engine = AVAudioEngine()
    private let lock = NSLock()
    private var converter: AVAudioConverter?
    private var samples: [Float] = []
    private var failure: Error?
    private let maxSamples = 16_000 * 300

    func start() throws {
        let input = engine.inputNode
        let inputFormat = input.outputFormat(forBus: 0)
        guard inputFormat.channelCount > 0, let outputFormat = AVAudioFormat(commonFormat: .pcmFormatFloat32, sampleRate: 16_000, channels: 1, interleaved: false), let converter = AVAudioConverter(from: inputFormat, to: outputFormat) else {
            throw AudioCaptureError.unavailable
        }
        lock.lock(); samples.removeAll(keepingCapacity: true); failure = nil; self.converter = converter; lock.unlock()
        input.installTap(onBus: 0, bufferSize: 4096, format: inputFormat) { [weak self] buffer, _ in self?.consume(buffer, converter: converter, outputFormat: outputFormat) }
        do { try engine.start() } catch { input.removeTap(onBus: 0); throw error }
    }

    func stop() -> [Float] {
        if engine.isRunning { engine.stop() }
        engine.inputNode.removeTap(onBus: 0)
        lock.lock(); defer { lock.unlock() }
        let result = failure == nil ? samples : []
        samples.removeAll(keepingCapacity: false); converter = nil; failure = nil
        return result
    }

    private func consume(_ input: AVAudioPCMBuffer, converter: AVAudioConverter, outputFormat: AVAudioFormat) {
        let capacity = AVAudioFrameCount(ceil(Double(input.frameLength) * 16_000 / input.format.sampleRate) + 64)
        guard let output = AVAudioPCMBuffer(pcmFormat: outputFormat, frameCapacity: capacity) else { return }
        let source = AudioConverterInput(input)
        var conversionError: NSError?
        let status = converter.convert(to: output, error: &conversionError) { _, state in
            guard let buffer = source.take() else { state.pointee = .noDataNow; return nil }
            state.pointee = .haveData
            return buffer
        }
        guard status != .error, conversionError == nil, let channel = output.floatChannelData?[0] else {
            lock.lock(); failure = conversionError ?? AudioCaptureError.conversion; lock.unlock(); return
        }
        let count = Int(output.frameLength)
        lock.lock(); defer { lock.unlock() }
        let remaining = maxSamples - samples.count
        if remaining > 0 { samples.append(contentsOf: UnsafeBufferPointer(start: channel, count: min(count, remaining))) }
    }

    private enum AudioCaptureError: Error, LocalizedError {
        case unavailable, conversion
        var errorDescription: String? { self == .unavailable ? "Microfono o formato audio non disponibile." : "Conversione audio non riuscita." }
    }
}
