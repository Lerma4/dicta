import Foundation

enum DictaSection: String, CaseIterable, Identifiable {
    case overview
    case statistics
    case history
    case models
    case settings

    var id: String { rawValue }

    var title: String {
        switch self {
        case .overview: "Panoramica"
        case .statistics: "Statistiche"
        case .history: "Cronologia"
        case .models: "Modelli"
        case .settings: "Impostazioni"
        }
    }

    var symbol: String {
        switch self {
        case .overview: "house"
        case .statistics: "chart.bar"
        case .history: "clock"
        case .models: "waveform"
        case .settings: "gearshape"
        }
    }

    var availability: String {
        switch self {
        case .overview:
            "Dettatura locale in memoria, con permessi espliciti e inserimento solo nel campo accessibile verificato dell'app in primo piano. Il testo non viene copiato automaticamente negli appunti."
        case .statistics:
            "Statistiche, grafici, filtri ed esportazione non sono ancora disponibili nella versione macOS nativa. Nessun dato dimostrativo viene mostrato."
        case .history:
            "Cronologia, ricerca, conservazione dei testi, cancellazione ed esportazione non sono ancora disponibili nella versione macOS nativa."
        case .models:
            "Installa su richiesta modelli Whisper verificati, controlla lo stato e seleziona il modello da usare. I file esistenti non vengono sovrascritti o rimossi."
        case .settings:
            "Controlla e richiedi i permessi microfono e Accessibilità. La scorciatoia globale è ⌘⇧Spazio."
        }
    }
}
