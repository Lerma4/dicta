# Specifica tecnica per Codex — App desktop di dettatura AI offline

> **Nome provvisorio:** Loquor (rinominabile; non vincolare percorsi, identificativi o API al brand).
> **Piattaforma prioritaria:** macOS, inizialmente Apple Silicon.
> **Destinazione futura:** Windows e Linux, senza riscrivere UI e logica di dominio.
> **Stato:** specifica di prodotto e implementazione. Codex deve **implementare**, **eseguire i test** e produrre un'app avviabile, non limitarsi a proporre un piano.

## 1. Obiettivo

Costruire un'app desktop **local-first e offline** per dettare testo a voce e inserirlo nell'applicazione attiva, ispirata all'esperienza di **Wispr Flow** (dettatura globale con overlay) e alle capacità di speech-to-text di **Whisper**. Le due cose sono distinte: Whisper è un motore di trascrizione, Wispr Flow un prodotto con molte funzioni applicative. Non si richiede una copia proprietaria o una parità non verificata con ogni versione di prodotti terzi, ma l'insieme di funzionalità dettagliate qui sotto.

Principi non negoziabili:
1. **Performance prima di tutto**: latenza, responsività, RAM, CPU/GPU, consumo a riposo e tempi di avvio sono requisiti primari e misurabili.
2. **Privacy**: trascrizione interamente in locale, niente upload di voce/testo, analytics esterni o telemetria abilitati di default; deve funzionare senza Internet dopo il download iniziale del modello.
3. **Qualità del codice**: modularità, componenti React piccoli, type safety, test automatici, documentazione essenziale, niente mega-componenti o business logic nella UI.
4. **Mac-first, cross-platform by design**: implementazioni native isolate dietro interfacce; evitare `cfg(target_os = "macos")` distribuiti nella logica applicativa.
5. **Affidabilità**: errori recuperabili, nessuna perdita silenziosa delle trascrizioni, niente inserimento del testo nella finestra sbagliata.

## 2. Stack obbligatorio / preferito

- **Desktop shell:** Tauri 2.x.
- **Frontend:** React + TypeScript (strict) + Vite; UI modulare, accessibile e responsive.
- **Core:** Rust stabile; separare domini, applicazione, infrastruttura e API Tauri.
- **Speech-to-text offline:** `whisper.cpp` come primo backend, con accelerazione Metal su macOS Apple Silicon; scegliere dopo uno spike tra sidecar nativo persistente e FFI/binding Rust in base a benchmark e semplicità. Non riavviare e ricaricare il modello a ogni dettatura.
- **Audio input:** Rust `cpal` oppure adattatore CoreAudio nativo se giustificato da latenza/affidabilità; audio PCM mono alla frequenza richiesta dal backend, resampling accurato.
- **VAD:** voice activity detection in locale, configurabile; non confondere VAD con trascrizione streaming.
- **Persistenza:** SQLite locale con migrazioni versionate e query parametrizzate; impostazioni semplici eventualmente nello store Tauri. Nessuna dipendenza da database remoto.
- **Grafici dashboard:** libreria React leggera, caricata lazy solo nella vista statistiche.
- **Tooling:** package manager scelto e fissato nel lockfile; formatting/linting TS, `cargo fmt`, `cargo clippy`, `cargo test`, frontend unit/integration testing, end-to-end quando praticabile.
- **Distribuzione:** bundle macOS firmabile e notarizzabile, senza assumere che siano già presenti tool, librerie o runtime sul computer dell'utente.

Codex deve controllare versioni correnti e compatibilità delle dipendenze prima di adottarle, fissarle con lockfile e documentare le decisioni. Non introdurre Python, Docker, server HTTP locali o cloud senza necessità comprovata.

## 3. Struttura del repository

Usare una struttura analoga alla seguente, modificabile se motivata:

```text
/
  src/
    app/                 # bootstrap, providers, routing, layout
    features/
      dictation/         # stato e UI di dettatura
      overlay/           # waveform e indicatori
      history/           # cronologia e ricerca
      dashboard/         # statistiche e grafici
      models/            # catalogo e gestione download
      settings/          # impostazioni, privacy e shortcuts
    components/ui/       # componenti riusabili, presentazionali
    hooks/
    services/            # adapter typed verso i comandi Tauri
    types/
    styles/
  src-tauri/
    src/
      commands/          # solo confine IPC, sottili
      domain/            # modelli, eventi, errori, interfacce
      application/       # use case, orchestrazione
      audio/             # capture, ring buffer, VAD
      inference/         # Whisper engine abstraction
      platform/          # macos, windows, linux, capabilities
      persistence/       # repositories e migrazioni
      metrics/           # statistiche e benchmark
      state/             # state machine dettatura
    capabilities/
    tauri.conf.json
  tests/
  scripts/
  docs/
    ARCHITECTURE.md
    PERFORMANCE.md
    PRIVACY.md
    PLATFORM_SUPPORT.md
    TESTING.md
```

