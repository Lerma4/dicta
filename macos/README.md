# Native macOS Dicta (Preview)

This SwiftUI/AppKit app is an ongoing native port and does not yet replace the Tauri app. Building requires macOS 13 or later, Xcode Command Line Tools, and CMake.

```sh
bash macos/Scripts/build-app.sh
open macos/.build/Dicta.app
```

Grant Microphone and Accessibility access in Settings. In Models, select an installed model or explicitly download one. With the Dicta window open, focus a text field in another app and press **⌘⇧Space** to start and stop dictation. If the original field cannot be written safely, recover the transcript in Dicta using the explicit Copy action.

For Rex, Dicta temporarily uses the clipboard and **⌘V** without pressing Return. Clipboard managers may see the transcript. Dicta restores the previous clipboard contents only if no other process has changed the clipboard in the meantime. Audio and inference remain local and in memory.

## Local data

The native app records basic statistics for completed dictations in `~/Library/Application Support/app.dicta.native/sessions.sqlite3`. Transcript history is optional, disabled by default, and stored as plaintext on this Mac. Turning history off stops saving new transcripts but does not delete existing ones. The History view can delete all saved transcripts while retaining statistics. Up to 200 recent saved transcripts are shown.

The native app uses Accelerate and explicitly disables OpenMP. The available Homebrew OpenMP library on this machine targets macOS 26, while Dicta's native build targets macOS 13 and later. Performance impact has not been benchmarked.

## Current limitations

- The complete microphone-to-transcription-to-insertion flow and permissions have not been validated end-to-end on different hardware and apps.
- The global shortcut is registered only while the Dicta window is open.
- Statistics and transcript history from the Tauri app are not imported; its database is left untouched.
- Charts, filters, exports, the dictation overlay, and the complete Settings feature set have not yet been ported.
- Whisper inference currently uses the CPU/Accelerate backend, not Metal.
- The app is ad-hoc signed, not notarized, and is not ready for distribution.

Available checks:

```sh
sh macos/Scripts/test-core.sh
swift build -c release --package-path macos
```

`swift test --package-path macos` requires a toolchain that includes XCTest.
