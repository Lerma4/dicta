import Combine
import Foundation

@MainActor
final class ModelManagement: ObservableObject {
    @Published private(set) var entries: [ModelCatalogEntry] = []
    @Published private(set) var downloadingModelID: String?
    @Published private(set) var status = "Scegli un modello e installalo esplicitamente."
    @Published private(set) var error: String?
    @Published var selectedModelID: String {
        didSet { UserDefaults.standard.set(selectedModelID, forKey: Self.selectionKey) }
    }

    private let catalog = ModelCatalogService()
    private var downloadTask: Task<Void, Never>?
    private static let selectionKey = "dicta.native.selectedModel"
    private var modelsDirectory: URL {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("app.dicta.desktop", isDirectory: true)
            .appendingPathComponent("models", isDirectory: true)
    }

    init() {
        let saved = UserDefaults.standard.string(forKey: Self.selectionKey)
        selectedModelID = ModelCatalogService.models.contains(where: { $0.id == saved }) ? saved! : "tiny"
        refresh()
    }

    func refresh() {
        do {
            entries = try catalog.listModels(in: modelsDirectory)
            if entries.contains(where: { $0.model.id == selectedModelID && $0.installed }) {
                status = "Modello selezionato: \(entries.first(where: { $0.model.id == selectedModelID })!.model.name)."
            } else {
                status = "Installa il modello selezionato per iniziare a dettare."
            }
            error = nil
        } catch {
            self.error = "Impossibile controllare i modelli: \(error.localizedDescription)"
        }
    }

    func install(_ model: CatalogModel) {
        guard downloadTask == nil else { return }
        downloadingModelID = model.id
        status = "Download di \(model.name) in corso. Il file viene verificato prima dell'installazione…"
        error = nil
        downloadTask = Task {
            defer { downloadTask = nil; downloadingModelID = nil }
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

    func cancelDownload() {
        downloadTask?.cancel()
    }

    func isInstalled(_ id: String) -> Bool {
        entries.first(where: { $0.model.id == id })?.installed == true
    }
}
