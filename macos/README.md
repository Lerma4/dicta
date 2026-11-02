# Dicta for macOS

Dicta is a native app built with SwiftUI and AppKit. It requires macOS 13 or later, Xcode Command Line Tools, and CMake to build.

## Build and launch

From the repository root:

```sh
bash macos/Scripts/build-app.sh
open macos/.build/Dicta.app
```

The local build is ad-hoc signed and not notarized. To install it in Applications:

```sh
ditto macos/.build/Dicta.app /Applications/Dicta.app
open /Applications/Dicta.app
```

On first launch, grant the Microphone and Accessibility permissions requested by macOS. Ad-hoc signing does not guarantee that permissions will remain valid after future rebuilds or app replacements.

## Models and dictation

The **Models** screen lists Parakeet first. Ultra and V3 require macOS 14 or later; Redux requires macOS 15. On compatible systems, a new installation selects Parakeet Ultra by default. Models are downloaded only when explicitly requested.

**WhisperFlow** is the alternative for less capable hardware or older macOS versions; Whisper supports macOS 13 and later. Parakeet uses FluidAudio/Core ML on compatible systems. Whisper uses Accelerate and disables OpenMP.

With Dicta open, press **⌘⇧Space** while a text field in another app is focused to start or stop dictation. If Dicta cannot safely insert text into the original field, recover it using the explicit **Copy** action. Recordings are limited to five minutes. With Rex, text insertion temporarily uses the clipboard and **⌘V**, without pressing Return.

## Local data

Dictation statistics are stored in `~/Library/Application Support/app.dicta.native/sessions.sqlite3`. Transcript history is optional and disabled by default; when enabled, transcripts are stored as plaintext on this Mac. Disabling history stops new transcripts from being saved but does not delete existing ones. The History screen can delete saved transcripts while retaining statistics. Audio is not saved.

Installed models are stored in the local Application Support cache. Deleting a model removes only its cache. FluidAudio and other dependencies are documented in `THIRD_PARTY_NOTICES.md`.

## Limitations and tests

- The global shortcut is active only while the Dicta window is open.
- Dictation statistics and transcript history are not imported from other Dicta installations.
- Whisper inference uses CPU/Accelerate, not Metal.
- Parakeet's initial compilation time and performance have not been measured locally; see `docs/benchmarks/parakeet-local.md`.
- The build is ad-hoc signed and is not ready for public distribution.

```sh
sh macos/Scripts/test-core.sh
swift build -c release --package-path macos
```

`swift test --package-path macos` requires a toolchain that includes XCTest.
