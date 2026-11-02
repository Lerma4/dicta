import SwiftUI
import AppKit
import AVFoundation
import ApplicationServices

@main
struct DictaMacOSApp: App {
    var body: some Scene {
        WindowGroup("Dicta", id: "main") {
            DictaWindow()
                .frame(minWidth: 760, minHeight: 500)
        }
        .defaultSize(width: 980, height: 680)

        MenuBarExtra("Dicta", systemImage: "waveform") {
            MenuBarActions()
        }
        .menuBarExtraStyle(.menu)
    }
}

private struct MenuBarActions: View {
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        Button("Apri Dicta") { openWindow(id: "main") }
        Divider()
        Button("Esci da Dicta") { NSApplication.shared.terminate(nil) }
    }
}

private struct DictaWindow: View {
    @State private var selection: DictaSection? = .overview
    @StateObject private var dictation = NativeDictationService()
    @StateObject private var models = ModelManagement()
    @State private var shortcut = GlobalShortcut()
    @State private var shortcutMessage: String?
    @AppStorage("dicta.native.language") private var language = "it"

    var body: some View {
        NavigationSplitView {
            List(DictaSection.allCases, selection: $selection) { section in
                NavigationLink(value: section) {
                    Label(section.title, systemImage: section.symbol)
                }
            }
            .listStyle(.sidebar)
            .navigationTitle("Dicta")
            .safeAreaInset(edge: .bottom) {
                Label("Funzioni native in migrazione", systemImage: "info.circle")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .padding()
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
        } detail: {
            if let selection {
                if selection == .overview {
                    DictationView(service: dictation, models: models, shortcutMessage: shortcutMessage, language: language)
                } else if selection == .statistics {
                    StatisticsView(service: dictation)
                } else if selection == .history {
                    HistoryView(service: dictation)
                } else if selection == .models {
                    ModelsView(models: models)
                } else if selection == .settings {
                    PermissionsView()
                } else {
                    SectionView(section: selection)
                }
            } else {
                VStack(spacing: 8) {
                    Image(systemName: "sidebar.left").font(.largeTitle).foregroundStyle(.secondary)
                    Text("Seleziona una sezione").font(.title3)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .toolbar {
            ToolbarItem(placement: .principal) {
                Label("Dettatura macOS", systemImage: "waveform")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
        }
        .preferredColorScheme(nil)
        .onAppear {
            let status = shortcut.register { dictation.toggle(modelID: models.selectedModelID, language: language) }
            shortcutMessage = status == noErr ? nil : "Scorciatoia ⌘⇧Spazio non disponibile (codice \(status)); un'altra app potrebbe usarla. Libera la scorciatoia nelle impostazioni dell'altra app e riapri Dicta."
        }
        .onDisappear { shortcut.unregister() }
    }
}

private struct DictationView: View {
    @ObservedObject var service: NativeDictationService
    @ObservedObject var models: ModelManagement
    let shortcutMessage: String?
    let language: String
    @State private var inputName: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 24) {
            Text("Dettatura macOS").font(.largeTitle.weight(.semibold))
            Text("La scorciatoia globale ⌘⇧Spazio avvia e termina la dettatura. L’audio resta in memoria; il salvataggio delle trascrizioni è facoltativo.")
                .foregroundStyle(.secondary).frame(maxWidth: 600, alignment: .leading)
            if let shortcutMessage {
                Label(shortcutMessage, systemImage: "keyboard.badge.exclamationmark")
                    .font(.callout).foregroundStyle(.orange).textSelection(.enabled)
            }
            if !models.isInstalled(models.selectedModelID) {
                Label("Installa un modello nella sezione Modelli per iniziare.", systemImage: "arrow.down.circle")
                    .foregroundStyle(.orange)
            }
            Button { service.toggle(modelID: models.selectedModelID, language: language) } label: {
                Label(service.phase == .recording ? "Termina dettatura" : "Inizia dettatura", systemImage: service.phase == .recording ? "stop.fill" : "mic.fill")
                    .font(.title3)
            }
            .disabled((service.phase.isActive && service.phase != .recording) || service.recoveryPending || !models.isInstalled(models.selectedModelID))
            if service.recoveryPending {
                Text(service.recoveryText ?? "").textSelection(.enabled).lineLimit(5)
                HStack {
                    Button("Copia testo negli appunti", action: service.copyRecovery)
                    Button("Scarta risultato", role: .destructive, action: service.discardRecovery)
                }
            }
            if let error = service.storageError {
                Label(error, systemImage: "exclamationmark.triangle").foregroundStyle(.orange).textSelection(.enabled)
            }
            Label(service.message, systemImage: service.phase == .error ? "exclamationmark.triangle" : "waveform")
                .foregroundStyle(service.phase == .error ? .orange : .secondary).textSelection(.enabled)
            if service.phase == .recording { Text("\(Int(service.elapsed)) s / 300 s").monospacedDigit() }
            VStack(alignment: .leading, spacing: 6) {
                Label("Ingresso di sistema: \(inputName ?? "Non disponibile")", systemImage: "mic")
                Text("Modello: \(ModelCatalogService.models.first(where: { $0.id == models.selectedModelID })?.name ?? "Non selezionato")")
            }
            .font(.callout).foregroundStyle(.secondary)
        }
        .padding(36).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onAppear { inputName = InputDevice.defaultName() }
        .onReceive(NotificationCenter.default.publisher(for: NSApplication.didBecomeActiveNotification)) { _ in
            inputName = InputDevice.defaultName()
        }
    }
}

private struct ModelsView: View {
    @ObservedObject var models: ModelManagement

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Text("Modelli vocali").font(.largeTitle.weight(.semibold))
            Text("I modelli vengono scaricati solo su richiesta, verificati tramite SHA-256 e conservati in Application Support. Nessun modello viene eliminato.")
                .foregroundStyle(.secondary).frame(maxWidth: 650, alignment: .leading)
            if let error = models.error {
                Label(error, systemImage: "exclamationmark.triangle.fill").foregroundStyle(.orange).textSelection(.enabled)
            }
            Text(models.status).foregroundStyle(.secondary)
            if models.downloadingModelID != nil {
                ProgressView("Download e verifica in corso…")
                Button("Annulla download", role: .cancel, action: models.cancelDownload)
            }
            List {
                ForEach(ModelCatalogService.models, id: \.id) { model in
                    HStack(spacing: 14) {
                        VStack(alignment: .leading, spacing: 5) {
                            Text(model.name).font(.headline)
                            Text(model.description).foregroundStyle(.secondary)
                            Text(String(format: "%.0f MB", Double(model.sizeBytes) / 1_000_000))
                                .font(.caption).foregroundStyle(.secondary)
                            if models.isInstalled(model.id) {
                                Label("Installato e verificato", systemImage: "checkmark.seal.fill")
                                    .font(.caption).foregroundStyle(.green)
                            }
                        }
                        Spacer()
                        if models.isInstalled(model.id) {
                            Button(models.selectedModelID == model.id ? "Selezionato" : "Seleziona") {
                                models.selectedModelID = model.id
                                models.refresh()
                            }
                            .disabled(models.selectedModelID == model.id)
                        } else {
                            Button("Installa") { models.install(model) }
                                .disabled(models.downloadingModelID != nil)
                        }
                    }.padding(.vertical, 6)
                }
            }
            .listStyle(.inset)
        }
        .padding(28)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .navigationTitle("Modelli")
        .onAppear(perform: models.refresh)
    }
}

private struct PermissionsView: View {
    @AppStorage("dicta.native.language") private var language = "it"
    @State private var microphone = AVCaptureDevice.authorizationStatus(for: .audio)
    @State private var accessibility = AXIsProcessTrusted()

