import Combine
import Foundation

@MainActor
final class ModelManagement: ObservableObject {
    @Published private(set) var entries: [ModelCatalogEntry] = []
    @Published private(set) var downloadingModelID: String?
    @Published private(set) var downloadProgress: Double?
    @Published private(set) var status = "Scegli un modello e installalo esplicitamente."
    @Published private(set) var error: String?
    @Published var selectedModelID: String {
        didSet { UserDefaults.standard.set(selectedModelID, forKey: Self.selectionKey) }
    }

    private let catalog = ModelCatalogService()
    private var downloadTask: Task<Void, Never>?
    private var activeParakeetProgress: DownloadProgressValue?
    private var progressTimer: Timer?
    private var cancellationRequested = false
    private static let selectionKey = "dicta.native.selectedModel"
    private var modelsDirectory: URL {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("app.dicta.desktop", isDirectory: true)
            .appendingPathComponent("models", isDirectory: true)
    }

    init() {
        let saved = UserDefaults.standard.string(forKey: Self.selectionKey)
        if let saved, ModelCatalogService.models.contains(where: { $0.id == saved }) || ParakeetModel.all.contains(where: { $0.id == saved }) {
            selectedModelID = saved
        } else {
            selectedModelID = ProcessInfo.processInfo.isOperatingSystemAtLeast(OperatingSystemVersion(majorVersion: 14, minorVersion: 0, patchVersion: 0)) ? "parakeet-ultra" : "tiny"
        }
        refresh()
    }

    func refresh() {
        do {
            entries = try catalog.listModels(in: modelsDirectory)
            if isInstalled(selectedModelID) {
                status = "Modello selezionato: \(modelName(selectedModelID))."
            } else {
                status = "Installa esplicitamente il modello selezionato per iniziare a dettare."
            }
            error = nil
        } catch {
            self.error = "Impossibile controllare i modelli: \(error.localizedDescription)"
        }
    }

    func install(_ model: CatalogModel) {
        guard downloadTask == nil else { return }
        downloadingModelID = model.id
        downloadProgress = nil
        status = "Download di \(model.name) in corso. Il file viene verificato prima dell'installazione…"
        error = nil
        downloadTask = Task {
            defer { downloadTask = nil; downloadingModelID = nil; downloadProgress = nil }
            do {
                _ = try await catalog.download(modelID: model.id, to: modelsDirectory)
                refresh()
                status = "\(model.name) installato e verificato."
            } catch is CancellationError {
                status = "Download annullato. Nessun modello parziale è stato installato."
                refresh()
            } catch {
                refresh()
                self.error = "Download di \(model.name) non riuscito: \(error.localizedDescription)"
                status = "Download non riuscito. Puoi riprovare."
            }
        }
    }

    func install(_ model: ParakeetModel) {
        guard downloadTask == nil else { return }
        guard #available(macOS 14, *), ProcessInfo.processInfo.isOperatingSystemAtLeast(OperatingSystemVersion(majorVersion: model.requiredOS, minorVersion: 0, patchVersion: 0)) else {
            error = "\(model.name) richiede macOS \(model.requiredOS) o successivo."
            return
        }
        downloadingModelID = model.id
        cancellationRequested = false
        let progress = DownloadProgressValue()
        activeParakeetProgress = progress
        downloadProgress = 0
        progressTimer?.invalidate()
        progressTimer = Timer.scheduledTimer(withTimeInterval: 0.25, repeats: true) { [weak self] _ in
            Task { @MainActor [weak self] in self?.downloadProgress = self?.activeParakeetProgress?.value }
        }
        status = "Download di \(model.name) esplicito in corso…"
        error = nil
        downloadTask = Task { [weak self] in
            do {
                try await Task.detached {
                    try ParakeetBridge.shared.download(model, progress: progress)
                }.value
                self?.refresh()
                self?.status = "\(model.name) installato e pronto."
            } catch {
                self?.refresh()
                if self?.cancellationRequested == true {
                    try? FileManager.default.removeItem(at: ParakeetBridge.cacheDirectory(for: model))
                    self?.refresh()
                    self?.status = "Download annullato. I file parziali sono stati rimossi."
                } else {
                    self?.error = "Download di \(model.name) non riuscito: \(error.localizedDescription)"
                    self?.status = "Download non riuscito. Puoi riprovare."
                }
            }
            self?.progressTimer?.invalidate()
            self?.progressTimer = nil
            self?.activeParakeetProgress = nil
            self?.downloadingModelID = nil
            self?.downloadProgress = nil
            self?.downloadTask = nil
        }
    }

    func delete(_ model: CatalogModel) {
        guard model.id != selectedModelID, downloadingModelID != model.id else { return }
        do {
            try catalog.delete(modelID: model.id, from: modelsDirectory)
            refresh()
            status = "\(model.name) eliminato."
        } catch {
            refresh()
            self.error = "Impossibile eliminare \(model.name): \(error.localizedDescription)"
        }
    }

    func delete(_ model: ParakeetModel) {
        guard model.id != selectedModelID, downloadingModelID != model.id else { return }
        ParakeetBridge.shared.unload(model)
        do {
            try FileManager.default.removeItem(at: ParakeetBridge.cacheDirectory(for: model))
            refresh()
            status = "\(model.name) e la sua cache sono stati eliminati."
        } catch {
            refresh()
            self.error = "Impossibile eliminare \(model.name): \(error.localizedDescription)"
        }
    }

    func cancelDownload() {
        if let id = downloadingModelID, ParakeetModel.all.contains(where: { $0.id == id }) {
            cancellationRequested = true
            ParakeetBridge.shared.cancelDownload()
        } else {
            downloadTask?.cancel()
        }
    }

    func isInstalled(_ id: String) -> Bool {
        if ModelCatalogService.models.contains(where: { $0.id == id }) {
            return entries.first(where: { $0.model.id == id })?.installed == true
        }
        guard let model = ParakeetModel.all.first(where: { $0.id == id }), #available(macOS 14, *) else { return false }
        return ParakeetBridge.shared.isInstalled(model)
    }

    func isAvailable(_ model: ParakeetModel) -> Bool {
        ProcessInfo.processInfo.isOperatingSystemAtLeast(OperatingSystemVersion(majorVersion: model.requiredOS, minorVersion: 0, patchVersion: 0))
    }

    func installedSize(_ model: ParakeetModel) -> Int64 {
        let root = ParakeetBridge.cacheDirectory(for: model)
        guard let enumerator = FileManager.default.enumerator(at: root, includingPropertiesForKeys: [.fileSizeKey], options: [.skipsHiddenFiles]) else { return 0 }
        return enumerator.compactMap { try? ($0 as? URL)?.resourceValues(forKeys: [.fileSizeKey]).fileSize }
            .reduce(Int64(0)) { $0 + Int64($1) }
    }

    func modelName(_ id: String) -> String {
        ParakeetModel.all.first(where: { $0.id == id })?.name ?? ModelCatalogService.models.first(where: { $0.id == id })?.name ?? id
    }

}