Separare la finestra `main` dalla finestra `overlay`, con entrypoint React indipendenti, senza duplicare i componenti condivisi.

## 4. Flusso principale di dettatura

### 4.1 Happy path

1. L'app si avvia nella **menu bar** macOS ed è utilizzabile senza tenere aperta la finestra principale.
2. L'utente configura una **scorciatoia globale** (default proposto ma modificabile e rilevabile in caso di conflitto) e sceglie il comportamento **hold-to-talk** o **toggle**.
3. Alla pressione, senza rubare focus, appare una **barretta flottante** discreta nella parte bassa dello schermo attivo.
4. La barretta visualizza stato, waveform/live audio level, durata, microfono in uso, ed eventuali indicazioni di permessi/errore.
5. Si registra da microfono, il VAD distingue silenzio/voce, il motore produce testo. Se il backend/strategia supporta partial results affidabili, mostrarli; altrimenti evitare di promettere vero streaming: visualizzare stato di registrazione e trascrivere al termine o per chunk.
6. Al rilascio/seconda pressione si finalizza il testo e lo si **inserisce nell'applicazione precedentemente attiva**, conservando correttamente il focus; se l'inserimento non è possibile, copiare negli appunti e offrire un'azione esplicita di recupero.
7. L'overlay passa da registrazione a elaborazione a completato/errore e si richiude automaticamente dopo una breve conferma (configurabile).
8. Si salva una sessione di utilizzo per statistiche locali; la conservazione del contenuto trascritto è **opt-in** e disattivabile, con cancellazione completa.

### 4.2 Stato esplicito

Implementare una state machine testata, indicativamente:

`idle -> preparing -> recording -> processing -> inserting -> completed -> idle`

Con transizioni verso `error`, `cancelled` e timeout/cancellazione, evitando doppi eventi, race condition, release ripetuti, focus errato e sessioni orfane. Il backend è la fonte autorevole dello stato; la UI si aggiorna tramite eventi Tauri tipizzati.

### 4.3 Comportamenti e casi limite

- Scorciatoia premuta ripetutamente, tasti rilasciati fuori focus, cambio applicazione o monitor, schermo bloccato, sospensione/risveglio.
- Microfono negato/assente/occupato, dispositivo audio cambiato durante la registrazione, silenzio prolungato, audio vuoto, errori modello e memoria insufficiente.
- Il testo viene inserito una sola volta (idempotenza), mai automaticamente se il target sicuro non è più noto.
- Gestione esplicita di clipboard originale: non sovrascrivere in modo persistente i dati dell'utente senza consenso, verificare possibilità di ripristino e spiegare eventuali limiti.
- Terminazione applicazione durante la registrazione, recupero da crash, download interrotti.
- Lingua italiana prioritaria; riconoscimento multilingue/configurazione lingua e auto-detect quando supportati.

## 5. Overlay Mac

Realizzare una piccola finestra separata, con UI React, design curato, sobrio e moderno:
- senza bordo standard, sfondo arrotondato, possibilità di trasparenza e ombra;
- **always-on-top**, visibile senza attivare la finestra principale né rubare focus;
- posizionata per default in basso al centro dello **schermo corrente**, con margine dalla Dock, configurabile;
- su schermi multipli e Spaces deve comparire nel display/spazio corretto; degradazione documentata per fullscreen/finestre protette;
- stati visivi: inattivo (nascosto), registrazione, silenzio, elaborazione, risultato pronto, errore;
- waveform **basata sul livello audio reale**, animata a basso overhead; supporto `prefers-reduced-motion`;
- opzionali: timer, indicatore idioma, tasto annulla, quick settings.

Iniziare con `WebviewWindow` Tauri e le opzioni `decorations: false`, `transparent`, `alwaysOnTop`, `focus: false`, `visible: false`. **Valutare** un adapter macOS basato su `NSPanel` per comportamento non-attivante, Spaces e fullscreen, ma verificare manutenzione della libreria scelta e impatti su signing/notarizzazione/App Store. Non usare API private non documentate senza un'analisi esplicita; preferire API pubbliche.

