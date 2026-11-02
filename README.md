# Dicta

Dicta is a local-first desktop dictation app for macOS. The original desktop app uses Tauri 2, React/TypeScript, Rust, CPAL, whisper.cpp, and SQLite. It processes speech locally; it does not require an account, upload audio, or use telemetry. Transcript history is opt-in.

> **Native macOS port:** `macos/` contains an in-progress SwiftUI/AppKit implementation. Its current scope and limitations are documented in [`macos/README.md`](macos/README.md). The Tauri app remains in the repository as the established implementation; the native app uses a separate data store and does not migrate legacy records yet.

## Run the existing Tauri app

Development requires macOS 12+, Xcode Command Line Tools, CMake, Rust (pinned in `rust-toolchain.toml`), and Node.js 24.

```sh
xcode-select --install          # If Apple command-line tools are missing
brew install cmake               # If CMake is missing
. "$HOME/.cargo/env"             # If Rust was installed with rustup
npm ci
npm run tauri -- dev
```

Grant Microphone and Accessibility access in Dicta's setup. Choose and explicitly download a Whisper model: Tiny is about 78 MB, Base about 148 MB, and Small about 488 MB. Downloads are checked by size and SHA-256. Once a model is installed, dictation works offline. Italian is the default language; other languages and automatic detection are available.

Use **⌘⇧Space** while a writable text field in another app is focused to start and stop dictation. The shortcut and hold-to-talk behavior can be configured in Settings. Closing the main window keeps Dicta available in the menu bar. The manual button can test the microphone and model, but does not type into Dicta's own controls.

Text is inserted only if the original application and verified Accessibility target are still focused. Protected or non-writable fields require explicit recovery. Audio is kept in memory only. Uninserted text remains recoverable in memory and blocks a new dictation until recovered. Copying replaces the clipboard only after an explicit action. Recording is limited to five minutes; this is not streaming transcription.

## Checks

```sh
npm run lint
npm test
npm run typecheck
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri -- build --bundles app,dmg
```

If Tauri's Finder-based DMG layout fails, use `sh scripts/package-macos.sh`. The local bundle is not signed or notarized for release; distribution needs an Apple signing identity and TCC validation with a stable identity.

## Local data and privacy

The Tauri app stores data under `~/Library/Application Support/app.dicta.desktop/`, including `dicta.sqlite3` and downloaded models. Audio is not saved. Metrics are stored locally even when transcript history is disabled. When enabled, transcripts are stored as local plaintext; disabling history in the Tauri app deletes existing transcript text. Dashboard calculations run in SQLite and use local dates. CSV/JSON exports use a native save dialog.

The native Swift app stores its own records under `~/Library/Application Support/app.dicta.native/`; it currently does not import or modify the Tauri database.

## Documentation

- [Architecture](docs/ARCHITECTURE.md) and [technical decisions](docs/ADR-001.md)
- [Privacy](docs/PRIVACY.md), [platform support](docs/PLATFORM_SUPPORT.md), and [persistence](docs/PERSISTENCE.md)
- [Testing](docs/TESTING.md), [performance](docs/PERFORMANCE.md), and [current progress](docs/PROGRESS.md)
- [Original product specification](docs/SPECIFICATION.md)
- [Native macOS port](macos/README.md)

The product is named Dicta. Audio import, custom vocabulary, voice commands, and alternative engines remain out of scope for the initial implementation.
