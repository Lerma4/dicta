import SwiftUI
import AppKit

struct StatisticsView: View {
    @ObservedObject var service: NativeDictationService
    @State private var totals: DictationTotals?
    @State private var error: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Text("Statistiche").font(.largeTitle.weight(.semibold))
            Text("Dati locali delle dettature completate con la versione nativa.").foregroundStyle(.secondary)
            if let error { Label(error, systemImage: "exclamationmark.triangle").foregroundStyle(.orange) }
            if let totals {
                if totals.count == 0 { Text("Nessuna dettatura completata.").foregroundStyle(.secondary) }
                LabeledContent("Dettature", value: "\(totals.count)")
                LabeledContent("Parole", value: "\(totals.words)")
                LabeledContent("Tempo registrato", value: Duration.seconds(totals.duration).formatted(.units(allowed: [.hours, .minutes, .seconds], width: .abbreviated)))
            }
            Spacer()
        }
        .padding(36).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onAppear(perform: reload)
        .onChange(of: service.dataVersion) { _ in reload() }
    }

    private func reload() {
        do { totals = try SessionStore().totals(); error = nil }
        catch { self.error = error.localizedDescription }
    }
}

struct HistoryView: View {
    @ObservedObject var service: NativeDictationService
    @AppStorage("dicta.native.historyEnabled") private var enabled = false
    @State private var records: [DictationRecord] = []
    @State private var error: String?
    @State private var confirmClear = false

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Cronologia").font(.largeTitle.weight(.semibold))
            Toggle("Salva le prossime trascrizioni su questo Mac", isOn: $enabled)
            Text("Disattivata per impostazione iniziale. I testi salvati restano locali e non sono cifrati; disattivare l’opzione non elimina i testi già salvati.")
                .foregroundStyle(.secondary).font(.callout)
            if let error { Label(error, systemImage: "exclamationmark.triangle").foregroundStyle(.orange) }
            if records.isEmpty {
                Text("Nessuna trascrizione salvata.").foregroundStyle(.secondary)
                Spacer()
            } else {
                List(records) { record in
                    VStack(alignment: .leading, spacing: 6) {
                        Text(record.date, style: .date) + Text(" · ") + Text(record.date, style: .time)
                        Text(record.text ?? "").textSelection(.enabled)
                        Button("Copia") {
                            NSPasteboard.general.clearContents()
                            if !NSPasteboard.general.setString(record.text ?? "", forType: .string) { error = "Impossibile copiare il testo." }
                        }
                    }.padding(.vertical, 6)
                }
                Button("Elimina tutte le trascrizioni…", role: .destructive) { confirmClear = true }
                    .confirmationDialog("Eliminare tutte le trascrizioni salvate? Le statistiche restano disponibili.", isPresented: $confirmClear) {
                        Button("Elimina trascrizioni", role: .destructive) {
                            do { try SessionStore().clearTranscripts(); reload() } catch { self.error = error.localizedDescription }
                        }
                    }
            }
        }
        .padding(36).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onAppear(perform: reload)
        .onChange(of: service.dataVersion) { _ in reload() }
    }

    private func reload() {
        do { records = try SessionStore().history(); error = nil }
        catch { self.error = error.localizedDescription }
    }
}