La finestra può essere persistente ma nascosta quando non usata: evitare ricreazione continua e carichi a riposo.

## 6. Integrazione macOS e portabilità

Creare trait/interfacce isolate per:

```text
AudioCapture
SpeechEngine
GlobalShortcutManager
OverlayWindowManager
TextInjector
AccessibilityPermissionManager
ModelRepository
SessionRepository
MetricsRepository
```

### macOS (da implementare completamente nel primo rilascio)
- CoreAudio e/o `cpal` per input.
- Inserimento testo mediante API di Accessibilità/input supportate su macOS con controllo permessi. Evitare qualsiasi accesso alle password o ai campi sicuri; non inserire in contesti non identificati con certezza.
- Identificare e ricordare l'app/controllo target della dettatura senza portare focus sull'overlay.
- Gestire richieste e spiegazioni dei permessi **Microfono**, **Accessibilità** e gli eventuali permessi necessari agli eventi globali; controllare lo stato all'avvio e recuperare se revocati.
- Tray/menu bar, autostart opzionale, multiple display e monitor sleep.
- Apple Silicon/Metal per inferenza; fallback CPU se necessario.

### Windows e Linux (predisposizione ora, implementazione dopo)
- Non scrivere logica OS-specific nella UI o nel dominio.
- Creare adapter con firma stabile, capability detection, stub chiari e test dei contratti.
- Windows: WASAPI, tray, hotkey e API native per input.
- Linux: PipeWire/ALSA, X11 e Wayland **con limitazioni diverse**; non promettere global shortcut e iniezione testo uniforme su tutti i compositor Wayland.
- Verificare che il codice condiviso sia compilabile con build/check CI su Windows e Linux non appena possibile; le funzioni mancanti devono produrre errori espliciti, non crash o false promesse.

## 7. Funzionalità richieste

### P0 — Prima versione utilizzabile su Mac
- Setup e onboarding con controllo permessi e installazione modello locale.
- Download manuale o guidato di un modello compatibile, con progress, checksum/verifica, stima dimensioni, cancellazione e gestione errori; nessun download silenzioso.
- Dittatura globale con shortcut personalizzabili, modalità hold/toggle, overlay e inserimento testo.
- Supporto italiano, multilingua selezionabile, punteggiatura automatica prodotta dal modello, copia negli appunti come fallback.
- Stati chiari, error handling, modello mantenuto caldo tra sessioni finché ragionevole.
- Impostazioni: lingua, modello, dispositivo input, modalità shortcut, posizione overlay, suoni UI opzionali, avvio al login, privacy, retention.
- Statistiche locali e dashboard descritte al §8.
- Cronologia recente **facoltativa, opt-in**; ricerca/copia/eliminazione/export locale se abilitata.
- Esportazione e cancellazione dei dati e impostazioni.

### P1 — Miglioramenti successivi
- Trascrizione di file audio caricati (dropzone, formati principali supportati dal decoder scelto), con progress e cancellazione.
- Vocabolario personalizzato, sostituzioni/espansioni testuali e correzioni locali deterministiche; testare precedenze e sicurezza delle trasformazioni.
- Rimozione di intercalari e ripetizioni in maniera opzionale e conservativa, con preview/reversibilità.
- Lingue multiple e selezione più intelligente del modello.
- Eventuale trascrizione incrementale per chunk con gestione overlap/deduplicazione; non dichiarare streaming nativo senza supporto effettivo.

### P2 — Solo se ben motivate e compatibili con offline/privacy
- Comandi vocali personalizzati, dizionari contestuali, export avanzato, profili per applicazione.
- Modelli/engine alternativi dietro `SpeechEngine`; confrontare realisticamente prestazioni e licenze.

Non costruire un servizio SaaS, account utente, abbonamenti, registrazione cloud o AI generativa esterna.

## 8. Dashboard statistiche — completa ma leggera

Tutte le statistiche devono derivare da eventi locali realmente registrati; evitare dati fittizi in produzione. Nessun invio a servizi esterni.

### KPI di riepilogo
- Numero totale dettature completate e avviate, completate vs cancellate/fallite.
- **Parole generate** totali e per periodo; caratteri generati.
- Tempo totale di registrazione e tempo totale di elaborazione.
- Media/mediana e, ove possibile, p95 della latenza di elaborazione, **distinguendo** tempo di inferenza dal tempo di inserimento.
- Velocità di dettatura (parole/min di audio) e throughput inferenza (rapporto durata audio / durata elaborazione).
- Giorni attivi e streak opzionale, uso medio giornaliero/settimanale.
- Modello e lingua più utilizzati, successo/fallimento, principali errori aggregati.