    var body: some View {
        VStack(alignment: .leading, spacing: 22) {
            Text("Impostazioni").font(.largeTitle.weight(.semibold))
            Picker("Lingua della dettatura", selection: $language) {
                Text("Italiano").tag("it")
                Text("Rilevamento automatico").tag("auto")
                Text("Inglese").tag("en")
                Text("Francese").tag("fr")
                Text("Spagnolo").tag("es")
                Text("Tedesco").tag("de")
                Text("Portoghese").tag("pt")
            }.frame(maxWidth: 360)
            Text("La lingua scelta viene usata dal modello locale; la selezione non garantisce una trascrizione esatta.")
                .font(.caption).foregroundStyle(.secondary)
            Text("Permessi").font(.title2.weight(.semibold))
            Text("Dicta richiede il microfono per registrare e Accessibilità per inserire il testo nel campo verificato dell'app in primo piano.")
                .foregroundStyle(.secondary).frame(maxWidth: 650, alignment: .leading)
            Label(microphoneLabel, systemImage: microphone == .authorized ? "checkmark.circle.fill" : "mic")
            Button("Richiedi accesso al microfono") {
                AVCaptureDevice.requestAccess(for: .audio) { _ in
                    DispatchQueue.main.async { microphone = AVCaptureDevice.authorizationStatus(for: .audio) }
                }
            }.disabled(microphone == .authorized)
            Label(accessibility ? "Accessibilità consentita" : "Accessibilità non consentita", systemImage: accessibility ? "checkmark.circle.fill" : "hand.raised")
            Button("Richiedi accesso ad Accessibilità") {
                let options = ["AXTrustedCheckOptionPrompt": true] as CFDictionary
                _ = AXIsProcessTrustedWithOptions(options)
                accessibility = AXIsProcessTrusted()
            }.disabled(accessibility)
            Button("Apri Impostazioni di Sistema") {
                NSWorkspace.shared.open(URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")!)
            }
            Text("Dopo aver modificato i permessi, aggiorna questa finestra o riapri Dicta per vedere lo stato aggiornato.")
                .font(.caption).foregroundStyle(.secondary)
        }
        .padding(36).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onAppear {
            microphone = AVCaptureDevice.authorizationStatus(for: .audio)
            accessibility = AXIsProcessTrusted()
        }
    }

    private var microphoneLabel: String {
        switch microphone {
        case .authorized: "Microfono consentito"
        case .denied: "Microfono negato: abilitalo in Impostazioni di Sistema"
        case .restricted: "Microfono limitato da questo sistema"
        case .notDetermined: "Microfono: consenso non ancora richiesto"
        @unknown default: "Stato microfono non riconosciuto"
        }
    }
}

private struct SectionView: View {
    let section: DictaSection

    var body: some View {
        VStack(alignment: .leading, spacing: 24) {
            VStack(alignment: .leading, spacing: 8) {
                Text(section.title)
                    .font(.largeTitle.weight(.semibold))
                Text("Dicta per macOS")
                    .font(.title3)
                    .foregroundStyle(.secondary)
            }

            VStack(alignment: .leading, spacing: 14) {
                Label("Non ancora disponibile", systemImage: "hourglass")
                    .font(.title2.weight(.medium))
                Text(section.availability)
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: 520, alignment: .leading)
            }
            .padding(22)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(.quaternary, in: RoundedRectangle(cornerRadius: 12))

            Label("Questa app non è collegata al motore Dicta e non accede né modifica dati utente.", systemImage: "lock.shield")
                .font(.callout)
                .foregroundStyle(.secondary)
        }
        .padding(36)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .navigationTitle(section.title)
    }
}
