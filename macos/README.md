# Native macOS Dicta (Preview)

This SwiftUI/AppKit app is an ongoing native port and does not yet replace the Tauri app. Building requires macOS 13 or later, Xcode Command Line Tools, and CMake.

```sh
bash macos/Scripts/build-app.sh
open macos/.build/Dicta.app
```

Grant Microphone and Accessibility access in Settings. In Models, select an installed model or explicitly download one. Whisper remains available on macOS 13+; Parakeet Ultra and V3 require macOS 14+, and Redux requires macOS 15+. A new installation on macOS 14+ defaults to Ultra; an existing saved Whisper selection is preserved. Inference never downloads a model. With the Dicta window open, focus a text field in another app and press **⌘⇧Space** to start and stop dictation. If the original field cannot be written safely, recover the transcript in Dicta using the explicit Copy action.

For Rex, Dicta temporarily uses the clipboard and **⌘V** without pressing Return. Clipboard managers may see the transcript. Dicta restores the previous clipboard contents only if no other process has changed the clipboard in the meantime. Audio and inference remain local and in memory.

## Local data

The native app records basic statistics for completed dictations in `~/Library/Application Support/app.dicta.native/sessions.sqlite3`. Transcript history is optional, disabled by default, and stored as plaintext on this Mac. Turning history off stops saving new transcripts but does not delete existing ones. The History view can delete all saved transcripts while retaining statistics. Up to 200 recent saved transcripts are shown.

Whisper uses Accelerate and explicitly disables OpenMP. Parakeet is provided by a separately built FluidAudio 0.17.3 dynamic bridge (macOS 14+) so the host remains compatible with macOS 13. Model files are downloaded on explicit request into a per-model Application Support cache; deleting a model removes only that model's cache. FluidAudio and model license/attribution information is bundled under `Contents/Resources` and documented in `THIRD_PARTY_NOTICES.md`.

## Current limitations

- The complete microphone-to-transcription-to-insertion flow and permissions have not been validated end-to-end on different hardware and apps.
- The global shortcut is registered only while the Dicta window is open.
- Statistics and transcript history from the Tauri app are not imported; its database is left untouched.
- Charts, filters, exports, the dictation overlay, and the complete Settings feature set have not yet been ported.
- Whisper inference currently uses the CPU/Accelerate backend, not Metal. Parakeet inference uses FluidAudio/Core ML on compatible macOS versions.
- First-use Parakeet model compilation time, inference duration, and real-time factor have not been measured locally. See `docs/benchmarks/parakeet-local.md`; no model was downloaded for this change.
- The app is ad-hoc signed, not notarized, and is not ready for distribution.

Available checks:

```sh
sh macos/Scripts/test-core.sh
swift build -c release --package-path macos
```

`swift test --package-path macos` requires a toolchain that includes XCTest.