### Grafici e analisi
- Timeline dettature e parole per **giorno/settimana/mese**, filtri data (oggi, 7 gg, 30 gg, 90 gg, personalizzato).
- **Heatmap giorno della settimana × ora** per numero di utilizzi e, opzionalmente, parole.
- Distribuzione utilizzi **per fascia oraria** e giorni più attivi.
- Durata delle sessioni (istogramma), elaborazione/latency trend, confronto tra modelli.
- Periodo corrente vs periodo precedente, con variazioni percentuali calcolate correttamente e gestione divisioni per zero.
- Elenco ultime sessioni con data/ora, durata, parole, lingua, modello, esito; contenuto testuale **solo se storage trascrizioni attivato**.
- Filtri per lingua e modello (eventualmente app target **solo opt-in**, considerando che può essere sensibile).
- Esportazione dati dashboard in CSV/JSON e reset granulare delle statistiche.

### Privacy e modello dati

Schema orientativo (raffinare con migrazioni e indici):

```text
sessions(
  id, started_at_utc, ended_at_utc, status,
  recorded_ms, inference_ms, insertion_ms,
  words_count, chars_count,
  language_code, model_id, error_category,
  input_mode, created_at_utc
)
transcripts(session_id, text_encrypted_or_plain_local)  # solo con opt-in; evitare di crearne righe altrimenti
models(id, name, version, local_path, size_bytes, checksum, installed_at_utc)
settings(key, value)
```

- Store timestamps in UTC e convertirli nel timezone locale per dashboard/heatmap, gestendo DST correttamente.
- Definire in modo univoco cosa significa "parola" (tokenizzazione coerente e documentata), cosa conta come "utilizzo" e come misurare intervalli.
- Usare **aggregazioni SQL** / query ottimizzate, non caricare tutte le sessioni in React per calcolare i grafici.
- Dashboard lazy-loaded, virtualizzazione/paginazione lista sessioni.
- Nessuna memorizzazione del file audio di default. Non archiviare trascrizioni senza consenso esplicito.
- La cifratura del testo salvato è una scelta di implementazione da documentare; se introdotta, custodire le chiavi con i meccanismi OS appropriati, non con una chiave hardcoded.

## 9. Performance: requisiti e benchmark

**Evitare numeri assoluti promessi senza benchmark su macchina reale.** Registrare hardware, modello, lunghezza audio, backend e condizioni test. Definire e misurare un baseline iniziale e poi budget/obiettivi verificabili.

Metriche da misurare:
- time to tray ready / time to overlay visible;
- latenza shortcut → overlay e shortcut → inizio registrazione;
- tempo primo utilizzo vs utilizzi successivi (model cold/warm);
- durata caricamento modello, RAM/RSS a idle, con modello caricato e durante inferenza;
- CPU media a idle, uso durante registrazione, overhead waveform;
- tempo audio-end → testo disponibile e tempo testo disponibile → inserimento;
- real-time factor = inference_duration / audio_duration (più basso = meglio), per più durate e modelli;
- p50, p95 e massimo su più prove, senza inventare valori.

**Obiettivi indicativi da verificare su Mac Apple Silicon scelto come baseline:** overlay percepito immediato (target p95 < 150 ms), applicazione idle senza lavoro continuo significativo, nessun blocco del thread UI, nessun reload modello a ogni invocazione. Adeguare gli obiettivi solo dopo aver ottenuto misurazioni riproducibili.

Indicazioni architetturali:
- Registrazione continua soltanto durante dettatura: **non** mantenere microfono aperto a riposo salvo opzione esplicita.
- Pipeline streaming audio con bounded queue/ring buffer e backpressure, non serializzare audio PCM continuamente via IPC JS.
- Rust workers dedicati per audio/inferenza/IO; cancellazione cooperativa e gestione dei timeout.
- Eventi UI per progress e volume con frequenza limitata (es. 15–30 Hz), non centinaia di re-render al secondo.
- Non mettere waveform o inferenza nel thread React.
- Nessuna query SQL bloccante nei percorsi critici; scrittura metriche dopo completamento/sessione, con garanzie di integrità.
- Evitare bundle pesanti e pacchetti installati ma non usati; profilare prima di ottimizzare.

