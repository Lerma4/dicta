# Third-party components

Dependency versions are fixed by package-lock.json and src-tauri/Cargo.lock. Dicta does not claim ownership of these components. This inventory is a development notice, not an automated exhaustive license audit.

| Component | Upstream license |
|---|---|
| Tauri / official plugins | MIT or Apache-2.0 |
| React / React DOM / Vite | MIT |
| Lucide icons | ISC |
| whisper-rs | Unlicense |
| whisper.cpp / GGML / Whisper model distribution | MIT |
| CPAL / Rubato | Apache-2.0 (Rubato also MIT) |
| rusqlite | MIT |
| SQLite | Public domain |
| reqwest / Rustls | MIT or Apache-2.0; Rustls also ISC |
| SHA2 / serde / chrono / uuid | MIT or Apache-2.0 (chrono also BSD-3-Clause) |
| Hound benchmark WAV decoder | Apache-2.0 |

Primary sources: [Tauri](https://github.com/tauri-apps/tauri), [Whisper](https://github.com/openai/whisper), [whisper.cpp](https://github.com/ggml-org/whisper.cpp), [whisper-rs](https://github.com/tazz4843/whisper-rs), [model catalog](https://huggingface.co/ggerganov/whisper.cpp), Cargo package licenses and installed npm package notices. Before public distribution, generate a full transitive license inventory and include the actual upstream notice/license texts in the distributed artifact.

Benchmark audio assets are not bundled. JFK sample is sourced from the upstream whisper.cpp sample at v1.8.4; synthetic Italian test audio is generated locally using the installed macOS TTS voice, and Apple voice resources/audio are not redistributed.
