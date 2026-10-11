# Dicta

Dettatura desktop locale per macOS Apple Silicon. React/TypeScript dentro Tauri 2, microfono CPAL, Whisper/whisper.cpp in Rust con Metal e fallback CPU, database SQLite. Niente account, upload, server o telemetria. La cronologia testuale è disattivata per impostazione iniziale.

## Avvio

Prerequisiti di sviluppo: macOS 12+ (verifica eseguita su macOS 26.6/M4), Xcode Command Line Tools, CMake, Rust 1.99 e Node 24. Il runtime distribuito non richiede Node, Rust o CMake sul Mac dell’utente.

```sh
xcode-select --install       # solo se gli strumenti Apple mancano
brew install cmake          # solo se CMake manca
# Rust: https://rustup.rs — rust-toolchain.toml fissa toolchain e componenti
. "$HOME/.cargo/env"        # dopo la prima installazione Rust, o riapri il terminale
npm ci
npm run tauri -- dev
```

Apri Dicta, concedi Microfono e Accessibilità dalle azioni di setup e scarica esplicitamente un modello. Tiny occupa circa 78 MB; Base circa 148 MB e Small circa 488 MB. L’installazione verifica dimensione e SHA-256. Dopo l’installazione non serve connessione per dettare. Italiano è il default; sono disponibili altre lingue e rilevamento automatico.

Usa **⌘ ⇧ Spazio** in un campo di testo di un’altra app, parla e premi nuovamente per terminare. La modalità “tieni premuto” e la scorciatoia si configurano nelle impostazioni. Chiudere la finestra mantiene Dicta nella barra dei menu; da lì puoi riaprirla, avviare/terminare/annullare o uscire. Il pulsante manuale nella finestra serve anche a provare microfono/modello: Dicta non inserisce testo nei propri controlli.

L’inserimento usa il campo Accessibilità originale soltanto se app e controllo sono ancora quelli identificati. Campi protetti o non scrivibili producono recupero esplicito. Il testo resta in RAM e puoi copiarlo da Dicta; una nuova dettatura viene rifiutata finché un risultato non inserito non è stato recuperato. Copia sostituisce gli appunti solo su tua richiesta. Il limite di registrazione è cinque minuti; non è trascrizione streaming.

## Verifica e bundle

```sh
npm run lint
npm test
npm run typecheck
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri -- build --bundles app,dmg
# Se la creazione DMG via Finder fallisce: sh scripts/package-macos.sh
```

Il bundle si trova in `src-tauri/target/release/bundle/macos/Dicta.app`; il DMG in `src-tauri/target/release/bundle/dmg/`. La build locale non è una release firmata/notarizzata: la distribuzione richiederà credenziali Apple e verifica TCC con identità stabile. Nessun tag, release GitHub o push è stato effettuato.

## Dati e documentazione

I dati sono nella directory applicativa restituita da Tauri, su macOS `~/Library/Application Support/app.dicta.desktop/`: `dicta.sqlite3` e `models/`. Non viene salvato audio. SQLite contiene metriche reali anche quando la cronologia è disattivata. La cronologia, se abilitata, contiene testo locale in chiaro; la disattivazione cancella subito il testo esistente. La dashboard aggrega nel backend SQL, rispetta le date locali e carica i grafici solo quando aperta. CSV/JSON si esportano con un dialogo di salvataggio nativo.

- [Architettura](docs/ARCHITECTURE.md), [decisioni tecniche](docs/ADR-001.md)
- [Privacy](docs/PRIVACY.md), [piattaforme e limiti](docs/PLATFORM_SUPPORT.md)
- [Test e checklist Mac](docs/TESTING.md), [benchmark](docs/PERFORMANCE.md)
- [Stato e passaggio di consegne](docs/PROGRESS.md)

La specifica originale è conservata in `docs/SPECIFICATION.md`; il nome definitivo è Dicta. P1/P2 (import audio, vocabolari, comandi vocali, altri motori) restano fuori dalla prima implementazione.