## 10. Qualità software, test e CI: obbligatori

### Frontend
- TypeScript strict, lint, format, component tests con React Testing Library/Vitest o equivalenti.
- Test per overlay in tutti gli stati, tastiera e accessibilità, dashboard vuota, filtri e grafici, permessi non disponibili, cambio modello, errori.
- Hooks/stores con stato prevedibile e test, evitare side effects sparsi nei componenti.

### Rust
- `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`.
- Unit tests per state machine, VAD boundaries simulati, token/word counter, intervalli date/DST, aggregazioni, permessi astratti, injection target guard, retries/cancellation.
- Test di integrazione SQLite su database temporaneo con migrazioni, query, export, privacy e reset.
- Mock `SpeechEngine`, `AudioCapture`, `TextInjector` per test deterministici indipendenti da microfono e modelli.

### End-to-end macOS
- Smoke test onboarding, download modello, registrazione, overlay, shortcut, cancellazione, trascrizione, inserimento in campo di testo non sicuro e fallback clipboard.
- Test manuale esplicito dei permessi TCC, focus, Spaces, secondo monitor, fullscreen, crash/restart e riavvio Mac.
- Evitare automazioni invasive in campi sensibili. Documentare quali test richiedono permessi/hardware/GUI e non sono eseguibili in CI.

### CI
- GitHub Actions per frontend checks, Rust tests, build e controllo piattaforme quando disponibili; test reali macOS su runner Mac, matrice di controllo Windows/Linux.
- Bloccare merge o rilascio se falliscono lint, format, tests, build.
- Nessuna dipendenza di rete nei test unitari; fixtures audio locali sintetiche o liberamente utilizzabili, con licenze documentate.

### Benchmark e regressioni
- Script ripetibile per benchmark con set di campioni audio fissi, output in JSON/CSV.
- Conservare baseline e confronto tra commit per RAM, CPU, overlay startup e tempi di inferenza.
- Non dichiarare "performance ottimali" senza numeri e senza indicare ambiente e condizioni.

## 11. Sicurezza, licenze, privacy e UX

- Permessi Tauri minimi mediante capabilities; CSP restrittiva; niente comandi IPC generici per shell arbitraria o percorsi utente non validati.
- Validare percorso dei modelli, checksum, download atomico, protezione da path traversal.
- Non salvare audio o testo nei log di debug/errore. Log locali configurabili con redazione dati sensibili.
- Informare chiaramente quando l'app ascolta e quando si ferma; nessuna registrazione occulta.
- Privacy view: modello on-device, uso disco, pulsanti per disabilitare storico, eliminare trascrizioni, metriche e modelli, export locale.
- Verificare licenze di modelli e dipendenze (incluso whisper.cpp) prima della distribuzione; documentare attribuzioni.
- Accessibilità: supporto keyboard navigation, contrasto, riduzione animazioni, messaggi errore leggibili, notifiche non invasive.
- Modalità dark/light coerente con macOS, lingua UI italiana iniziale con predisposizione i18n inglese.

## 12. UX dettagliata

Schermate:
1. **Onboarding:** benvenuto, privacy offline, permessi, modello consigliato e download, shortcut, test microfono.
2. **Home:** stato applicazione, avvio dettatura manuale, ultimo utilizzo, link dashboard e setting; mostra se modello pronto.
3. **Dashboard:** KPI, grafici temporali, heatmap, confronto periodi, performance, ultimi eventi e filtri.
4. **Cronologia (opt-in):** elenco trascrizioni con ricerca e copia, esporta, elimina per riga / intervallo / tutto.
5. **Modelli:** installati/disponibili, dimensione, lingua, prestazioni indicative *misurate o dichiaratamente stimate*, download/progress/cancel/delete.
6. **Impostazioni:** microfono, shortcut, modalità toggle/hold, overlay, avvio automatico, lingua, privacy, storage, debug.
7. **Menu bar:** start/stop, stato microfono/modello, apri dashboard, impostazioni, quit.

Usare design system leggero, componenti riusabili (Button, Card, StatTile, ChartPanel, EmptyState, PermissionBanner, ModelRow, Modal, Toast). Niente monoliti JSX; separare query, calcoli, stato e rendering.

## 13. Strategia di implementazione per Codex

Procedere **incrementalmente**, mantenendo un progetto buildabile dopo ogni milestone:

**M0 — Setup verificato:** controllare macOS/architettura, installare *solo ciò che manca* (es. Xcode Command Line Tools, Rust toolchain, Node, Tauri prerequisites), documentare versioni; creare Tauri 2 + React + TS; ottenere startup funzionante su Mac.

**M1 — Foundation:** layout, navigazione, base SQLite, migrazioni, settings, architecture traits, logging sicuro, lint e test iniziali.

**M2 — Audio/AI offline:** microfono, VAD di base, integrazione whisper.cpp, installazione modello, trascrizione end-to-end a partire da file/campione di test e poi microfono; test e benchmark.

**M3 — Dettatura globale:** hotkey, status state machine, target app/focus, inserimento sicuro, fallback clipboard, onboarding permessi.

**M4 — Overlay:** mini finestra React sempre in primo piano, focus-safe, reale audio level, schermate errori, test multi-monitor e Spaces. Se Tauri window non raggiunge un comportamento Mac adeguato, implementare adapter NSPanel documentando il perché.

**M5 — Dashboard:** SQLite metrics, KPI, aggregazioni per ora/giorno/settimana/mese, heatmap, filtri, confronto periodi, grafici, export e reset; verificare la correttezza con dataset di test.

**M6 — Hardening:** privacy, crash recovery, performance budget, CI, installer Mac, documentazione setup e roadmap Windows/Linux.

Per ogni milestone Codex deve: implementare, eseguire controlli/test disponibili, correggere gli errori, aggiornare README e riportare brevemente cosa funziona, cosa non è ancora fatto e gli eventuali blocchi **reali**. Non sostituire funzionalità essenziali con schermate mock dichiarandole completate.

## 14. Definition of Done — prima release Mac

Il lavoro è considerato pronto per una release alfa quando:

- [ ] L'app parte su macOS Apple Silicon dal repository secondo README, senza passaggi nascosti.
- [ ] Può restare nella menu bar e non consuma risorse in modo anomalo a riposo.
- [ ] Installa/verifica almeno un modello locale, poi funziona completamente offline.
- [ ] Shortcut globale attiva e termina la registrazione, con toggle e/o hold implementati e testati.
- [ ] Overlay flottante mostra stato e livello del microfono e non sottrae focus.
- [ ] Il parlato in italiano viene trascritto con whisper.cpp e inserito nel campo dell'app attiva, con protezioni contro l'input nel target sbagliato.
- [ ] Se l'inserimento fallisce, il testo resta recuperabile e non va perso.
- [ ] Le metriche vengono registrate senza memorizzare audio o trascrizioni per default.
- [ ] Dashboard mostra valori reali, timeline, heatmap oraria/giornaliera, parole, sessioni, latenze, successo/errori, filtri ed export.
- [ ] Impostazioni e dati locali possono essere cancellati; nessun server richiesto.
- [ ] Unit, integration e smoke test disponibili passano; limitazioni dei test GUI manuali sono documentate.
- [ ] Benchmark ripetibili e misure iniziali incluse in `docs/PERFORMANCE.md`.
- [ ] Repository e core sono organizzati per introdurre adapter Windows/Linux senza riscrivere business logic o React.
- [ ] Build Mac distribuibile, documentazione delle eventuali limitazioni di firma/notarizzazione.

## 15. Istruzioni finali a Codex

- **Implementa il software, non soltanto il documento di design.** Se il repository è vuoto, inizializzalo; se esiste, ispezionalo e adattati a esso senza distruggere il codice esistente.
- Usa le versioni stabili attualmente disponibili e verifica la documentazione ufficiale durante l'implementazione.
- Se una scelta tecnica è incerta, implementa uno spike minimo, misura e motiva la decisione in un ADR in `docs/`.
- Mantieni componenti React piccoli e riusabili, backend Rust stratificato, IPC typed, interfacce testabili e platform-specific adapters.
- **Esegui davvero** build, lint, test e benchmark dove l'ambiente lo permette. Non affermare che passano se non sono stati eseguiti.
- Non richiedere all'utente decisioni di dettaglio non bloccanti: scegli default sensati e documentali.
- Dai precedenza assoluta al **percorso principale** (shortcut → registrazione → overlay → trascrizione locale → inserimento) rispetto alle feature secondarie, senza trascurare la dashboard richiesta.
- Quando una funzionalità non è ancora implementata, marcala esplicitamente come `TODO` e non mostrare dati finti come se fossero reali.
- Al termine consegna un resoconto di architettura, file principali, come eseguire l'app, come testarla, benchmark misurati e limiti residui.
